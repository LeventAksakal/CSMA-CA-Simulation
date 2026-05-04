# CSMA-CA Simulation

Rust-first simulator for a CSMA/CA coursework project. The repository is set up for CLI-driven development, reproducible experiment runs, CSV result export, and agent-assisted iteration inside VS Code.

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

## Project Layout

- `src/config.rs`: simulation and experiment configuration types.
- `src/model.rs`: station state and transmission outcomes.
- `src/simulator.rs`: slot-by-slot CSMA/CA execution.
- `src/metrics.rs`: aggregate and per-class metrics.
- `src/experiments.rs`: parameter sweeps and mixed-class studies.
- `src/output.rs`: CSV serialization.
- `src/cli.rs`: command-line entry points.
- `tests/simulation.rs`: deterministic and behavioral tests.

## Model Scope

This is a slotted CSMA/CA study model inspired by 802.11 DCF rather than a packet-level PHY simulator. The implementation focuses on:

- contention and backoff behavior,
- collision handling and CW expansion,
- delay and throughput trends,
- comparative advantage of a lower-CWmin class.

## Quality Gates

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `pre-commit run --all-files`
