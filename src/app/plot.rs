use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use plotters::{coord::Shift, prelude::*};

use crate::app::summary::{ExperimentSummaryRecord, read_summary_records};

const SWEEP_CHART_SIZE: (u32, u32) = (1800, 1000);
const MIXED_CHART_SIZE: (u32, u32) = (1800, 1000);

#[derive(Debug, Clone, PartialEq)]
struct NumericSummaryPoint {
    x: f64,
    mean: f64,
    ci_low: f64,
    ci_high: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct LabeledSummaryPoint {
    label: String,
    mean: f64,
    ci_low: f64,
    ci_high: f64,
}

struct SweepMetricPanels<'a> {
    delay: &'a [NumericSummaryPoint],
    throughput: &'a [NumericSummaryPoint],
    fairness: &'a [NumericSummaryPoint],
    variance: &'a [NumericSummaryPoint],
    zero_success: &'a [NumericSummaryPoint],
    max_share: &'a [NumericSummaryPoint],
}

pub fn write_plots(
    users_input: &Path,
    cw_input: &Path,
    mixed_input: &Path,
    output_dir: &Path,
) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let users_records = read_summary_records(users_input)?;
    let cw_records = read_summary_records(cw_input)?;
    let mixed_records = read_summary_records(mixed_input)?;

    let users_delay = numeric_points(
        &users_records,
        |record| record.total_users as f64,
        |record| {
            (
                record.mean_average_delay_slots,
                record.ci95_low_average_delay_slots,
                record.ci95_high_average_delay_slots,
            )
        },
    );
    let users_throughput = numeric_points(
        &users_records,
        |record| record.total_users as f64,
        |record| {
            (
                record.mean_throughput_bits_per_slot,
                record.ci95_low_throughput_bits_per_slot,
                record.ci95_high_throughput_bits_per_slot,
            )
        },
    );
    let users_fairness = numeric_points(
        &users_records,
        |record| record.total_users as f64,
        |record| {
            (
                record.mean_jain_fairness_index,
                record.ci95_low_jain_fairness_index,
                record.ci95_high_jain_fairness_index,
            )
        },
    );
    let users_variance = numeric_points(
        &users_records,
        |record| record.total_users as f64,
        |record| {
            (
                record.mean_per_user_throughput_variance,
                record.ci95_low_per_user_throughput_variance,
                record.ci95_high_per_user_throughput_variance,
            )
        },
    );
    let users_zero_success = numeric_points(
        &users_records,
        |record| record.total_users as f64,
        |record| {
            (
                record.mean_zero_success_station_fraction,
                record.ci95_low_zero_success_station_fraction,
                record.ci95_high_zero_success_station_fraction,
            )
        },
    );
    let users_max_share = numeric_points(
        &users_records,
        |record| record.total_users as f64,
        |record| {
            (
                record.mean_max_station_throughput_share,
                record.ci95_low_max_station_throughput_share,
                record.ci95_high_max_station_throughput_share,
            )
        },
    );
    let cw_delay = numeric_points(
        &cw_records,
        |record| record.cw_min.unwrap_or_default() as f64,
        |record| {
            (
                record.mean_average_delay_slots,
                record.ci95_low_average_delay_slots,
                record.ci95_high_average_delay_slots,
            )
        },
    );
    let cw_fairness = numeric_points(
        &cw_records,
        |record| record.cw_min.unwrap_or_default() as f64,
        |record| {
            (
                record.mean_jain_fairness_index,
                record.ci95_low_jain_fairness_index,
                record.ci95_high_jain_fairness_index,
            )
        },
    );
    let cw_variance = numeric_points(
        &cw_records,
        |record| record.cw_min.unwrap_or_default() as f64,
        |record| {
            (
                record.mean_per_user_throughput_variance,
                record.ci95_low_per_user_throughput_variance,
                record.ci95_high_per_user_throughput_variance,
            )
        },
    );
    let cw_zero_success = numeric_points(
        &cw_records,
        |record| record.cw_min.unwrap_or_default() as f64,
        |record| {
            (
                record.mean_zero_success_station_fraction,
                record.ci95_low_zero_success_station_fraction,
                record.ci95_high_zero_success_station_fraction,
            )
        },
    );
    let cw_max_share = numeric_points(
        &cw_records,
        |record| record.cw_min.unwrap_or_default() as f64,
        |record| {
            (
                record.mean_max_station_throughput_share,
                record.ci95_low_max_station_throughput_share,
                record.ci95_high_max_station_throughput_share,
            )
        },
    );
    let cw_throughput = numeric_points(
        &cw_records,
        |record| record.cw_min.unwrap_or_default() as f64,
        |record| {
            (
                record.mean_throughput_bits_per_slot,
                record.ci95_low_throughput_bits_per_slot,
                record.ci95_high_throughput_bits_per_slot,
            )
        },
    );
    let mixed_delay = labeled_points(
        &mixed_records,
        |record| record.class_name.clone(),
        |record| {
            (
                record.mean_average_delay_slots,
                record.ci95_low_average_delay_slots,
                record.ci95_high_average_delay_slots,
            )
        },
    );
    let mixed_throughput = labeled_points(
        &mixed_records,
        |record| record.class_name.clone(),
        |record| {
            (
                record.mean_throughput_bits_per_slot,
                record.ci95_low_throughput_bits_per_slot,
                record.ci95_high_throughput_bits_per_slot,
            )
        },
    );
    let mixed_fairness = fairness_points(&mixed_records, |record| {
        format!(
            "cw{}-{}",
            record.lower_cw_min.unwrap_or_default(),
            record.higher_cw_min.unwrap_or_default()
        )
    });
    let mixed_variance = labeled_points(
        &mixed_records,
        |record| {
            format!(
                "cw{}-{}",
                record.lower_cw_min.unwrap_or_default(),
                record.higher_cw_min.unwrap_or_default()
            )
        },
        |record| {
            (
                record.mean_per_user_throughput_variance,
                record.ci95_low_per_user_throughput_variance,
                record.ci95_high_per_user_throughput_variance,
            )
        },
    );
    let mixed_zero_success = labeled_points(
        &mixed_records,
        |record| {
            format!(
                "cw{}-{}",
                record.lower_cw_min.unwrap_or_default(),
                record.higher_cw_min.unwrap_or_default()
            )
        },
        |record| {
            (
                record.mean_zero_success_station_fraction,
                record.ci95_low_zero_success_station_fraction,
                record.ci95_high_zero_success_station_fraction,
            )
        },
    );
    let mixed_max_share = labeled_points(
        &mixed_records,
        |record| {
            format!(
                "cw{}-{}",
                record.lower_cw_min.unwrap_or_default(),
                record.higher_cw_min.unwrap_or_default()
            )
        },
        |record| {
            (
                record.mean_max_station_throughput_share,
                record.ci95_low_max_station_throughput_share,
                record.ci95_high_max_station_throughput_share,
            )
        },
    );

    let users_output = output_dir.join("users.png");
    let cw_output = output_dir.join("cw.png");
    let mixed_output = output_dir.join("mixed.png");

    draw_sweep_chart(
        &users_output,
        "users sweep",
        "users",
        SweepMetricPanels {
            delay: &users_delay,
            throughput: &users_throughput,
            fairness: &users_fairness,
            variance: &users_variance,
            zero_success: &users_zero_success,
            max_share: &users_max_share,
        },
    )?;
    draw_sweep_chart(
        &cw_output,
        "cwmin sweep",
        "cwmin",
        SweepMetricPanels {
            delay: &cw_delay,
            throughput: &cw_throughput,
            fairness: &cw_fairness,
            variance: &cw_variance,
            zero_success: &cw_zero_success,
            max_share: &cw_max_share,
        },
    )?;
    draw_mixed_chart(
        &mixed_output,
        &mixed_delay,
        &mixed_throughput,
        &mixed_fairness,
        &mixed_variance,
        &mixed_zero_success,
        &mixed_max_share,
    )?;

    Ok(vec![users_output, cw_output, mixed_output])
}

