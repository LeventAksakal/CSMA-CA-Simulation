use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};

use crate::domain::report::{AggregateReport, ClassReport, SimulationReport};
use crate::domain::scenario::Scenario;

use super::{
    backoff::BackoffState,
    medium::MediumState,
    metrics::build_report,
    phase::StationPhase,
    resolver::{TransmissionResolution, resolve_transmission},
    station::StationRuntime,
    timing::TimingModel,
    window::ContentionWindow,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StationTraceSnapshot {
    pub id: usize,
    pub class_name: String,
    pub phase: StationPhase,
    pub current_cw: u32,
    pub backoff_counter: u32,
    pub frozen_backoff_counter: Option<u32>,
    pub defer_slots_remaining: u32,
    pub packet_age_slots: u64,
    pub successful_packets: u64,
    pub collision_attempts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceProgressSnapshot {
    pub elapsed_slots: u64,
    pub aggregate: AggregateReport,
    pub per_class: Vec<ClassReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotEvent {
    Busy { busy_slots_remaining: u32 },
    Idle,
    Success { station_id: usize },
    Collision { station_ids: Vec<usize> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceFrame {
    pub slot: u64,
    pub medium_busy: bool,
    pub medium_busy_slots_remaining: u32,
    pub idle_slots: u32,
    pub event: SlotEvent,
    pub stations: Vec<StationTraceSnapshot>,
    pub progress: TraceProgressSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationTrace {
    pub frames: Vec<TraceFrame>,
    pub report: SimulationReport,
}

struct DcfSimulation {
    scenario: Scenario,
    timing: TimingModel,
    rng: StdRng,
    medium: MediumState,
    stations: Vec<StationRuntime>,
    collision_events: u64,
    next_slot: u64,
}

pub fn run(scenario: &Scenario) -> Result<SimulationReport> {
    let mut simulation = DcfSimulation::new(scenario)?;

    while simulation.step()?.is_some() {}

    Ok(simulation.into_report())
}

pub fn trace(scenario: &Scenario) -> Result<SimulationTrace> {
    let mut simulation = DcfSimulation::new(scenario)?;
    let mut frames = Vec::new();

    while let Some(frame) = simulation.step()? {
        frames.push(frame);
    }

    let report = simulation.into_report();

    Ok(SimulationTrace { frames, report })
}

impl DcfSimulation {
    fn new(scenario: &Scenario) -> Result<Self> {
        validate_scenario(scenario)?;

        let timing = TimingModel::new(scenario.timing);
        let mut rng = StdRng::seed_from_u64(scenario.seed);
        let stations = build_stations(scenario, &mut rng, timing.defer_slots());

        Ok(Self {
            scenario: scenario.clone(),
            timing,
            rng,
            medium: MediumState::idle(),
            stations,
            collision_events: 0,
            next_slot: 0,
        })
    }

    fn step(&mut self) -> Result<Option<TraceFrame>> {
        if self.next_slot >= self.scenario.timing.total_slots {
            return Ok(None);
        }

        let slot = self.next_slot;
        increment_packet_ages(&mut self.stations);

        let event = if self.medium.is_busy() {
            self.medium.consume_busy_slot();
            SlotEvent::Busy {
                busy_slots_remaining: self.medium.busy_slots_remaining,
            }
        } else {
            begin_idle_contention(&mut self.stations, self.timing.defer_slots());

            let contenders: Vec<usize> = self
                .stations
                .iter()
                .enumerate()
                .filter_map(|(index, station)| station.is_ready_to_transmit().then_some(index))
                .collect();

            match resolve_transmission(contenders) {
                TransmissionResolution::Idle => {
                    self.medium.observe_idle_slot();
                    advance_idle_slot(&mut self.stations);
                    SlotEvent::Idle
                }
                TransmissionResolution::Success { station_id } => {
                    self.medium
                        .start_busy(self.timing.busy_slots_after_success());
                    handle_success(&mut self.stations, station_id, &mut self.rng)?;
                    SlotEvent::Success { station_id }
                }
                TransmissionResolution::Collision { station_ids } => {
                    self.collision_events += 1;
                    self.medium
                        .start_busy(self.timing.busy_slots_after_collision());
                    handle_collision(&mut self.stations, &station_ids, &mut self.rng)?;
                    SlotEvent::Collision { station_ids }
                }
            }
        };

        let elapsed_slots = slot + 1;
        let frame = TraceFrame {
            slot,
            medium_busy: self.medium.is_busy(),
            medium_busy_slots_remaining: self.medium.busy_slots_remaining,
            idle_slots: self.medium.idle_slots,
            event,
            stations: snapshot_stations(&self.stations),
            progress: build_progress_snapshot(
                &self.scenario,
                &self.stations,
                self.collision_events,
                elapsed_slots,
            ),
        };
        self.next_slot = elapsed_slots;

        Ok(Some(frame))
    }

    fn into_report(self) -> SimulationReport {
        build_report(&self.scenario, self.stations, self.collision_events)
    }
}

fn validate_scenario(scenario: &Scenario) -> Result<()> {
    ensure!(
        scenario.timing.total_slots > 0,
        "total_slots must be greater than zero"
    );
    ensure!(
        scenario.timing.payload_bits > 0,
        "payload_bits must be greater than zero"
    );
    ensure!(
        scenario.window.cw_max > 0,
        "cw_max must be greater than zero"
    );
    ensure!(
        scenario.timing.tx_duration_slots > 0,
        "tx_duration_slots must be greater than zero"
    );
    ensure!(
        !scenario.classes.is_empty(),
        "at least one station class is required"
    );

    for class in &scenario.classes {
        ensure!(
            class.users > 0,
            "station classes must contain at least one user"
        );
        ensure!(
            class.cw_min <= scenario.window.cw_max,
            "cw_min must be less than or equal to cw_max"
        );
    }

    Ok(())
}

fn build_stations(
    scenario: &Scenario,
    rng: &mut StdRng,
    initial_defer_slots: u32,
) -> Vec<StationRuntime> {
    let mut stations = Vec::with_capacity(scenario.total_users() as usize);

    for class in &scenario.classes {
        for _ in 0..class.users {
            let id = stations.len();
            let backoff = sample_backoff_unchecked(class.cw_min, rng);

            let mut station = StationRuntime {
                id,
                class_name: class.name.clone(),
                phase: StationPhase::WaitingForMedium,
                cw_min: class.cw_min,
                window: ContentionWindow::new(class.cw_min, scenario.window.cw_max),
                backoff: BackoffState::new(backoff),
                defer_slots_remaining: 0,
                packet_age_slots: 0,
                successful_packets: 0,
                collision_attempts: 0,
                total_delay_slots: 0,
            };
            station.enter_defer(initial_defer_slots);

            stations.push(station);
        }
    }

    stations
}

fn increment_packet_ages(stations: &mut [StationRuntime]) {
    for station in stations {
        station.packet_age_slots += 1;
    }
}

fn begin_idle_contention(stations: &mut [StationRuntime], defer_slots: u32) {
    for station in stations {
        if station.phase == StationPhase::WaitingForMedium {
            station.enter_defer(defer_slots);
        }
    }
}

fn advance_idle_slot(stations: &mut [StationRuntime]) {
    for station in stations {
        station.advance_idle_slot();
    }
}

fn handle_success(
    stations: &mut [StationRuntime],
    station_id: usize,
    rng: &mut StdRng,
) -> Result<()> {
    for (index, station) in stations.iter_mut().enumerate() {
        if index == station_id {
            station.phase = StationPhase::Transmitting;
            station.successful_packets += 1;
            station.total_delay_slots += station.packet_age_slots;
            station.packet_age_slots = 0;
            station.window.reset();
            station
                .backoff
                .replace(sample_backoff(station.current_cw(), rng)?);
            station.phase = StationPhase::AwaitingResult;
            station.wait_for_medium();
        } else {
            station.freeze_for_busy();
        }
    }

    Ok(())
}

fn handle_collision(
    stations: &mut [StationRuntime],
    station_ids: &[usize],
    rng: &mut StdRng,
) -> Result<()> {
    for (index, station) in stations.iter_mut().enumerate() {
        if station_ids.contains(&index) {
            station.phase = StationPhase::Transmitting;
            station.phase = StationPhase::CollisionRecovery;
            station.collision_attempts += 1;
            station.window.increase_binary_exponential();
            station
                .backoff
                .replace(sample_backoff(station.current_cw(), rng)?);
            station.wait_for_medium();
        } else {
            station.freeze_for_busy();
        }
    }

    Ok(())
}

fn sample_backoff(cw: u32, rng: &mut StdRng) -> Result<u32> {
    Ok(sample_backoff_unchecked(cw, rng))
}

fn sample_backoff_unchecked(cw: u32, rng: &mut StdRng) -> u32 {
    rng.random_range(0..=cw)
}

fn snapshot_stations(stations: &[StationRuntime]) -> Vec<StationTraceSnapshot> {
    stations
        .iter()
        .map(|station| StationTraceSnapshot {
            id: station.id,
            class_name: station.class_name.clone(),
            phase: station.phase,
            current_cw: station.current_cw(),
            backoff_counter: station.backoff.counter,
            frozen_backoff_counter: station.backoff.frozen_counter,
            defer_slots_remaining: station.defer_slots_remaining,
            packet_age_slots: station.packet_age_slots,
            successful_packets: station.successful_packets,
            collision_attempts: station.collision_attempts,
        })
        .collect()
}

fn build_progress_snapshot(
    scenario: &Scenario,
    stations: &[StationRuntime],
    collision_events: u64,
    elapsed_slots: u64,
) -> TraceProgressSnapshot {
    let total_successful_packets = stations
        .iter()
        .map(|station| station.successful_packets)
        .sum();
    let total_delay_slots: u64 = stations
        .iter()
        .map(|station| station.total_delay_slots)
        .sum();
    let throughput_bits_per_slot = if elapsed_slots == 0 {
        0.0
    } else {
        (total_successful_packets as f64 * scenario.timing.payload_bits as f64)
            / elapsed_slots as f64
    };
    let average_delay_slots = if total_successful_packets == 0 {
        0.0
    } else {
        total_delay_slots as f64 / total_successful_packets as f64
    };
    let station_throughputs: Vec<f64> = stations
        .iter()
        .map(|station| {
            if elapsed_slots == 0 {
                0.0
            } else {
                (station.successful_packets as f64 * scenario.timing.payload_bits as f64)
                    / elapsed_slots as f64
            }
        })
        .collect();
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
        let entry = grouped
            .entry(station.class_name.clone())
            .or_insert((0, 0, 0, 0));
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
                let throughput_bits_per_slot = if elapsed_slots == 0 {
                    0.0
                } else {
                    (successful_packets as f64 * scenario.timing.payload_bits as f64)
                        / elapsed_slots as f64
                };

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

    TraceProgressSnapshot {
        elapsed_slots,
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

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use crate::domain::scenario::{Scenario, TimingConfig};

    use super::{SlotEvent, run, sample_backoff, trace, validate_scenario};

    #[test]
    fn validation_rejects_zero_tx_duration() {
        let scenario = Scenario::standard(
            1,
            4,
            1,
            TimingConfig {
                total_slots: 10,
                payload_bits: 1_500,
                difs_slots: 1,
                sifs_slots: 0,
                tx_duration_slots: 0,
                collision_penalty_slots: 4,
            },
            32,
        );

        assert!(validate_scenario(&scenario).is_err());
    }

    #[test]
    fn transmit_duration_and_sifs_extend_busy_period() {
        let scenario = Scenario::standard(
            1,
            1,
            1,
            TimingConfig {
                total_slots: 3,
                payload_bits: 1_500,
                difs_slots: 0,
                sifs_slots: 1,
                tx_duration_slots: 2,
                collision_penalty_slots: 4,
            },
            8,
        );

        let result = run(&scenario).expect("scenario should run");

        assert_eq!(result.aggregate.total_successful_packets, 1);
    }

    #[test]
    fn sample_backoff_can_hit_inclusive_upper_bound() {
        let saw_upper_bound = (0_u64..64).any(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            sample_backoff(1, &mut rng).expect("cw=1 should be valid") == 1
        });

        assert!(saw_upper_bound);
    }

    #[test]
    fn sample_backoff_allows_zero_window() {
        let mut rng = StdRng::seed_from_u64(7);

        let sample = sample_backoff(0, &mut rng).expect("cw=0 should be valid");

        assert_eq!(sample, 0);
    }

    #[test]
    fn zero_cw_min_scenario_runs_as_pathological_case() {
        let scenario = Scenario::standard(
            4,
            0,
            7,
            TimingConfig {
                total_slots: 32,
                payload_bits: 1_500,
                difs_slots: 0,
                sifs_slots: 0,
                tx_duration_slots: 1,
                collision_penalty_slots: 4,
            },
            31,
        );

        let result = run(&scenario).expect("cw=0 scenario should run");

        assert!(result.aggregate.collision_events > 0);
    }

    #[test]
    fn trace_emits_one_frame_per_slot() {
        let scenario = Scenario::standard(
            2,
            1,
            5,
            TimingConfig {
                total_slots: 6,
                payload_bits: 1_500,
                difs_slots: 0,
                sifs_slots: 0,
                tx_duration_slots: 1,
                collision_penalty_slots: 4,
            },
            7,
        );

        let result = trace(&scenario).expect("trace should run");

        assert_eq!(result.frames.len(), 6);
        assert_eq!(result.report.total_slots, 6);
    }

    #[test]
    fn trace_captures_collision_or_success_events() {
        let scenario = Scenario::standard(
            4,
            1,
            7,
            TimingConfig {
                total_slots: 12,
                payload_bits: 1_500,
                difs_slots: 0,
                sifs_slots: 0,
                tx_duration_slots: 1,
                collision_penalty_slots: 4,
            },
            7,
        );

        let result = trace(&scenario).expect("trace should run");

        assert!(result.frames.iter().any(|frame| {
            matches!(
                frame.event,
                SlotEvent::Collision { .. } | SlotEvent::Success { .. }
            )
        }));
    }
}
