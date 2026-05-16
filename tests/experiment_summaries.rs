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
    approx_eq(first.mean_average_delay_slots, 21.268573491655967);
    approx_eq(first.ci95_low_average_delay_slots, 16.594041732985012);
    approx_eq(first.mean_throughput_bits_per_slot, 216.796875);

    let second = &summaries[1];
    assert_eq!(second.total_users, 8);
    approx_eq(second.mean_average_delay_slots, 37.86662644787645);
    approx_eq(
        second.ci95_high_throughput_bits_per_slot,
        219.81141001824165,
    );
    approx_eq(second.mean_per_user_throughput_bits_per_slot, 25.390625);
}

#[test]
fn sweep_cw_summary_matches_golden_values() {
    let records =
        experiments::sweep_cwmins(8, 4, 12, 4, &baseline_params()).expect("cw sweep should run");
    let summaries = summarize_records(&records);

    assert_eq!(summaries.len(), 3);

    let lowest = &summaries[0];
    assert_eq!(lowest.cw_min, Some(4));
    approx_eq(lowest.mean_collision_attempts, 49.666666666666664);
    approx_eq(lowest.mean_throughput_bits_per_slot, 203.125);

    let highest = &summaries[2];
    assert_eq!(highest.cw_min, Some(12));
    approx_eq(highest.mean_average_delay_slots, 34.309169983782986);
    approx_eq(
        highest.ci95_low_throughput_bits_per_slot,
        195.18187402361443,
    );
}

#[test]
fn sweep_cw_summary_allows_zero_minimum_window() {
    let records =
        experiments::sweep_cwmins(8, 0, 8, 4, &baseline_params()).expect("cw=0 sweep should run");
    let summaries = summarize_records(&records);

    assert_eq!(summaries.len(), 3);
    assert_eq!(summaries[0].cw_min, Some(0));
    assert!(summaries[0].mean_successful_packets > 0.0);
    assert!(summaries[0].mean_average_delay_slots >= 0.0);
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
    approx_eq(higher.mean_average_delay_slots, 59.644444444444446);
    approx_eq(higher.mean_jain_fairness_index, 0.40132968553583526);

    let lower = summaries
        .iter()
        .find(|summary| summary.class_name == "lower-cw")
        .expect("lower-cw summary should exist");
    approx_eq(lower.mean_throughput_bits_per_slot, 197.265625);
    approx_eq(
        lower.ci95_high_per_user_throughput_variance,
        1888.927983808227,
    );
}
