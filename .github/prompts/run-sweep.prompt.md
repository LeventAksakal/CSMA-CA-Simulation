---
mode: ask
description: "Use when adding, modifying, or rerunning a CSMA/CA experiment sweep with reproducible Cargo commands and CSV outputs."
---

Extend or rerun the experiment suite for this repository.

Requirements:

- keep the simulator core in Rust,
- prefer Cargo CLI commands for changes and execution,
- write outputs to a reproducible file path under `results/`,
- describe which parameter range is being changed and why,
- update README examples if the public workflow changes,
- run the relevant formatting, linting, and test commands after changes.
