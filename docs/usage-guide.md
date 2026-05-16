# Usage Guide

This guide explains how to run the simulator, what each major configuration means, what files each workflow produces, and how to interpret the reported metrics.

## 1. Environment And Validation

Expected local setup:

```powershell
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
pip install pre-commit
pre-commit install --install-hooks --overwrite
```

Repository quality gates:

```powershell
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
pre-commit run --all-files
```

## 2. Command Overview

The CLI provides seven top-level workflows:

- `single`: run one scenario and print metrics to stdout.
- `sweep-users`: vary the number of users while holding the class configuration fixed.
- `sweep-cw`: vary CWmin while holding the user count fixed.
- `mixed-classes`: compare two classes with different CWmin values.
- `plot`: turn summary CSVs into PNG figures.
- `report`: turn summary CSVs into a markdown report.
- `demo`: run the TUI over a live or replayed slot trace.

## 3. Single-Run Workflow

Example:

```powershell
cargo run -- single --users 20 --cw-min 16 --cw-max 1024 --slots 20000 --payload-bits 12000 --seed 7 --timing-preset baseline
```

This command prints:

- aggregate successful packets,
- aggregate collision events,
- aggregate average delay in slots,
- aggregate throughput in bits per slot,
- per-class metrics for each class in the scenario.

Use `single` when you want a fast deterministic sanity check before running a larger sweep.

## 4. Sweep Workflows

### Sweep Users

Example:

```powershell
cargo run -- sweep-users --min-users 10 --max-users 50 --step 10 --cw-min 16 --cw-max 1024 --slots 20000 --payload-bits 12000 --trials 10 --seed 7 --timing-preset baseline --output results/users.csv
```

This expands the user counts `10, 20, 30, 40, 50`, runs each point for `10` trials, and writes:

- `results/users.csv`
- `results/users-summary.csv`

### Sweep CWmin

Example with the repository's seeded CW set:

```powershell
cargo run -- sweep-cw --users 20 --cw-values 0,2,4,8,16,32,64 --cw-max 1024 --slots 20000 --payload-bits 12000 --trials 10 --seed 7 --timing-preset baseline --output results/cw.csv
```

Use `--cw-values` when you want an explicit set. The repository defaults to `0,2,4,8,16,32,64` when no range is provided.

You can also use a linear range:

```powershell
cargo run -- sweep-cw --users 20 --min-cw 8 --max-cw 64 --step 8 --trials 10 --output results/cw.csv
```

This workflow writes:

- `results/cw.csv`
- `results/cw-summary.csv`

### Mixed Classes

Example:

```powershell
cargo run -- mixed-classes --lower-users 10 --higher-users 10 --lower-cw-min 8 --higher-cw-min 32 --cw-max 1024 --slots 20000 --payload-bits 12000 --trials 10 --seed 7 --timing-preset baseline --output results/mixed.csv
```

This creates two station classes in one shared collision domain:

- `lower-cw`: the lower starting CWmin class.
- `higher-cw`: the higher starting CWmin class.

It writes:

- `results/mixed.csv`
- `results/mixed-summary.csv`

Use this workflow when the question is fairness or advantage between classes, not just aggregate throughput.

## 5. Plot And Report Workflows

Generate figures:

```powershell
cargo run -- plot --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --output-dir results/plots
```

Generate the markdown report:

```powershell
cargo run -- report --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --plots-dir results/plots --output results/report.md
```

Expected figure outputs:

- `results/plots/users.png`
- `results/plots/cw.png`
- `results/plots/mixed.png`

Expected report output:

- `results/report.md`

## 6. TUI Demo Workflow

Run a live demo:

```powershell
cargo run -- demo --preset mixed --slots 180 --seed 7 --tick-ms 150
```

Export a deterministic trace while running the demo:

```powershell
cargo run -- demo --preset mixed --slots 180 --seed 7 --tick-ms 150 --export-trace results/traces/mixed-seed7.json
```

Replay an exported trace:

```powershell
cargo run -- demo --replay results/traces/mixed-seed7.json --tick-ms 150
```

Compare runs:

```powershell
cargo run -- demo --preset mixed --slots 180 --seed 7 --compare-seed 11 --tick-ms 150
cargo run -- demo --preset mixed --slots 180 --seed 7 --compare-cw-min 8 --tick-ms 150
```

Demo presets:

- `single`: one station, useful for showing clean defer and success behavior.
- `collision`: higher contention, useful for showing collisions and recovery.
- `mixed`: lower-CW versus higher-CW fairness comparison.

Demo controls:

- `space`: pause or resume.
- `n`: step one slot while paused.
- `f`: speed up playback.
- `s`: slow down playback.
- `r`: restart the current trace or comparison.
- `q`: quit.
- `t`: toggle teaching captions.

## 7. Configuration Reference

### Scenario Size And Reproducibility

- `users`: number of saturated stations in a single-class scenario.
- `lower-users`, `higher-users`: user counts in the mixed-class scenario.
- `trials`: number of independent runs for each sweep point.
- `seed`: base RNG seed. The simulator is deterministic, so the same inputs reproduce the same outputs.

### Contention Window Inputs

- `cw-min`: initial CW for a class and the CW reset target after a success.
- `lower-cw-min`, `higher-cw-min`: per-class CWmin values in the mixed-class scenario.
- `cw-max`: maximum CW allowed after binary exponential backoff growth.
- `min-cw`, `max-cw`, `step`: range expansion inputs for `sweep-cw`.
- `cw-values`: explicit CWmin set for `sweep-cw`.

