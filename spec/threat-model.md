# Threat Model

## Adversaries
- passive observer
- active crawler
- censor
- sybil adversary
- malicious relay / neighbor
- unstable environment with churn and NAT

## MVP protection goals
- survive partial bootstrap blocking
- survive partial transport blocking
- reduce easy enumeration
- mitigate local eclipse pressure
- tolerate churn and relay loss

## Non-goals in MVP
- strong anonymity like Tor
- global passive adversary resistance
- perfect anti-Sybil guarantees