fn numeric_points(
    records: &[ExperimentSummaryRecord],
    x_value: impl Fn(&ExperimentSummaryRecord) -> f64,
    metric: impl Fn(&ExperimentSummaryRecord) -> (f64, f64, f64),
) -> Vec<NumericSummaryPoint> {
    let mut points: Vec<_> = records
        .iter()
        .map(|record| {
            let (mean, ci_low, ci_high) = metric(record);
            NumericSummaryPoint {
                x: x_value(record),
                mean,
                ci_low,
                ci_high,
            }
        })
        .collect();

    points.sort_by(|left, right| left.x.total_cmp(&right.x));
    points
}

fn labeled_points(
    records: &[ExperimentSummaryRecord],
    label: impl Fn(&ExperimentSummaryRecord) -> String,
    metric: impl Fn(&ExperimentSummaryRecord) -> (f64, f64, f64),
) -> Vec<LabeledSummaryPoint> {
    let mut grouped = BTreeMap::new();

    for record in records {
        let (mean, ci_low, ci_high) = metric(record);
        grouped.entry(label(record)).or_insert(LabeledSummaryPoint {
            label: label(record),
            mean,
            ci_low,
            ci_high,
        });
    }

    grouped.into_values().collect()
}

fn fairness_points(
    records: &[ExperimentSummaryRecord],
    label: impl Fn(&ExperimentSummaryRecord) -> String,
) -> Vec<LabeledSummaryPoint> {
    let mut grouped = BTreeMap::new();

    for record in records {
        grouped.entry(label(record)).or_insert(LabeledSummaryPoint {
            label: label(record),
            mean: record.mean_jain_fairness_index,
            ci_low: record.ci95_low_jain_fairness_index,
            ci_high: record.ci95_high_jain_fairness_index,
        });
    }

    grouped.into_values().collect()
}

