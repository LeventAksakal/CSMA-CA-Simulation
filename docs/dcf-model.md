# DCF Model

This document describes the CSMA/CA model implemented by the simulator. It is intentionally explicit about what is modeled, how it is simplified, and what the results do and do not mean.

## 1. Modeling Goal

The repository implements a deterministic slot-based study model inspired by IEEE 802.11 Distributed Coordination Function (DCF). The goal is to study contention, backoff, collisions, throughput, delay, and fairness under shared-medium access, while keeping the logic easy to inspect and test.

The canonical local behavior target is [reference/IEEE-80211-2024.txt](reference/IEEE-80211-2024.txt). That file is a repository specification derived from public sources, not the copyrighted IEEE standard text.

## 2. High-Level Structure

At a high level, one simulation run contains:

- one shared collision domain,
- one or more station classes,
- a timing profile,
- a contention-window cap,
- a deterministic RNG seed,
- a fixed number of logical slots.

Relevant code structure:

- `src/domain/scenario.rs`: scenario, timing, and class definitions.
- `src/domain/config.rs`: user-facing configuration and timing presets.
- `src/sim/dcf/engine.rs`: slot-by-slot DCF execution loop.
- `src/sim/dcf/station.rs`: per-station runtime state and transitions.
- `src/sim/dcf/timing.rs`: logical defer and busy-slot timing model.
- `src/sim/dcf/metrics.rs`: aggregate and fairness metric derivation.

## 3. Traffic Assumption

Stations are modeled as saturated sources.

That means each station always has another packet available after it completes a transmission attempt. After a success, the station immediately starts waiting for the medium again with a fresh packet age counter. This is useful for studying contention behavior, but it is different from bursty or finite offered-load traffic.

Implication: throughput and delay trends describe a saturated contention regime, not an application-layer workload trace.

## 4. Medium Model

The simulator uses one shared medium with two top-level states:

- idle,
- busy for a remaining number of slots.

When the medium is busy, stations cannot continue contention countdown. The busy period lasts for a configured number of logical slots after either a success or a collision.

There is no hidden-node topology in the baseline model. All stations observe the same medium state.

## 5. Station State Machine

Each station moves through explicit phases such as:

- waiting for medium,
- defer,
- backoff countdown,
- transmitting,
- collision recovery,
- awaiting result.

The important behavior is:

1. A station waiting for the medium enters defer once the medium is idle.
2. After defer is satisfied, the station resumes or starts backoff countdown.
3. A station whose backoff counter reaches zero attempts transmission.
4. On success, it records delivery and resets CW to CWmin.
5. On collision, it increments collision attempts and grows CW using binary exponential backoff.
6. When the medium becomes busy while a station is counting down, the station freezes its counter and resumes it later instead of drawing a new one.

This freeze/resume behavior is one of the core DCF properties the simulator is designed to preserve.

## 6. Backoff And Contention Window Rule

Each station owns:

- `cw_min`: class-specific initial CW,
- `current_cw`: current contention window,
- `cw_max`: shared maximum CW cap.

For a transmission attempt, the station samples a backoff counter from the inclusive range:

`0 .. current_cw`

After a success:

- the packet counts as delivered,
- delay for that packet is recorded,
- `current_cw` resets to `cw_min`.

After a collision:

- the station increments its collision counter,
- `current_cw` grows using binary exponential backoff,
- the new CW is capped by `cw_max`.

The growth rule in this repository is the standard explicit form used by the codebase:

`current_cw = min(cw_max, 2 * current_cw + 1)`

## 7. Timing Abstraction

The simulator does not model microsecond-accurate PHY timing. Instead it uses configurable logical slot counts:

- `difs_slots`: idle slots that must elapse before contention countdown can proceed,
- `sifs_slots`: additional busy slots applied after a successful transmission,
- `tx_duration_slots`: logical payload transmission duration,
- `collision_penalty_slots`: additional busy slots applied after a collision.

