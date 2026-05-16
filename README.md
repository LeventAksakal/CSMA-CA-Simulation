# CSMA-CA Simulation

Rust-first simulator for a CSMA/CA coursework project. The repository now covers an explicit slot-based DCF study model, reproducible experiment sweeps, CSV export, PNG plotting, and end-to-end behavioral tests around the core medium-access rules.

## Bootstrap

Initialize or refresh the local toolchain with CLI commands:

```powershell
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
pip install pre-commit
pre-commit install --install-hooks --overwrite
pre-commit run --all-files
```

## Commands

Run a single scenario and print a summary:

```powershell
cargo run -- single --users 20 --cw-min 16 --slots 20000 --seed 7
```

Sweep the number of users and write per-class records to CSV:

```powershell
cargo run -- sweep-users --min-users 10 --max-users 50 --step 10 --cw-min 16 --trials 5 --output results/users.csv
```

Sweep the minimum contention window:

```powershell
cargo run -- sweep-cw --users 20 --min-cw 8 --max-cw 64 --step 8 --trials 5 --output results/cw.csv
```

Run the mixed-class scenario where one class starts with a lower CWmin:

```powershell
cargo run -- mixed-classes --lower-users 10 --higher-users 10 --lower-cw-min 8 --higher-cw-min 32 --trials 10 --output results/mixed.csv
```

Render plots from previously generated CSV files:

```powershell
cargo run -- plot --users-input results/users.csv --cw-input results/cw.csv --mixed-input results/mixed.csv --output-dir results/plots
```

Launch the live terminal demo over real simulator trace output:

```powershell
cargo run -- demo --preset mixed --slots 180 --seed 7 --tick-ms 150
```

Run the full workflow from validated simulator outputs to plots:

```powershell
cargo test
cargo run -- sweep-users --min-users 10 --max-users 50 --step 10 --cw-min 16 --trials 10 --output results/users.csv
cargo run -- sweep-cw --users 20 --min-cw 8 --max-cw 64 --step 8 --trials 10 --output results/cw.csv
cargo run -- mixed-classes --lower-users 10 --higher-users 10 --lower-cw-min 8 --higher-cw-min 32 --trials 10 --output results/mixed.csv
cargo run -- plot --users-input results/users.csv --cw-input results/cw.csv --mixed-input results/mixed.csv --output-dir results/plots
```

## Project Layout

- `src/app/cli.rs`: CLI entry points for single runs, sweeps, mixed-class studies, and plotting.
- `src/app/experiments/mod.rs`: parameter sweeps and mixed-class orchestration.
- `src/app/output.rs`: CSV serialization format shared by experiment export and plotting.
- `src/app/plot.rs`: CSV aggregation and PNG chart rendering.
- `src/app/tui.rs`: live terminal demo that replays per-slot simulator traces.
- `src/domain/config.rs`: user-facing simulation configuration types.
- `src/domain/scenario.rs`: explicit scenario, class, timing, and contention-window inputs.
- `src/domain/report.rs`: aggregate and per-class report types.
- `src/sim/runner.rs`: simulator entry points for config-based and scenario-based execution.
- `src/sim/dcf/`: explicit DCF engine, backoff, timing, medium, station state, and metrics logic.
- `tests/simulation.rs`: deterministic and behavioral tests.
- `docs/reference/IEEE-80211-2024.txt`: plain-text repository CSMA/CA behavior specification derived from public sources, with the official IEEE 802.11-2024 standard identified as the normative standard family reference.

## Model Scope

This is a slotted CSMA/CA study model inspired by 802.11 DCF rather than a packet-level PHY simulator. The implementation focuses on:

- contention and backoff behavior,
- collision handling and CW expansion,
- delay and throughput trends,
- comparative advantage of a lower-CWmin class.

The repository behavior spec is tracked at `docs/reference/IEEE-80211-2024.txt`. It is a practical derived spec for implementation work, not a verbatim copy of the IEEE standard.

## DCF Coverage

The implemented simulator behavior is intentionally explicit and audit-friendly. The current baseline includes:

- physical carrier sensing over one shared collision domain,
- DIFS-style defer before contention,
- random backoff with freeze/resume while the medium is busy,
- binary exponential contention-window growth after collision,
- CW reset after success,
- per-slot trace capture for replayable live demos,
- deterministic seeded execution,
- aggregate and per-class delay/throughput reporting.

The repository does not claim exact IEEE clause-level conformance. It is a validated CSMA/CA study model aligned to the local behavior spec in `docs/reference/IEEE-80211-2024.txt`.

## Output Artifacts

The sweep commands emit per-trial CSV records. The plot command consumes those CSVs and writes three PNG artifacts:

- `results/plots/users.png`: average delay and throughput versus user count.
- `results/plots/cw.png`: average delay and throughput versus CWmin.
- `results/plots/mixed.png`: average delay and throughput comparison for the lower-CW and higher-CW classes.

The demo command does not replace the batch workflow. It runs the real simulator, records a deterministic per-slot trace, and replays it in a TUI with:

- a live station table showing phase, backoff, frozen counter, CW, and per-station outcomes,
- aggregate and per-class summaries,
- recent event narration for idle slots, success, collision, and busy periods,
- pause, step, restart, and playback-speed controls.

Demo presets:

- `single`: one station showing defer and clean success behavior.
- `collision`: high-contention standard class run showing collisions and recovery.
- `mixed`: lower-CW versus higher-CW classes for a live fairness demo.

Demo controls:

- `space`: pause or resume playback.
- `n`: advance one slot while paused.
- `f`: speed up playback.
- `s`: slow down playback.
- `r`: restart the same trace.
- `q`: quit.

With the default commands above, the expected trend is:

- delay increases as user count grows,
- throughput decreases as CWmin grows,
- the lower-CW class has lower delay and higher throughput than the higher-CW class.

## Validation Notes

The repository includes both unit coverage for DCF subcomponents and integration coverage for end-to-end simulator behavior, including deterministic replay, DIFS gating, collision recovery via CW growth, and mixed-class advantage.

## Quality Gates

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `pre-commit run --all-files`
