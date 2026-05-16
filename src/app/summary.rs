use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use csv::Reader;
use serde::{Deserialize, Serialize};

use crate::{app::output::ExperimentRecord, domain::config::TimingPreset};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperimentSummaryRecord {
    pub scenario: String,
    pub timing_preset: TimingPreset,
    pub total_users: u32,
    pub cw_min: Option<u32>,
    pub lower_cw_min: Option<u32>,
    pub higher_cw_min: Option<u32>,
    pub class_name: String,
    pub class_users: u32,
    pub trials: u32,
    pub mean_successful_packets: f64,
    pub mean_collision_attempts: f64,
    pub mean_average_delay_slots: f64,
    pub stddev_average_delay_slots: f64,
    pub ci95_low_average_delay_slots: f64,
    pub ci95_high_average_delay_slots: f64,
    pub mean_throughput_bits_per_slot: f64,
    pub stddev_throughput_bits_per_slot: f64,
    pub ci95_low_throughput_bits_per_slot: f64,
    pub ci95_high_throughput_bits_per_slot: f64,
    pub mean_per_user_throughput_bits_per_slot: f64,
    pub mean_jain_fairness_index: f64,
    pub ci95_low_jain_fairness_index: f64,
    pub ci95_high_jain_fairness_index: f64,
    pub mean_per_user_throughput_variance: f64,
    pub ci95_low_per_user_throughput_variance: f64,
    pub ci95_high_per_user_throughput_variance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Stats {
    mean: f64,
    stddev: f64,
    ci_low: f64,
    ci_high: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct FairnessMetric {
    jain_fairness_index: f64,
    per_user_throughput_variance: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SummaryKey {
    scenario: String,
    timing_preset: TimingPreset,
    total_users: u32,
    cw_min: Option<u32>,
    lower_cw_min: Option<u32>,
    higher_cw_min: Option<u32>,
    class_name: String,
    class_users: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParentKey {
    scenario: String,
    timing_preset: TimingPreset,
    total_users: u32,
    cw_min: Option<u32>,
    lower_cw_min: Option<u32>,
    higher_cw_min: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TrialKey {
    parent: ParentKey,
    trial: u32,
}

pub fn summarize_records(records: &[ExperimentRecord]) -> Vec<ExperimentSummaryRecord> {
    let mut grouped: BTreeMap<SummaryKey, Vec<&ExperimentRecord>> = BTreeMap::new();

    for record in records {
        grouped.entry(summary_key(record)).or_default().push(record);
    }

    let fairness_stats = fairness_stats_by_parent(records);

    grouped
        .into_iter()
        .map(|(key, records)| {
            let successful_packets = summarize_values(
                records
                    .iter()
                    .map(|record| record.successful_packets as f64),
            );
            let collision_attempts = summarize_values(
                records
                    .iter()
                    .map(|record| record.collision_attempts as f64),
            );
            let average_delay_slots =
                summarize_values(records.iter().map(|record| record.average_delay_slots));
            let throughput_bits_per_slot =
                summarize_values(records.iter().map(|record| record.throughput_bits_per_slot));
            let per_user_throughput_bits_per_slot =
                summarize_values(records.iter().map(|record| {
                    record.throughput_bits_per_slot / record.class_users.max(1) as f64
                }));
            let fairness = fairness_stats
                .get(&parent_key_from_summary(&key))
                .copied()
                .unwrap_or_else(default_fairness_stats);

            ExperimentSummaryRecord {
                scenario: key.scenario,
                timing_preset: key.timing_preset,
                total_users: key.total_users,
                cw_min: key.cw_min,
                lower_cw_min: key.lower_cw_min,
                higher_cw_min: key.higher_cw_min,
                class_name: key.class_name,
                class_users: key.class_users,
                trials: records.len() as u32,
                mean_successful_packets: successful_packets.mean,
                mean_collision_attempts: collision_attempts.mean,
                mean_average_delay_slots: average_delay_slots.mean,
                stddev_average_delay_slots: average_delay_slots.stddev,
                ci95_low_average_delay_slots: average_delay_slots.ci_low,
                ci95_high_average_delay_slots: average_delay_slots.ci_high,
                mean_throughput_bits_per_slot: throughput_bits_per_slot.mean,
                stddev_throughput_bits_per_slot: throughput_bits_per_slot.stddev,
                ci95_low_throughput_bits_per_slot: throughput_bits_per_slot.ci_low,
                ci95_high_throughput_bits_per_slot: throughput_bits_per_slot.ci_high,
                mean_per_user_throughput_bits_per_slot: per_user_throughput_bits_per_slot.mean,
                mean_jain_fairness_index: fairness.0.mean,
                ci95_low_jain_fairness_index: fairness.0.ci_low,
                ci95_high_jain_fairness_index: fairness.0.ci_high,
                mean_per_user_throughput_variance: fairness.1.mean,
                ci95_low_per_user_throughput_variance: fairness.1.ci_low,
                ci95_high_per_user_throughput_variance: fairness.1.ci_high,
            }
        })
        .collect()
}

pub fn read_summary_records(path: &Path) -> Result<Vec<ExperimentSummaryRecord>> {
    let mut reader = Reader::from_path(path)
        .with_context(|| format!("failed to open summary {}", path.display()))?;
    let mut records = Vec::new();

    for record in reader.deserialize() {
        records
            .push(record.with_context(|| format!("failed to parse summary {}", path.display()))?);
    }

    ensure!(
        !records.is_empty(),
        "summary {} did not contain any records",
        path.display()
    );

    Ok(records)
}

pub fn summary_output_path(raw_output: &Path) -> PathBuf {
    let parent = raw_output.parent();
    let stem = raw_output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("results");
    let file_name = format!("{stem}-summary.csv");

    match parent {
        Some(parent) => parent.join(file_name),
        None => PathBuf::from(file_name),
    }
}

fn fairness_stats_by_parent(records: &[ExperimentRecord]) -> BTreeMap<ParentKey, (Stats, Stats)> {
    let mut grouped_trials: BTreeMap<TrialKey, Vec<&ExperimentRecord>> = BTreeMap::new();

    for record in records {
        grouped_trials
            .entry(TrialKey {
                parent: parent_key(record),
                trial: record.trial,
            })
            .or_default()
            .push(record);
    }

    let mut fairness_by_parent: BTreeMap<ParentKey, Vec<FairnessMetric>> = BTreeMap::new();

    for (trial_key, records) in grouped_trials {
        fairness_by_parent
            .entry(trial_key.parent)
            .or_default()
            .push(fairness_metric(&records));
    }

    fairness_by_parent
        .into_iter()
        .map(|(parent, metrics)| {
            let jain = summarize_values(metrics.iter().map(|metric| metric.jain_fairness_index));
            let variance = summarize_values(
                metrics
                    .iter()
                    .map(|metric| metric.per_user_throughput_variance),
            );
            (parent, (clamp_unit_stats(jain), variance))
        })
        .collect()
}

fn fairness_metric(records: &[&ExperimentRecord]) -> FairnessMetric {
    let total_users: u32 = records.iter().map(|record| record.class_users).sum();
    if total_users == 0 {
        return FairnessMetric {
            jain_fairness_index: 0.0,
            per_user_throughput_variance: 0.0,
        };
    }

    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for record in records {
        let per_user = record.throughput_bits_per_slot / record.class_users.max(1) as f64;
        let users = record.class_users as f64;
        sum += users * per_user;
        sum_sq += users * per_user * per_user;
    }

    let count = total_users as f64;
    let mean = sum / count;
    let variance = records
        .iter()
        .map(|record| {
            let per_user = record.throughput_bits_per_slot / record.class_users.max(1) as f64;
            record.class_users as f64 * (per_user - mean).powi(2)
        })
        .sum::<f64>()
        / count;

    let jain = if sum_sq == 0.0 {
        0.0
    } else {
        (sum * sum) / (count * sum_sq)
    };

    FairnessMetric {
        jain_fairness_index: jain,
        per_user_throughput_variance: variance,
    }
}

fn summarize_values(values: impl IntoIterator<Item = f64>) -> Stats {
    let values: Vec<f64> = values.into_iter().collect();
    if values.is_empty() {
        return Stats {
            mean: 0.0,
            stddev: 0.0,
            ci_low: 0.0,
            ci_high: 0.0,
        };
    }

    let count = values.len() as f64;
    let mean = values.iter().sum::<f64>() / count;
    let variance = if values.len() > 1 {
        values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (count - 1.0)
    } else {
        0.0
    };
    let stddev = variance.sqrt();
    let margin = if values.len() > 1 {
        1.96 * stddev / count.sqrt()
    } else {
        0.0
    };

    Stats {
        mean,
        stddev,
        ci_low: mean - margin,
        ci_high: mean + margin,
    }
}

fn clamp_unit_stats(stats: Stats) -> Stats {
    Stats {
        ci_low: stats.ci_low.clamp(0.0, 1.0),
        ci_high: stats.ci_high.clamp(0.0, 1.0),
        mean: stats.mean.clamp(0.0, 1.0),
        stddev: stats.stddev,
    }
}

fn default_fairness_stats() -> (Stats, Stats) {
    (
        Stats {
            mean: 0.0,
            stddev: 0.0,
            ci_low: 0.0,
            ci_high: 0.0,
        },
        Stats {
            mean: 0.0,
            stddev: 0.0,
            ci_low: 0.0,
            ci_high: 0.0,
        },
    )
}

fn summary_key(record: &ExperimentRecord) -> SummaryKey {
    SummaryKey {
        scenario: record.scenario.clone(),
        timing_preset: record.timing_preset,
        total_users: record.total_users,
        cw_min: record.cw_min,
        lower_cw_min: record.lower_cw_min,
        higher_cw_min: record.higher_cw_min,
        class_name: record.class_name.clone(),
        class_users: record.class_users,
    }
}

fn parent_key(record: &ExperimentRecord) -> ParentKey {
    ParentKey {
        scenario: record.scenario.clone(),
        timing_preset: record.timing_preset,
        total_users: record.total_users,
        cw_min: record.cw_min,
        lower_cw_min: record.lower_cw_min,
        higher_cw_min: record.higher_cw_min,
    }
}

fn parent_key_from_summary(summary: &SummaryKey) -> ParentKey {
    ParentKey {
        scenario: summary.scenario.clone(),
        timing_preset: summary.timing_preset,
        total_users: summary.total_users,
        cw_min: summary.cw_min,
        lower_cw_min: summary.lower_cw_min,
        higher_cw_min: summary.higher_cw_min,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::{TimingPreset, app::output::ExperimentRecord};

    use super::{summarize_records, summary_output_path};

    fn record(
        trial: u32,
        class_name: &str,
        class_users: u32,
        throughput: f64,
        delay: f64,
    ) -> ExperimentRecord {
        ExperimentRecord {
            scenario: String::from("mixed-classes"),
            timing_preset: TimingPreset::Baseline,
            trial,
            seed: u64::from(trial),
            total_users: 10,
            cw_min: None,
            lower_cw_min: Some(4),
            higher_cw_min: Some(16),
            class_name: class_name.to_string(),
            class_users,
            successful_packets: 10,
            collision_attempts: 2,
            average_delay_slots: delay,
            throughput_bits_per_slot: throughput,
        }
    }

    #[test]
    fn summarize_records_captures_fairness_metrics() {
        let records = vec![
            record(0, "lower-cw", 5, 100.0, 10.0),
            record(0, "higher-cw", 5, 50.0, 20.0),
            record(1, "lower-cw", 5, 110.0, 12.0),
            record(1, "higher-cw", 5, 55.0, 22.0),
        ];

        let summaries = summarize_records(&records);
        let lower = summaries
            .iter()
            .find(|record| record.class_name == "lower-cw")
            .expect("lower-cw summary should exist");

        assert_eq!(lower.trials, 2);
        assert!(lower.mean_jain_fairness_index < 1.0);
        assert!(lower.mean_per_user_throughput_variance > 0.0);
        assert_eq!(lower.mean_average_delay_slots, 11.0);
    }

    #[test]
    fn summary_path_adds_summary_suffix() {
        let path = summary_output_path(Path::new("results/users.csv"));

        assert_eq!(path, PathBuf::from("results/users-summary.csv"));
    }
}
