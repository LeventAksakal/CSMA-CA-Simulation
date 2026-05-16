use csma_ca_simulation::{
    SweepParameters, TimingPreset,
    app::{experiments, summary::summarize_records},
};

fn approx_eq(left: f64, right: f64) {
    let delta = (left - right).abs();
    assert!(delta < 1e-9, "expected {right}, got {left} (delta {delta})");
}

fn baseline_params() -> SweepParameters {
    SweepParameters {
        total_slots: 256,
        payload_bits: 1_500,
        cw_max: 1_024,
        trials: 3,
        base_seed: 17,
        timing_preset: TimingPreset::Baseline,
    }
}

#[test]
fn sweep_users_summary_matches_golden_values() {
    let records =
        experiments::sweep_users(4, 8, 4, 8, &baseline_params()).expect("users sweep should run");
    let summaries = summarize_records(&records);

    assert_eq!(summaries.len(), 2);

    let first = &summaries[0];
    assert_eq!(first.total_users, 4);
    approx_eq(first.mean_average_delay_slots, 17.92525641025641);
    approx_eq(first.ci95_low_average_delay_slots, 13.735932354717825);
    approx_eq(first.mean_throughput_bits_per_slot, 251.953125);

    let second = &summaries[1];
    assert_eq!(second.total_users, 8);
    approx_eq(second.mean_average_delay_slots, 29.018707482993197);
    approx_eq(second.ci95_high_throughput_bits_per_slot, 287.03125);
    approx_eq(second.mean_per_user_throughput_bits_per_slot, 35.400390625);
}

#[test]
fn sweep_cw_summary_matches_golden_values() {
    let records =
        experiments::sweep_cwmins(8, 4, 12, 4, &baseline_params()).expect("cw sweep should run");
    let summaries = summarize_records(&records);

    assert_eq!(summaries.len(), 3);

    let lowest = &summaries[0];
    assert_eq!(lowest.cw_min, Some(4));
    approx_eq(lowest.mean_collision_attempts, 62.0);
    approx_eq(lowest.mean_throughput_bits_per_slot, 306.640625);

    let highest = &summaries[2];
    assert_eq!(highest.cw_min, Some(12));
    approx_eq(highest.mean_average_delay_slots, 30.73859606705351);
    approx_eq(highest.ci95_low_throughput_bits_per_slot, 257.6818740236144);
}

#[test]
fn mixed_summary_matches_golden_values() {
    let records = experiments::mixed_classes(4, 4, 4, 16, &baseline_params())
        .expect("mixed-class sweep should run");
    let summaries = summarize_records(&records);

    assert_eq!(summaries.len(), 2);

    let higher = summaries
        .iter()
        .find(|summary| summary.class_name == "higher-cw")
        .expect("higher-cw summary should exist");
    approx_eq(higher.mean_average_delay_slots, 56.72380952380953);
    approx_eq(higher.mean_jain_fairness_index, 0.6650949963215028);

    let lower = summaries
        .iter()
        .find(|summary| summary.class_name == "lower-cw")
        .expect("lower-cw summary should exist");
    approx_eq(lower.mean_throughput_bits_per_slot, 255.859375);
    approx_eq(
        lower.ci95_high_per_user_throughput_variance,
        1031.8700516055744,
    );
}
