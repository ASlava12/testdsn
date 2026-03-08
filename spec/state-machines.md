# State Machines

## Session lifecycle
States:
- idle
- opening
- established
- degraded
- closing
- closed

## Presence publish lifecycle
States:
- build_record
- publish_pending
- quorum_reached
- refresh_due
- expired

## Lookup lifecycle
States:
- init
- shortlist_built
- querying
- result_found
- not_found
- budget_exhausted

## Relay introduction lifecycle
States:
- request_ticket
- ticket_received
- intro_attempt
- direct_established_or_relay_bound
- failed
