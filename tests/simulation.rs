use csma_ca_simulation::{
    Scenario, SimulationConfig, SimulationSettings, TimingConfig, run, simulate,
};

#[test]
fn single_user_has_no_collisions() {
    let config = SimulationConfig::standard(
        1,
        4,
        SimulationSettings {
            total_slots: 512,
            payload_bits: 1_500,
            cw_max: 32,
            seed: 11,
        },
    );
    let result = simulate(&config).expect("single-user simulation should run");

    assert!(result.aggregate.total_successful_packets > 0);
    assert_eq!(result.aggregate.collision_events, 0);
}

#[test]
fn seeded_runs_are_deterministic() {
    let config = SimulationConfig::standard(
        12,
        16,
        SimulationSettings {
            total_slots: 5_000,
            payload_bits: 12_000,
            cw_max: 128,
            seed: 77,
        },
    );

    let first = simulate(&config).expect("first seeded run should succeed");
    let second = simulate(&config).expect("second seeded run should succeed");

    assert_eq!(first, second);
}

#[test]
fn lower_cw_class_has_better_mixed_class_outcome() {
    let config = SimulationConfig::mixed(
        10,
        10,
        8,
        32,
        SimulationSettings {
            total_slots: 30_000,
            payload_bits: 12_000,
            cw_max: 256,
            seed: 23,
        },
    );
    let result = simulate(&config).expect("mixed-class simulation should run");

    let lower = result
        .per_class
        .iter()
        .find(|class| class.class_name == "lower-cw")
        .expect("lower-cw class should exist");
    let higher = result
        .per_class
        .iter()
        .find(|class| class.class_name == "higher-cw")
        .expect("higher-cw class should exist");

    assert!(lower.throughput_bits_per_slot > higher.throughput_bits_per_slot);
    assert!(lower.average_delay_slots < higher.average_delay_slots);
}

#[test]
fn difs_delays_initial_contention() {
    let scenario = Scenario::standard(
        1,
        1,
        5,
        TimingConfig {
            total_slots: 2,
            payload_bits: 1_500,
            difs_slots: 2,
            sifs_slots: 0,
            tx_duration_slots: 1,
        },
        8,
    );

    let result = run(&scenario).expect("scenario with DIFS should run");

    assert_eq!(result.aggregate.total_successful_packets, 0);
    assert_eq!(result.aggregate.collision_events, 0);
}

#[test]
fn zero_difs_allows_immediate_contention() {
    let scenario = Scenario::standard(
        1,
        1,
        5,
        TimingConfig {
            total_slots: 1,
            payload_bits: 1_500,
            difs_slots: 0,
            sifs_slots: 0,
            tx_duration_slots: 1,
        },
        8,
    );

    let result = run(&scenario).expect("scenario without DIFS should run");

    assert_eq!(result.aggregate.total_successful_packets, 1);
}
