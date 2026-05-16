use std::{collections::BTreeMap, fs, path::Path};

use anyhow::Result;

use crate::app::summary::{ExperimentSummaryRecord, read_summary_records};

pub fn write_report(
    users_input: &Path,
    cw_input: &Path,
    mixed_input: &Path,
    output: &Path,
    plots_dir: Option<&Path>,
) -> Result<()> {
    let users = read_summary_records(users_input)?;
    let cw = read_summary_records(cw_input)?;
    let mixed = read_summary_records(mixed_input)?;

    let mut markdown = String::new();
    markdown.push_str("# csma/ca experiment report\n\n");
    markdown.push_str("## inputs\n\n");
    markdown.push_str(&format!("- users summary: {}\n", users_input.display()));
    markdown.push_str(&format!("- cw summary: {}\n", cw_input.display()));
    markdown.push_str(&format!("- mixed summary: {}\n", mixed_input.display()));
    if let Some(plots_dir) = plots_dir {
        markdown.push_str(&format!("- plots directory: {}\n", plots_dir.display()));
    }
    markdown.push('\n');

    append_numeric_section(&mut markdown, "users sweep", "users", users, |record| {
        record.total_users.to_string()
    });
    append_numeric_section(&mut markdown, "cw sweep", "cw_min", cw, |record| {
        record
            .cw_min
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("n/a"))
    });
    append_mixed_section(&mut markdown, mixed);

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, markdown)?;

    Ok(())
}

fn append_numeric_section(
    markdown: &mut String,
    title: &str,
    x_label: &str,
    mut records: Vec<ExperimentSummaryRecord>,
    x_value: impl Fn(&ExperimentSummaryRecord) -> String,
) {
    records.sort_by_key(|record| {
        (
            record.total_users,
            record.cw_min.unwrap_or_default(),
            record.class_name.clone(),
        )
    });

    markdown.push_str(&format!("## {title}\n\n"));
    if let Some(first) = records.first() {
        markdown.push_str(&format!(
            "timing preset: `{:?}` ({})\n\n",
            first.timing_preset,
            first.timing_preset.description()
        ));
    }
    markdown.push_str(&format!(
        "| {x_label} | delay mean | delay 95% ci | throughput mean | throughput 95% ci | fairness | variance | zero-success frac | max-station share |\n"
    ));
    markdown.push_str("| --- | ---: | --- | ---: | --- | ---: | ---: | ---: | ---: |\n");

    for record in records {
        markdown.push_str(&format!(
            "| {} | {:.4} | [{:.4}, {:.4}] | {:.4} | [{:.4}, {:.4}] | {:.4} | {:.6} | {:.4} | {:.4} |\n",
            x_value(&record),
            record.mean_average_delay_slots,
            record.ci95_low_average_delay_slots,
            record.ci95_high_average_delay_slots,
            record.mean_throughput_bits_per_slot,
            record.ci95_low_throughput_bits_per_slot,
            record.ci95_high_throughput_bits_per_slot,
            record.mean_jain_fairness_index,
            record.mean_per_user_throughput_variance,
            record.mean_zero_success_station_fraction,
            record.mean_max_station_throughput_share,
        ));
    }

    markdown.push('\n');
}

fn append_mixed_section(markdown: &mut String, records: Vec<ExperimentSummaryRecord>) {
    markdown.push_str("## mixed classes\n\n");
    if let Some(first) = records.first() {
        markdown.push_str(&format!(
            "timing preset: `{:?}` ({})\n\n",
            first.timing_preset,
            first.timing_preset.description()
        ));
    }
    markdown.push_str(
        "| class | users | delay mean | delay 95% ci | throughput mean | throughput 95% ci | per-user throughput |\n",
    );
    markdown.push_str("| --- | ---: | ---: | --- | ---: | --- | ---: |\n");

    for record in &records {
        markdown.push_str(&format!(
            "| {} | {} | {:.4} | [{:.4}, {:.4}] | {:.4} | [{:.4}, {:.4}] | {:.4} |\n",
            record.class_name,
            record.class_users,
            record.mean_average_delay_slots,
            record.ci95_low_average_delay_slots,
            record.ci95_high_average_delay_slots,
            record.mean_throughput_bits_per_slot,
            record.ci95_low_throughput_bits_per_slot,
            record.ci95_high_throughput_bits_per_slot,
            record.mean_per_user_throughput_bits_per_slot,
        ));
    }

    markdown.push('\n');
    markdown.push_str("### fairness summary\n\n");
    markdown.push_str(
        "| scenario | jain fairness | fairness 95% ci | throughput variance | variance 95% ci | zero-success frac | zero-success 95% ci | max-station share | max-share 95% ci |\n",
    );
    markdown.push_str("| --- | ---: | --- | ---: | --- | ---: | --- | ---: | --- |\n");

    for record in unique_fairness_rows(&records) {
        markdown.push_str(&format!(
            "| {} | {:.4} | [{:.4}, {:.4}] | {:.6} | [{:.6}, {:.6}] | {:.4} | [{:.4}, {:.4}] | {:.4} | [{:.4}, {:.4}] |\n",
            record.scenario,
            record.mean_jain_fairness_index,
            record.ci95_low_jain_fairness_index,
            record.ci95_high_jain_fairness_index,
            record.mean_per_user_throughput_variance,
            record.ci95_low_per_user_throughput_variance,
            record.ci95_high_per_user_throughput_variance,
            record.mean_zero_success_station_fraction,
            record.ci95_low_zero_success_station_fraction,
            record.ci95_high_zero_success_station_fraction,
            record.mean_max_station_throughput_share,
            record.ci95_low_max_station_throughput_share,
            record.ci95_high_max_station_throughput_share,
        ));
    }

    markdown.push('\n');
}

fn unique_fairness_rows(records: &[ExperimentSummaryRecord]) -> Vec<ExperimentSummaryRecord> {
    let mut grouped = BTreeMap::new();

    for record in records {
        grouped
            .entry((
                record.scenario.clone(),
                record.total_users,
                record.lower_cw_min,
                record.higher_cw_min,
                record.timing_preset,
            ))
            .or_insert_with(|| record.clone());
    }

    grouped.into_values().collect()
}