### Timing And Load Inputs

- `slots`: number of logical slots simulated.
- `payload-bits`: payload credited for each successful transmission.
- `timing-preset`: named logical timing profile.

Timing presets:

- `baseline`: `difs=1`, `sifs=0`, `tx=1`, `collision-penalty=4`
- `short-defer`: `difs=0`, `sifs=0`, `tx=1`, `collision-penalty=4`
- `long-transmission`: `difs=1`, `sifs=1`, `tx=3`, `collision-penalty=6`

Interpretation of these fields:

- `difs`: idle slots that must elapse before stations can resume or begin contention.
- `sifs`: extra busy slots held after a successful transmission.
- `tx`: logical payload transmission duration in slots.
- `collision-penalty`: extra busy slots held after a collision.

These are logical timing abstractions for this study model. They are not direct PHY timing claims in microseconds.

### Output And Post-Processing Inputs

- `output`: raw CSV path for sweep commands.
- `users-input`, `cw-input`, `mixed-input`: summary CSV paths consumed by `plot` and `report`.
- `output-dir`: directory where PNG plots are written.
- `plots-dir`: optional directory path recorded in the markdown report.
- `replay`: trace JSON to replay in the TUI.
- `export-trace`: trace JSON path to write during a live demo.
- `compare-seed`, `compare-cw-min`, `compare-replay`: optional TUI comparison modes.

## 8. Output Files And Schemas

### Raw CSV

The raw CSV stores one row per class per trial. It includes:

- scenario identity,
- timing preset,
- trial number and seed,
- class size and CW settings,
- successful packets,
- collision attempts,
- average delay,
- throughput,
- fairness and starvation indicators copied from the scenario aggregate.

### Summary CSV

The summary CSV groups raw rows by scenario point and reports:

- means,
- standard deviations,
- 95% confidence intervals,
- per-class throughput summaries,
- aggregate fairness, variance, zero-success fraction, and max-station share.

The sweep commands always write the summary file next to the raw file using the `-summary.csv` suffix.

## 9. How To Interpret The Metrics

### Aggregate Efficiency Metrics

- `mean_average_delay_slots`: average packet age at the moment of successful delivery.
- `mean_throughput_bits_per_slot`: delivered payload bits divided by total simulated slots.
- `mean_successful_packets`: average number of successful transmissions.
- `mean_collision_attempts`: average failed transmission attempts.

Delay in this simulator is success-conditioned. If a station never succeeds, it does not directly raise the reported average delay value, but it does show up in the starvation metrics.

### Fairness And Starvation Metrics

- `mean_jain_fairness_index`: equality across per-station throughputs. Higher is better.
- `mean_per_user_throughput_variance`: spread across per-station throughputs. Lower is better.
- `mean_zero_success_station_fraction`: share of stations with zero successes. Lower is better.
- `mean_max_station_throughput_share`: dominance of the top station. Lower is better.

Read these together:

- high throughput plus low fairness usually means capture, not a healthy configuration,
- low delay plus high zero-success fraction means successful packets are moving quickly but many stations are being starved,
- max-station share near `1.0` means one station is carrying almost all delivered throughput.

### Confidence Intervals

The summary CSV and plots use normal-approximation 95% confidence intervals across trials. Narrow intervals mean the metric is stable across seeds for that scenario point. Wide intervals mean the point is sensitive to random backoff evolution.

## 10. How To Read The Plots

### Users Sweep Plot

This figure shows how increasing the number of users changes:

- average delay,
- throughput,
- fairness,
- per-station throughput variance,
- zero-success fraction,
- max-station share.

Use it to answer whether scaling the offered contention causes congestion, unfairness, or both.

### CW Sweep Plot

This figure shows how changing CWmin affects:

- average delay,
- throughput,
- fairness,
- per-station throughput variance,
- zero-success fraction,
- max-station share.

Do not judge CWmin from throughput alone. In this model, very small CWmin values can keep throughput high while producing extremely poor fairness and heavy starvation.

### Mixed-Class Plot

This figure compares the two classes on delay and throughput, and shows aggregate fairness, starvation, and capture metrics for the mixed scenario.

Use it to answer whether the lower-CW class is advantaged and how severe that advantage becomes.

## 11. Recommended End-To-End Workflow

For a clean experiment run:

```powershell
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -- sweep-users --min-users 10 --max-users 50 --step 10 --cw-min 16 --trials 10 --timing-preset baseline --output results/users.csv
cargo run -- sweep-cw --users 20 --cw-values 0,2,4,8,16,32,64 --trials 10 --timing-preset baseline --output results/cw.csv
cargo run -- mixed-classes --lower-users 10 --higher-users 10 --lower-cw-min 8 --higher-cw-min 32 --trials 10 --timing-preset baseline --output results/mixed.csv
cargo run -- plot --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --output-dir results/plots
cargo run -- report --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --plots-dir results/plots --output results/report.md
```

## 12. Common Interpretation Mistakes

- Treating average delay as if it includes permanently starved packets. It does not.
- Treating throughput as a fairness metric. It is not.
- Reading a low-delay point as healthy without checking zero-success fraction and max-station share.
- Treating the timing presets as exact 802.11 PHY timing.
- Claiming strict IEEE conformance from this repository alone.

For the simulator mechanics behind these outputs, see [dcf-model.md](dcf-model.md).
