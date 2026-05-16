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
    let station_throughputs: Vec<f64> = stations
        .iter()
        .map(|station| {
            (station.successful_packets as f64 * scenario.timing.payload_bits as f64)
                / scenario.timing.total_slots as f64
        })
        .collect();
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
    let jain_fairness_index = jain_fairness_index(&station_throughputs);
    let per_station_throughput_variance = throughput_variance(&station_throughputs);
    let zero_success_station_fraction = if stations.is_empty() {
        0.0
    } else {
        stations
            .iter()
            .filter(|station| station.successful_packets == 0)
            .count() as f64
            / stations.len() as f64
    };
    let max_station_throughput_share = max_station_share(&station_throughputs);

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
            jain_fairness_index,
            per_station_throughput_variance,
            zero_success_station_fraction,
            max_station_throughput_share,
        },
        per_class,
    }
}

fn jain_fairness_index(throughputs: &[f64]) -> f64 {
    if throughputs.is_empty() {
        return 0.0;
    }

    let sum = throughputs.iter().sum::<f64>();
    let sum_sq = throughputs.iter().map(|value| value * value).sum::<f64>();

    if sum_sq == 0.0 {
        0.0
    } else {
        (sum * sum) / (throughputs.len() as f64 * sum_sq)
    }
}

fn throughput_variance(throughputs: &[f64]) -> f64 {
    if throughputs.is_empty() {
        return 0.0;
    }

    let mean = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
    throughputs
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / throughputs.len() as f64
}

fn max_station_share(throughputs: &[f64]) -> f64 {
    let total = throughputs.iter().sum::<f64>();
    if total == 0.0 {
        return 0.0;
    }

    throughputs.iter().copied().fold(0.0, f64::max) / total
}
