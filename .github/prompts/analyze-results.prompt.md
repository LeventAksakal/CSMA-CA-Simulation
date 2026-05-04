---
mode: ask
description: "Use when a sweep output, fairness trend, throughput curve, or delay result looks suspicious and needs technical interpretation."
---

Analyze the current CSMA/CA experiment outputs.

Focus on:

- whether the observed trend matches the current slot model,
- whether lower-CWmin users gained throughput at the cost of fairness,
- whether the result could be explained by collisions, freezing, or sampling noise,
- whether the anomaly indicates a likely simulator bug or just a parameter-regime effect.

If the result looks wrong, identify the most likely code locations to inspect next.
