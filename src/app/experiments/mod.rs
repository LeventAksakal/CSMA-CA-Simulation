mod seed_schedule;

use anyhow::Result;

use crate::{
    app::output::ExperimentRecord,
    domain::{
        config::SweepParameters,
        scenario::{Scenario, TimingConfig},
    },
    sim::{run, validate_range_step},
};

use self::seed_schedule::TrialSeedSchedule;

pub fn sweep_users(
    min_users: u32,
    max_users: u32,
    step: u32,
    cw_min: u32,
    params: &SweepParameters,
) -> Result<Vec<ExperimentRecord>> {
    let mut records = Vec::new();
    let schedule = TrialSeedSchedule::new(params.base_seed, params.trials)?;

    for users in validate_range_step(min_users, max_users, step)? {
        for trial in 0..schedule.trials {
            let seed = schedule.seed_for_trial(trial)?;
            let scenario = standard_scenario(users, cw_min, seed, params);
            let report = run(&scenario)?;

            records.extend(
                report
                    .per_class
                    .into_iter()
                    .map(|class_metrics| ExperimentRecord {
                        scenario: String::from("sweep-users"),
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
                    }),
            );
        }
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
    let mut records = Vec::new();
    let schedule = TrialSeedSchedule::new(params.base_seed, params.trials)?;

    for cw_min in validate_range_step(min_cw, max_cw, step)? {
        for trial in 0..schedule.trials {
            let seed = schedule.seed_for_trial(trial)?;
            let scenario = standard_scenario(users, cw_min, seed, params);
            let report = run(&scenario)?;

            records.extend(
                report
                    .per_class
                    .into_iter()
                    .map(|class_metrics| ExperimentRecord {
                        scenario: String::from("sweep-cw"),
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
                    }),
            );
        }
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
    let mut records = Vec::new();
    let schedule = TrialSeedSchedule::new(params.base_seed, params.trials)?;

    for trial in 0..schedule.trials {
        let seed = schedule.seed_for_trial(trial)?;
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

        records.extend(
            report
                .per_class
                .into_iter()
                .map(|class_metrics| ExperimentRecord {
                    scenario: String::from("mixed-classes"),
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
                }),
        );
    }

    Ok(records)
}

fn standard_scenario(users: u32, cw_min: u32, seed: u64, params: &SweepParameters) -> Scenario {
    Scenario::standard(users, cw_min, seed, timing_config(params), params.cw_max)
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
        timing_config(params),
        params.cw_max,
    )
}

fn timing_config(params: &SweepParameters) -> TimingConfig {
    TimingConfig {
        total_slots: params.total_slots,
        payload_bits: params.payload_bits,
        difs_slots: 1,
        sifs_slots: 0,
        tx_duration_slots: 1,
    }
}
