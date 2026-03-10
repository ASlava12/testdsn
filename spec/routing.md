# Routing

Lookup resolves reachability.
Routing chooses a good path.

## Current Milestone 7 baseline
- keep path metrics local and deterministic;
- update observed RTT, loss, and jitter with integer EWMA defaults;
- use the conservative integer path-score formula from `docs/OPEN_QUESTIONS.md`;
- require absolute and relative improvement, dwell time, and switch-rate caps
  before switching away from the current path;
- break equal-score ties deterministically by lower `path_id`.

## Path metrics
- estimated RTT
- observed RTT
- loss
- jitter
- stability
- relay cost
- diversity bonus
- censorship risk

## Switching rules
- use EWMA for RTT/loss
- require minimum improvement threshold
- require minimum dwell time on current path
- cap switch frequency