fn draw_sweep_chart(
    output: &Path,
    title: &str,
    x_label: &str,
    panels: SweepMetricPanels<'_>,
) -> Result<()> {
    ensure!(!panels.delay.is_empty(), "delay series must not be empty");
    ensure!(
        !panels.throughput.is_empty(),
        "throughput series must not be empty"
    );

    let root = BitMapBackend::new(output, SWEEP_CHART_SIZE).into_drawing_area();
    root.fill(&WHITE)?;
    let areas = root.split_evenly((2, 3));

    draw_line_chart(
        &areas[0],
        &format!("{title}: delay"),
        x_label,
        "average delay (slots)",
        panels.delay,
        RED,
    )?;
    draw_line_chart(
        &areas[1],
        &format!("{title}: throughput"),
        x_label,
        "throughput (bits/slot)",
        panels.throughput,
        BLUE,
    )?;
    draw_line_chart(
        &areas[2],
        &format!("{title}: fairness"),
        x_label,
        "jain fairness index",
        panels.fairness,
        GREEN,
    )?;
    draw_line_chart(
        &areas[3],
        &format!("{title}: variance"),
        x_label,
        "per-station throughput variance",
        panels.variance,
        MAGENTA,
    )?;
    draw_line_chart(
        &areas[4],
        &format!("{title}: zero-success fraction"),
        x_label,
        "zero-success station fraction",
        panels.zero_success,
        CYAN,
    )?;
    draw_line_chart(
        &areas[5],
        &format!("{title}: max station share"),
        x_label,
        "max station throughput share",
        panels.max_share,
        BLACK,
    )?;

    root.present()?;
    Ok(())
}

