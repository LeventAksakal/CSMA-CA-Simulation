use std::collections::BTreeMap;

use crate::domain::{
    report::{AggregateReport, ClassReport, SimulationReport},
    scenario::Scenario,
};

use super::station::StationRuntime;

pub fn build_report(
    scenario: &Scenario,
    stations: Vec<StationRuntime>,
    collision_events: u64,
) -> SimulationReport {
    let total_successful_packets = stations
        .iter()
        .map(|station| station.successful_packets)
        .sum();
    let total_delay_slots: u64 = stations
        .iter()
        .map(|station| station.total_delay_slots)
        .sum();
    let throughput_bits_per_slot = (total_successful_packets as f64
        * scenario.timing.payload_bits as f64)
        / scenario.timing.total_slots as f64;
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
                    * scenario.timing.payload_bits as f64)
                    / scenario.timing.total_slots as f64;

                ClassReport {
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

    SimulationReport {
        total_slots: scenario.timing.total_slots,
        payload_bits: scenario.timing.payload_bits,
        aggregate: AggregateReport {
            total_successful_packets,
            collision_events,
            average_delay_slots,
            throughput_bits_per_slot,
        },
        per_class,
    }
}
