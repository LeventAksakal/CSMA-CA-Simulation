mod seed_schedule;

use anyhow::Result;

use crate::{
    app::output::ExperimentRecord,
    domain::{config::SweepParameters, scenario::Scenario},
    sim::run,
};

use self::seed_schedule::TrialSeedSchedule;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

pub fn sweep_users(
    min_users: u32,
    max_users: u32,
    step: u32,
    cw_min: u32,
    params: &SweepParameters,
) -> Result<Vec<ExperimentRecord>> {
    let mut records = Vec::new();
    let schedule = TrialSeedSchedule::new(params.base_seed, params.trials)?;

    for users in expand_linear_values(min_users, max_users, step)? {
        records.extend(collect_trial_record_batches(&schedule, |trial, seed| {
            let scenario = standard_scenario(users, cw_min, seed, params);
            let report = run(&scenario)?;

            Ok(report
                .per_class
                .into_iter()
                .map(|class_metrics| ExperimentRecord {
                    scenario: String::from("sweep-users"),
                    timing_preset: params.timing_preset,
                    trial,
                    seed,
                    total_users: users,
                    cw_min: Some(cw_min),
                    lower_cw_min: None,
                    higher_cw_min: None,
                    class_name: class_metrics.class_name,
                    class_users: class_metrics.users,
                    successful_packets: class_metrics.successful_packets,
                    collision_attempts: class_metrics.collision_attempts,
                    average_delay_slots: class_metrics.average_delay_slots,
                    throughput_bits_per_slot: class_metrics.throughput_bits_per_slot,
                    jain_fairness_index: report.aggregate.jain_fairness_index,
                    per_station_throughput_variance: report
                        .aggregate
                        .per_station_throughput_variance,
                    zero_success_station_fraction: report.aggregate.zero_success_station_fraction,
                    max_station_throughput_share: report.aggregate.max_station_throughput_share,
                })
                .collect())
        })?);
    }

    Ok(records)
}

pub fn sweep_cwmins(
    users: u32,
    min_cw: u32,
    max_cw: u32,
    step: u32,
    params: &SweepParameters,
) -> Result<Vec<ExperimentRecord>> {
    let values = expand_linear_values(min_cw, max_cw, step)?;
    sweep_cw_values(users, &values, params)
}

pub fn sweep_cw_values(
    users: u32,
    cw_values: &[u32],
    params: &SweepParameters,
) -> Result<Vec<ExperimentRecord>> {
    let mut records = Vec::new();
    let schedule = TrialSeedSchedule::new(params.base_seed, params.trials)?;

    for &cw_min in cw_values {
        records.extend(collect_trial_record_batches(&schedule, |trial, seed| {
            let scenario = standard_scenario(users, cw_min, seed, params);
            let report = run(&scenario)?;

            Ok(report
                .per_class
                .into_iter()
                .map(|class_metrics| ExperimentRecord {
                    scenario: String::from("sweep-cw"),
                    timing_preset: params.timing_preset,
                    trial,
                    seed,
                    total_users: users,
                    cw_min: Some(cw_min),
                    lower_cw_min: None,
                    higher_cw_min: None,
                    class_name: class_metrics.class_name,
                    class_users: class_metrics.users,
                    successful_packets: class_metrics.successful_packets,
                    collision_attempts: class_metrics.collision_attempts,
                    average_delay_slots: class_metrics.average_delay_slots,
                    throughput_bits_per_slot: class_metrics.throughput_bits_per_slot,
                    jain_fairness_index: report.aggregate.jain_fairness_index,
                    per_station_throughput_variance: report
                        .aggregate
                        .per_station_throughput_variance,
                    zero_success_station_fraction: report.aggregate.zero_success_station_fraction,
                    max_station_throughput_share: report.aggregate.max_station_throughput_share,
                })
                .collect())
        })?);
    }

    Ok(records)
}

