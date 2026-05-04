# CSMA/CA Simulator Instructions

Use Rust as the primary implementation language for simulator logic, experiments, and CLI entry points. Keep the model explicit and inspectable rather than hiding behavior behind framework abstractions.

Prefer CLI-first workflows:

- use Cargo commands for dependency management and project tasks,
- keep experiment outputs reproducible from documented commands,
- export result files instead of embedding analysis-only state.

When editing the simulator:

- preserve deterministic seeded execution,
- keep contention-window and collision rules easy to audit,
- treat lower-CWmin versus higher-CWmin class comparisons as a first-class project goal,
- avoid introducing plotting or reporting logic into the simulator core.

Validation expectations:

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `pre-commit run --all-files`
