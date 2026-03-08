# Routing

Lookup resolves reachability.
Routing chooses a good path.

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
