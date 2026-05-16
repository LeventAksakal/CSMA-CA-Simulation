use anyhow::{Result, ensure};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::domain::report::SimulationReport;
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

pub fn run(scenario: &Scenario) -> Result<SimulationReport> {
    validate_scenario(scenario)?;

    let timing = TimingModel::new(scenario.timing);
    let mut rng = StdRng::seed_from_u64(scenario.seed);
    let mut medium = MediumState::idle();
    let mut stations = build_stations(scenario, &mut rng, timing.defer_slots());
    let mut collision_events = 0_u64;

    for _slot in 0..scenario.timing.total_slots {
        increment_packet_ages(&mut stations);

        if medium.is_busy() {
            medium.consume_busy_slot();
            continue;
        }

        begin_idle_contention(&mut stations, timing.defer_slots());

        let contenders: Vec<usize> = stations
            .iter()
            .enumerate()
            .filter_map(|(index, station)| station.is_ready_to_transmit().then_some(index))
            .collect();

        match resolve_transmission(contenders) {
            TransmissionResolution::Idle => {
                medium.observe_idle_slot();
                advance_idle_slot(&mut stations);
            }
            TransmissionResolution::Success { station_id } => {
                medium.start_busy(timing.busy_slots_after_success());
                handle_success(&mut stations, station_id, &mut rng)?;
            }
            TransmissionResolution::Collision { station_ids } => {
                collision_events += 1;
                medium.start_busy(timing.busy_slots_after_collision());
                handle_collision(&mut stations, &station_ids, &mut rng)?;
            }
        }
    }

    Ok(build_report(scenario, stations, collision_events))
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
        ensure!(class.cw_min > 0, "cw_min must be greater than zero");
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
    ensure!(cw > 0, "contention window must be greater than zero");
    Ok(sample_backoff_unchecked(cw, rng))
}

fn sample_backoff_unchecked(cw: u32, rng: &mut StdRng) -> u32 {
    rng.random_range(0..=cw)
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use crate::domain::scenario::{Scenario, TimingConfig};

    use super::{run, sample_backoff, validate_scenario};

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
}