pub fn mixed_classes(
    lower_users: u32,
    higher_users: u32,
    lower_cw_min: u32,
    higher_cw_min: u32,
    params: &SweepParameters,
) -> Result<Vec<ExperimentRecord>> {
    let schedule = TrialSeedSchedule::new(params.base_seed, params.trials)?;

    collect_trial_record_batches(&schedule, |trial, seed| {
        let scenario = mixed_scenario(
            lower_users,
            higher_users,
            lower_cw_min,
            higher_cw_min,
            seed,
            params,
        );
        let total_users = scenario.total_users();
        let report = run(&scenario)?;

        Ok(report
            .per_class
            .into_iter()
            .map(|class_metrics| ExperimentRecord {
                scenario: String::from("mixed-classes"),
                timing_preset: params.timing_preset,
                trial,
                seed,
                total_users,
                cw_min: None,
                lower_cw_min: Some(lower_cw_min),
                higher_cw_min: Some(higher_cw_min),
                class_name: class_metrics.class_name,
                class_users: class_metrics.users,
                successful_packets: class_metrics.successful_packets,
                collision_attempts: class_metrics.collision_attempts,
                average_delay_slots: class_metrics.average_delay_slots,
                throughput_bits_per_slot: class_metrics.throughput_bits_per_slot,
                jain_fairness_index: report.aggregate.jain_fairness_index,
                per_station_throughput_variance: report.aggregate.per_station_throughput_variance,
                zero_success_station_fraction: report.aggregate.zero_success_station_fraction,
                max_station_throughput_share: report.aggregate.max_station_throughput_share,
            })
            .collect())
    })
}

fn standard_scenario(users: u32, cw_min: u32, seed: u64, params: &SweepParameters) -> Scenario {
    Scenario::standard(
        users,
        cw_min,
        seed,
        params
            .timing_preset
            .timing_config(params.total_slots, params.payload_bits),
        params.cw_max,
    )
}

fn mixed_scenario(
    lower_users: u32,
    higher_users: u32,
    lower_cw_min: u32,
    higher_cw_min: u32,
    seed: u64,
    params: &SweepParameters,
) -> Scenario {
    Scenario::mixed(
        lower_users,
        higher_users,
        lower_cw_min,
        higher_cw_min,
        seed,
        params
            .timing_preset
            .timing_config(params.total_slots, params.payload_bits),
        params.cw_max,
    )
}

fn expand_linear_values(start: u32, end: u32, step: u32) -> Result<Vec<u32>> {
    use anyhow::{anyhow, ensure};

    ensure!(step > 0, "step must be greater than zero");
    ensure!(
        start <= end,
        "range start must be less than or equal to range end"
    );

    let mut values = Vec::new();
    let mut current = start;

    while current <= end {
        values.push(current);

        match current.checked_add(step) {
            Some(next) if next > current => current = next,
            _ => return Err(anyhow!("range overflow while expanding values")),
        }
    }

    Ok(values)
}

#[cfg(feature = "rayon")]
fn collect_trial_record_batches<F>(
    schedule: &TrialSeedSchedule,
    build_records: F,
) -> Result<Vec<ExperimentRecord>>
where
    F: Fn(u32, u64) -> Result<Vec<ExperimentRecord>> + Sync + Send,
{
    let mut per_trial = (0..schedule.trials)
        .into_par_iter()
        .map(|trial| {
            let seed = schedule.seed_for_trial(trial)?;
            let records = build_records(trial, seed)?;
            Ok((trial, records))
        })
        .collect::<Result<Vec<_>>>()?;

    per_trial.sort_by_key(|(trial, _)| *trial);

    Ok(per_trial
        .into_iter()
        .flat_map(|(_, records)| records)
        .collect())
}

#[cfg(not(feature = "rayon"))]
fn collect_trial_record_batches<F>(
    schedule: &TrialSeedSchedule,
    build_records: F,
) -> Result<Vec<ExperimentRecord>>
where
    F: Fn(u32, u64) -> Result<Vec<ExperimentRecord>>,
{
    let mut records = Vec::new();

    for trial in 0..schedule.trials {
        let seed = schedule.seed_for_trial(trial)?;
        records.extend(build_records(trial, seed)?);
    }

    Ok(records)
}
