# Relay Layer

## Goal

Provide fallback connectivity when direct transport is unavailable.

## Relay modes
- forward relay
- intro relay
- rendezvous relay
- bridge relay

## Current Milestone 6 baseline
- use a fresh verified `IntroTicket` before relay introduction;
- prefer direct transport attempts first, then relay fallback candidates;
- keep secondary relay candidates instead of one mandatory relay;
- enforce local relay quotas conservatively.

## Rules
- do not rely on one relay
- keep secondary relay candidates
- enforce local quotas
- reject expired relay hints or intro tickets as fresh fallback inputs
