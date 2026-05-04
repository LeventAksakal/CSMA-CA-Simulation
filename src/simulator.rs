use std::collections::BTreeMap;

use anyhow::{Result, anyhow, ensure};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    config::SimulationConfig,
    metrics::{AggregateMetrics, ClassMetrics, SimulationResult},
    model::{StationState, TransmissionOutcome},
};

pub fn simulate(config: &SimulationConfig) -> Result<SimulationResult> {
    validate_config(config)?;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut stations = build_stations(config, &mut rng);
    let mut collision_events = 0_u64;

    for _slot in 0..config.total_slots {
        let contenders: Vec<usize> = stations
            .iter()
            .enumerate()
            .filter_map(|(index, station)| (station.backoff == 0).then_some(index))
            .collect();

        increment_packet_ages(&mut stations);

        let outcome = match contenders.as_slice() {
            [] => TransmissionOutcome::Idle,
            [station_id] => TransmissionOutcome::Success {
                station_id: *station_id,
            },
            _ => TransmissionOutcome::Collision {
                station_ids: contenders,
            },
        };

        match outcome {
            TransmissionOutcome::Idle => decrement_backoff_counters(&mut stations),
            TransmissionOutcome::Success { station_id } => {
                let station = &mut stations[station_id];
                station.successful_packets += 1;
                station.total_delay_slots += station.packet_age_slots;
                station.packet_age_slots = 0;
                station.current_cw = station.cw_min;
                station.backoff = sample_backoff(station.current_cw, &mut rng)?;
            }
            TransmissionOutcome::Collision { station_ids } => {
                collision_events += 1;

                for station_id in station_ids {
                    let station = &mut stations[station_id];
                    station.collision_attempts += 1;
                    station.current_cw = station.current_cw.saturating_mul(2).min(config.cw_max);
                    station.backoff = sample_backoff(station.current_cw, &mut rng)?;
                }
            }
        }
    }

    let total_successful_packets = stations
        .iter()
        .map(|station| station.successful_packets)
        .sum();
    let total_delay_slots: u64 = stations
        .iter()
        .map(|station| station.total_delay_slots)
        .sum();
    let throughput_bits_per_slot =
        (total_successful_packets as f64 * config.payload_bits as f64) / config.total_slots as f64;
    let average_delay_slots = if total_successful_packets == 0 {
        0.0
    } else {
        total_delay_slots as f64 / total_successful_packets as f64
    };

    let mut grouped: BTreeMap<String, (u32, u64, u64, u64)> = BTreeMap::new();

    for station in stations {
        let entry = grouped.entry(station.class_name).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        entry.1 += station.successful_packets;
        entry.2 += station.collision_attempts;
        entry.3 += station.total_delay_slots;
    }

    let per_class = grouped
        .into_iter()
        .map(
            |(class_name, (users, successful_packets, collision_attempts, total_delay_slots))| {
                let average_delay_slots = if successful_packets == 0 {
                    0.0
                } else {
                    total_delay_slots as f64 / successful_packets as f64
                };

                let throughput_bits_per_slot = (successful_packets as f64
                    * config.payload_bits as f64)
                    / config.total_slots as f64;

                ClassMetrics {
                    class_name,
                    users,
                    successful_packets,
                    collision_attempts,
                    average_delay_slots,
                    throughput_bits_per_slot,
                }
            },
        )
        .collect();

    Ok(SimulationResult {
        total_slots: config.total_slots,
        payload_bits: config.payload_bits,
        aggregate: AggregateMetrics {
            total_successful_packets,
            collision_events,
            average_delay_slots,
            throughput_bits_per_slot,
        },
        per_class,
    })
}

fn validate_config(config: &SimulationConfig) -> Result<()> {
    ensure!(
        config.total_slots > 0,
        "total_slots must be greater than zero"
    );
    ensure!(
        config.payload_bits > 0,
        "payload_bits must be greater than zero"
    );
    ensure!(
        !config.classes.is_empty(),
        "at least one station class is required"
    );

    for class in &config.classes {
        ensure!(
            class.users > 0,
            "station classes must contain at least one user"
        );
        ensure!(class.cw_min > 0, "cw_min must be greater than zero");
        ensure!(
            class.cw_min <= config.cw_max,
            "cw_min must be less than or equal to cw_max"
        );
    }

    Ok(())
}

fn build_stations(config: &SimulationConfig, rng: &mut StdRng) -> Vec<StationState> {
    let mut stations = Vec::with_capacity(config.total_users() as usize);

    for class in &config.classes {
        for _ in 0..class.users {
            let id = stations.len();
            let backoff = sample_backoff_unchecked(class.cw_min, rng);

            stations.push(StationState {
                id,
                class_name: class.name.clone(),
                cw_min: class.cw_min,
                current_cw: class.cw_min,
                backoff,
                packet_age_slots: 0,
                successful_packets: 0,
                collision_attempts: 0,
                total_delay_slots: 0,
            });
        }
    }

    stations
}

fn increment_packet_ages(stations: &mut [StationState]) {
    for station in stations {
        station.packet_age_slots += 1;
    }
}

fn decrement_backoff_counters(stations: &mut [StationState]) {
    for station in stations {
        if station.backoff > 0 {
            station.backoff -= 1;
        }
    }
}

fn sample_backoff(cw: u32, rng: &mut StdRng) -> Result<u32> {
    ensure!(cw > 0, "contention window must be greater than zero");
    Ok(sample_backoff_unchecked(cw, rng))
}

fn sample_backoff_unchecked(cw: u32, rng: &mut StdRng) -> u32 {
    rng.random_range(0..cw)
}

pub fn validate_range_step(start: u32, end: u32, step: u32) -> Result<Vec<u32>> {
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