The effective busy durations are:

- success busy duration: `tx_duration_slots - 1 + sifs_slots`
- collision busy duration: `tx_duration_slots - 1 + collision_penalty_slots`

These parameters are deliberately simple. They let the study vary defer length and busy cost without introducing a full MAC+PHY timing model.

## 8. Success And Collision Resolution

On each eligible idle contention slot, the engine finds all stations whose backoff counter is zero.

- If no stations are ready, the slot is idle and countdown advances.
- If exactly one station is ready, the transmission succeeds.
- If two or more stations are ready, the event is treated as a collision.

In a real 802.11 system, failure is often inferred through missing ACK reception. In this simulator, simultaneous transmitters are resolved directly as a collision event.

## 9. Delay Semantics

Packet delay is tracked as packet age in simulated slots.

Important detail: average delay is computed only over successful packets. If a station is permanently starved and never succeeds, that station does not directly increase the average delay value. Instead, starvation shows up through the fairness metrics.

This is why the repository now reports fairness, per-station variance, zero-success fraction, and max-station share alongside throughput and delay.

## 10. Fairness And Capture Metrics

The simulator derives fairness from per-station throughputs, not from per-class averages.

Reported aggregate fairness metrics:

- Jain fairness index,
- per-station throughput variance,
- zero-success station fraction,
- max-station throughput share.

These metrics exist because aggregate throughput alone can hide pathological behavior. A low-CW configuration may deliver a lot of payload while starving most of the stations. In that case:

- throughput can still look strong,
- average delay for successful packets can still look small,
- fairness collapses,
- zero-success fraction rises,
- max-station share approaches one.

## 11. What The Model Captures Well

This simulator is well-suited for studying:

- relative contention pressure as user count rises,
- the effect of CWmin on backoff aggressiveness,
- recovery from collisions through CW growth,
- comparative advantage of a lower-CW class,
- fairness collapse and capture under aggressive settings,
- deterministic, seed-reproducible scenario comparisons.

## 12. What The Model Does Not Yet Claim

This baseline does not claim to model:

- exact IEEE clause-level timing or compliance,
- hidden terminals or multi-hop topology,
- RTS/CTS handshakes,
- NAV state,
- retransmission limits,
- channel errors separate from collisions,
- rate adaptation,
- frame aggregation or block ACK,
- finite queues or unsaturated offered load,
- detailed ACK timeout modeling,
- PHY capture effects beyond the abstract collision rule.

That means results should be interpreted as a DCF study model, not as a complete WLAN performance predictor.

## 13. Why Very Small CWmin Needs Careful Interpretation

Small CWmin values make stations more aggressive. In this simulator, that can create a regime where one or a few stations repeatedly win the channel. Aggregate throughput may remain high because someone is transmitting successfully, but fairness can become extremely poor.

The correct interpretation is not "small CWmin is good" or "small CWmin is bad" in isolation. The correct interpretation is:

- small CWmin can improve raw access aggressiveness,
- the same setting can severely damage fairness and starve many stations,
- you must read throughput, delay, fairness, zero-success fraction, and max-station share together.

## 14. Validation Strategy

The repository validates the model through:

- unit tests for backoff, medium, timing, station behavior, and resolver logic,
- integration tests for deterministic seeded runs, DIFS gating, collision recovery, and mixed-class advantage,
- golden summary tests for the experiment/reporting pipeline.

Recommended validation commands:

```powershell
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
pre-commit run --all-files
```

## 15. When To Extend The Model

Extend the model when the research question requires behavior that the current assumptions cannot represent. Common examples:

- finite offered-load traffic instead of saturation,
- richer failure timing such as ACK timeout or EIFS-like recovery,
- retransmission limits,
- topology effects such as hidden nodes,
- control-frame exchanges such as RTS/CTS.

When making such changes, update both this file and [usage-guide.md](usage-guide.md) so the repository documentation stays aligned with the implemented behavior.