fn draw_mixed_chart(
    output: &Path,
    delay_points: &[LabeledSummaryPoint],
    throughput_points: &[LabeledSummaryPoint],
    fairness_points: &[LabeledSummaryPoint],
    variance_points: &[LabeledSummaryPoint],
    zero_success_points: &[LabeledSummaryPoint],
    max_share_points: &[LabeledSummaryPoint],
) -> Result<()> {
    ensure!(!delay_points.is_empty(), "delay bars must not be empty");
    ensure!(
        !throughput_points.is_empty(),
        "throughput bars must not be empty"
    );

    let root = BitMapBackend::new(output, MIXED_CHART_SIZE).into_drawing_area();
    root.fill(&WHITE)?;
    let areas = root.split_evenly((2, 3));

    draw_bar_chart(
        &areas[0],
        "mixed classes: delay",
        "average delay (slots)",
        delay_points,
        RED,
    )?;
    draw_bar_chart(
        &areas[1],
        "mixed classes: throughput",
        "throughput (bits/slot)",
        throughput_points,
        BLUE,
    )?;
    draw_bar_chart(
        &areas[2],
        "mixed classes: jain fairness",
        "jain fairness index",
        fairness_points,
        GREEN,
    )?;
    draw_bar_chart(
        &areas[3],
        "mixed classes: throughput variance",
        "per-station throughput variance",
        variance_points,
        MAGENTA,
    )?;
    draw_bar_chart(
        &areas[4],
        "mixed classes: zero-success fraction",
        "zero-success station fraction",
        zero_success_points,
        CYAN,
    )?;
    draw_bar_chart(
        &areas[5],
        "mixed classes: max station share",
        "max station throughput share",
        max_share_points,
        BLACK,
    )?;

    root.present()?;
    Ok(())
}

fn draw_line_chart(
    area: &DrawingArea<BitMapBackend<'_>, Shift>,
    title: &str,
    x_label: &str,
    y_label: &str,
    points: &[NumericSummaryPoint],
    color: RGBColor,
) -> Result<()> {
    let x_min = points.first().map(|point| point.x).unwrap_or(0.0);
    let x_max = points.last().map(|point| point.x).unwrap_or(1.0);
    let y_max = points
        .iter()
        .map(|point| point.ci_high.max(point.mean))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let x_end = if (x_max - x_min).abs() < f64::EPSILON {
        x_max + 1.0
    } else {
        x_max
    };

    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 28).into_font())
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(x_min..x_end, 0.0_f64..(y_max * 1.1))?;

    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .light_line_style(TRANSPARENT)
        .draw()?;

    chart.draw_series(LineSeries::new(
        points.iter().map(|point| (point.x, point.mean)),
        color.stroke_width(3),
    ))?;
    chart.draw_series(
        points
            .iter()
            .map(|point| Circle::new((point.x, point.mean), 5, color.filled())),
    )?;
    chart.draw_series(
        points
            .iter()
            .flat_map(|point| confidence_lines(point.x, point.ci_low, point.ci_high, color)),
    )?;

    Ok(())
}

fn draw_bar_chart(
    area: &DrawingArea<BitMapBackend<'_>, Shift>,
    title: &str,
    y_label: &str,
    points: &[LabeledSummaryPoint],
    color: RGBColor,
) -> Result<()> {
    let y_max = points
        .iter()
        .map(|point| point.ci_high.max(point.mean))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 28).into_font())
        .margin(20)
        .x_label_area_size(60)
        .y_label_area_size(70)
        .build_cartesian_2d(0.0_f64..points.len() as f64, 0.0_f64..(y_max * 1.15))?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_labels(points.len())
        .x_label_formatter(&|value| {
            let index = value.floor() as usize;
            points
                .get(index)
                .map(|point| point.label.clone())
                .unwrap_or_default()
        })
        .y_desc(y_label)
        .draw()?;

    chart.draw_series(points.iter().enumerate().map(|(index, point)| {
        let left = index as f64 + 0.15;
        let right = index as f64 + 0.85;
        Rectangle::new([(left, 0.0), (right, point.mean)], color.mix(0.7).filled())
    }))?;

    chart.draw_series(points.iter().enumerate().flat_map(|(index, point)| {
        let center = index as f64 + 0.5;
        confidence_lines(center, point.ci_low, point.ci_high, color)
    }))?;

    Ok(())
}

