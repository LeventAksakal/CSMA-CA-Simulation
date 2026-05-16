# CSMA/CA Simulation

Rust-first CSMA/CA simulator for reproducible DCF studies. The repository provides:

- an explicit slot-based medium-access model,
- deterministic seeded runs,
- CLI workflows for single runs and parameter sweeps,
- CSV, PNG, markdown, and JSON trace outputs,
- tests around the core DCF behaviors the simulator claims to model.

This project targets an inspectable study model inspired by IEEE 802.11 DCF. It does not claim exact clause-level IEEE 802.11 conformance.

## Documentation Map

- [docs/README.md](docs/README.md): documentation index.
- [docs/usage-guide.md](docs/usage-guide.md): how to run the simulator, what each flag means, and how to interpret the outputs.
- [docs/dcf-model.md](docs/dcf-model.md): the simulator's DCF model, assumptions, timing abstraction, and limitations.
- [docs/reference/IEEE-80211-2024.txt](docs/reference/IEEE-80211-2024.txt): repository-local CSMA/CA behavior specification derived from public sources.

## Quick Start

Install the expected local toolchain:

```powershell
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
pip install pre-commit
pre-commit install --install-hooks --overwrite
```

Run the validation gates:

```powershell
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
pre-commit run --all-files
```

Run one deterministic scenario:

```powershell
cargo run -- single --users 20 --cw-min 16 --slots 20000 --seed 7 --timing-preset baseline
```

Generate the standard coursework-style outputs:

```powershell
cargo run -- sweep-users --min-users 10 --max-users 50 --step 10 --cw-min 16 --trials 10 --timing-preset baseline --output results/users.csv
cargo run -- sweep-cw --users 20 --cw-values 0,2,4,8,16,32,64 --trials 10 --timing-preset baseline --output results/cw.csv
cargo run -- mixed-classes --lower-users 10 --higher-users 10 --lower-cw-min 8 --higher-cw-min 32 --trials 10 --timing-preset baseline --output results/mixed.csv
cargo run -- plot --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --output-dir results/plots
cargo run -- report --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --plots-dir results/plots --output results/report.md
```

## CLI Workflows

Run a single scenario and print aggregate plus per-class metrics:

```powershell
cargo run -- single --users 20 --cw-min 16 --slots 20000 --payload-bits 12000 --seed 7 --timing-preset baseline
```

Sweep user count:

```powershell
cargo run -- sweep-users --min-users 10 --max-users 50 --step 10 --cw-min 16 --trials 5 --timing-preset baseline --output results/users.csv
```

Sweep CWmin with the seeded set used by the repository:

```powershell
cargo run -- sweep-cw --users 20 --cw-values 0,2,4,8,16,32,64 --trials 5 --timing-preset baseline --output results/cw.csv
```

Run the mixed-class fairness scenario:

```powershell
cargo run -- mixed-classes --lower-users 10 --higher-users 10 --lower-cw-min 8 --higher-cw-min 32 --trials 10 --timing-preset baseline --output results/mixed.csv
```

Render plots and a markdown report from summary CSVs:

```powershell
cargo run -- plot --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --output-dir results/plots
cargo run -- report --users-input results/users-summary.csv --cw-input results/cw-summary.csv --mixed-input results/mixed-summary.csv --plots-dir results/plots --output results/report.md
```

Launch the live TUI demo, export a trace, replay a trace, or compare two runs:

```powershell
cargo run -- demo --preset mixed --slots 180 --seed 7 --tick-ms 150
cargo run -- demo --preset mixed --slots 180 --seed 7 --tick-ms 150 --export-trace results/traces/mixed-seed7.json
cargo run -- demo --replay results/traces/mixed-seed7.json --tick-ms 150
cargo run -- demo --preset mixed --slots 180 --seed 7 --compare-seed 11 --tick-ms 150
cargo run -- demo --preset mixed --slots 180 --seed 7 --compare-cw-min 8 --tick-ms 150
```

Enable optional parallel trial execution:

```powershell
cargo run --features rayon -- sweep-users --min-users 10 --max-users 50 --step 10 --cw-min 16 --trials 10 --output results/users.csv
```

## What The Main Inputs Mean

- `users`: number of saturated stations in a single-class run.
- `lower-users` and `higher-users`: user counts for the two mixed classes.
- `cw-min`: class starting contention window. After a success the station resets back to this value.
- `cw-max`: cap for binary exponential backoff growth.
- `slots`: number of logical contention slots to simulate.
- `payload-bits`: payload size credited per successful transmission.
- `seed`: deterministic RNG seed used to sample backoff counters.
- `trials`: independent repetitions for sweep commands. Trial seeds start from `seed` and advance deterministically.
- `timing-preset`: named logical timing profile. See [docs/usage-guide.md](docs/usage-guide.md) for the exact values.
- `output`: raw trial CSV path. Sweep commands also write a sibling `-summary.csv` file automatically.

## How To Read The Outputs

The simulator produces several output layers:

- raw CSV: one row per class per trial.
- summary CSV: mean, standard deviation, and 95% confidence interval aggregates for each scenario point.
- PNG plots: sweep and mixed-class figures generated from the summary CSVs.
- markdown report: table-oriented report generated from the summary CSVs.
- JSON trace: slot-by-slot execution history for TUI replay.

The most important metrics are:

- average delay: average packet age in slots at the moment of successful delivery. This is success-conditioned.
- throughput: delivered payload bits per simulated slot.
- Jain fairness index: `1.0` is perfectly equal, values near `0.0` indicate strong capture or starvation.
- per-station throughput variance: larger values mean more inequality across stations.
- zero-success station fraction: share of stations that delivered no packets at all.
- max-station throughput share: share of total throughput captured by the dominant station.

Interpret the aggregate metrics together. A scenario can show strong throughput and low success-conditioned delay while still being pathological on fairness, starvation, and capture.

## Timing Presets

- `baseline`: `difs=1`, `sifs=0`, `tx=1`, `collision-penalty=4`
- `short-defer`: `difs=0`, `sifs=0`, `tx=1`, `collision-penalty=4`
- `long-transmission`: `difs=1`, `sifs=1`, `tx=3`, `collision-penalty=6`

These are logical slot counts, not direct microsecond claims.

## Project Layout

- `src/app/cli.rs`: CLI command surface.
- `src/app/experiments/mod.rs`: user sweep, CW sweep, and mixed-class orchestration.
- `src/app/output.rs`: raw CSV serialization.
- `src/app/summary.rs`: summary-statistics aggregation and summary CSV handling.
- `src/app/plot.rs`: PNG plot generation.
- `src/app/report.rs`: markdown report generation.
- `src/app/tui.rs`: live demo, replay, and side-by-side comparison.
- `src/domain/`: scenario/config/report types.
- `src/sim/dcf/`: explicit DCF engine, timing, backoff, medium, station, and metrics logic.
- `tests/`: deterministic integration and golden summary regression tests.

## Model Scope

The simulator models one shared collision domain with explicit DIFS-style defer, random backoff, freeze/resume, success handling, collision handling, binary exponential contention-window growth, and CW reset after success.

It does not currently model hidden nodes, RTS/CTS exchange details, NAV, rate adaptation, PHY waveform details, or exact clause-level 802.11 timing.

For the precise modeling statement, see [docs/dcf-model.md](docs/dcf-model.md).
