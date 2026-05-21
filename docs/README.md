# Documentation

This directory holds the repository's human-facing documentation.

## Start Here

- [../README.md](../README.md): top-level overview, quickstart, and common commands.
- [presentation.md](presentation.md): speaker-ready 10-minute presentation notes with linked plots, code references, and a live demo script.
- [usage-guide.md](usage-guide.md): complete CLI usage, configuration meanings, outputs, and interpretation guidance.
- [dcf-model.md](dcf-model.md): what the simulator models, how the slot-based DCF engine works, and what is intentionally out of scope.
- [reference/IEEE-80211-2024.txt](reference/IEEE-80211-2024.txt): local behavior specification derived from public CSMA/CA and DCF descriptions.

## Recommended Reading Order

1. Read the top-level README to get the workflow.
2. Read [usage-guide.md](usage-guide.md) to understand command selection, flags, outputs, and plots.
3. Read [dcf-model.md](dcf-model.md) before making model changes or interpreting surprising fairness results.
4. Use [reference/IEEE-80211-2024.txt](reference/IEEE-80211-2024.txt) as the repository-local behavior target when discussing CSMA/CA semantics.
