use anyhow::Result;

use crate::{
    config::{SimulationConfig, SimulationSettings, SweepParameters},
    output::ExperimentRecord,
    simulator::{simulate, validate_range_step},
};

pub fn sweep_users(
    min_users: u32,
    max_users: u32,
    step: u32,
    cw_min: u32,
    params: &SweepParameters,
) -> Result<Vec<ExperimentRecord>> {
    let mut records = Vec::new();

    for users in validate_range_step(min_users, max_users, step)? {
        for trial in 0..params.trials {
            let seed = params.base_seed + u64::from(trial);
            let config = SimulationConfig::standard(
                users,
                cw_min,
                SimulationSettings {
                    total_slots: params.total_slots,
                    payload_bits: params.payload_bits,
                    cw_max: params.cw_max,
                    seed,
                },
            );
            let result = simulate(&config)?;

            for class_metrics in result.per_class {
                records.push(ExperimentRecord {
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
                });
            }
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

    for cw_min in validate_range_step(min_cw, max_cw, step)? {
        for trial in 0..params.trials {
            let seed = params.base_seed + u64::from(trial);
            let config = SimulationConfig::standard(
                users,
                cw_min,
                SimulationSettings {
                    total_slots: params.total_slots,
                    payload_bits: params.payload_bits,
                    cw_max: params.cw_max,
                    seed,
                },
            );
            let result = simulate(&config)?;

            for class_metrics in result.per_class {
                records.push(ExperimentRecord {
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
                });
            }
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

    for trial in 0..params.trials {
        let seed = params.base_seed + u64::from(trial);
        let config = SimulationConfig::mixed(
            lower_users,
            higher_users,
            lower_cw_min,
            higher_cw_min,
            SimulationSettings {
                total_slots: params.total_slots,
                payload_bits: params.payload_bits,
                cw_max: params.cw_max,
                seed,
            },
        );
        let total_users = config.total_users();
        let result = simulate(&config)?;

        for class_metrics in result.per_class {
            records.push(ExperimentRecord {
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
            });
        }
    }

    Ok(records)
}