fn confidence_lines(
    x: f64,
    ci_low: f64,
    ci_high: f64,
    color: RGBColor,
) -> [PathElement<(f64, f64)>; 3] {
    let style = color.mix(0.6).stroke_width(2);
    let cap_half_width = 0.1;

    [
        PathElement::new(vec![(x, ci_low), (x, ci_high)], style),
        PathElement::new(
            vec![(x - cap_half_width, ci_low), (x + cap_half_width, ci_low)],
            style,
        ),
        PathElement::new(
            vec![(x - cap_half_width, ci_high), (x + cap_half_width, ci_high)],
            style,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crate::{TimingPreset, app::summary::ExperimentSummaryRecord};

    use super::{fairness_points, labeled_points, numeric_points};

    fn record(
        total_users: u32,
        cw_min: Option<u32>,
        class_name: &str,
        delay: f64,
        throughput: f64,
    ) -> ExperimentSummaryRecord {
        ExperimentSummaryRecord {
            scenario: String::from("test"),
            timing_preset: TimingPreset::Baseline,
            total_users,
            cw_min,
            lower_cw_min: Some(4),
            higher_cw_min: Some(16),
            class_name: class_name.to_string(),
            class_users: total_users,
            trials: 3,
            mean_successful_packets: 10.0,
            mean_collision_attempts: 2.0,
            mean_average_delay_slots: delay,
            stddev_average_delay_slots: 1.0,
            ci95_low_average_delay_slots: delay - 0.5,
            ci95_high_average_delay_slots: delay + 0.5,
            mean_throughput_bits_per_slot: throughput,
            stddev_throughput_bits_per_slot: 2.0,
            ci95_low_throughput_bits_per_slot: throughput - 1.0,
            ci95_high_throughput_bits_per_slot: throughput + 1.0,
            mean_per_user_throughput_bits_per_slot: throughput / total_users.max(1) as f64,
            mean_jain_fairness_index: 0.8,
            ci95_low_jain_fairness_index: 0.75,
            ci95_high_jain_fairness_index: 0.85,
            mean_per_user_throughput_variance: 0.02,
            ci95_low_per_user_throughput_variance: 0.01,
            ci95_high_per_user_throughput_variance: 0.03,
            mean_zero_success_station_fraction: 0.15,
            ci95_low_zero_success_station_fraction: 0.1,
            ci95_high_zero_success_station_fraction: 0.2,
            mean_max_station_throughput_share: 0.35,
            ci95_low_max_station_throughput_share: 0.3,
            ci95_high_max_station_throughput_share: 0.4,
        }
    }

    #[test]
    fn numeric_points_sort_by_x_value() {
        let records = vec![
            record(20, Some(16), "standard", 22.0, 300.0),
            record(10, Some(8), "standard", 12.0, 150.0),
        ];

        let points = numeric_points(
            &records,
            |record| record.total_users as f64,
            |record| {
                (
                    record.mean_average_delay_slots,
                    record.ci95_low_average_delay_slots,
                    record.ci95_high_average_delay_slots,
                )
            },
        );

        assert_eq!(points[0].x, 10.0);
        assert_eq!(points[0].mean, 12.0);
        assert_eq!(points[1].x, 20.0);
    }

    #[test]
    fn labeled_points_keep_single_row_per_label() {
        let records = vec![
            record(20, None, "lower-cw", 50.0, 1000.0),
            record(20, None, "higher-cw", 150.0, 300.0),
            record(20, None, "lower-cw", 70.0, 1100.0),
        ];

        let points = labeled_points(
            &records,
            |record| record.class_name.clone(),
            |record| {
                (
                    record.mean_average_delay_slots,
                    record.ci95_low_average_delay_slots,
                    record.ci95_high_average_delay_slots,
                )
            },
        );

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].label, "higher-cw");
        assert_eq!(points[1].label, "lower-cw");
    }

    #[test]
    fn fairness_points_deduplicate_mixed_summary_rows() {
        let records = vec![
            record(20, None, "lower-cw", 50.0, 1000.0),
            record(20, None, "higher-cw", 150.0, 300.0),
        ];

        let points = fairness_points(&records, |record| {
            format!(
                "cw{}-{}",
                record.lower_cw_min.unwrap_or_default(),
                record.higher_cw_min.unwrap_or_default()
            )
        });

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].mean, 0.8);
    }
}
