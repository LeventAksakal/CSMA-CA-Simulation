# CSMA/CA Simulator Instructions

## Project Definition

_Write a simulator for the CSMA/CA (Carrier Sense Multiple Access with Collision Avoidance) protocol. Plot average delay and throughput graphs by varying
number of users and minimum contention window size. Also, group the users
into two classes, where one class starts from a lower minimum contention window size. Show that that class becomes advantageous in terms of average delay
and throughput._

## Agent Instructions

Use Rust as the primary implementation language for simulator logic, experiments, and CLI entry points. Keep the model explicit and inspectable rather than hiding behavior behind framework abstractions.

For feature work and non-trivial changes:

- use `codebase-memory` first to gather repository context before editing,
- use `codebase-memory` to calculate blast radius and identify impacted modules, symbols, tests, and outward-facing behavior before implementing changes,
- use `codebase-memory` again after edits when needed to verify impacted areas and follow-on changes,
- use `Context7` for the latest library, framework, SDK, CLI, and tool documentation instead of relying on stale knowledge.

After each coherent unit of work:

- write a conventional commitizen-style commit message,
- use lowercase text for the subject and body,
- use `-` bullets in the body when a body is needed,
- prefer the format `<type>(<scope>): <summary>`, for example `feat(dcf): implement cw growth`.

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
