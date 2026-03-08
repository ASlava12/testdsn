# Architecture

## Goal

Build a censorship-resistant overlay network that balances:
- partial node privacy;
- exact reachability by `node_id`;
- latency-aware path selection;
- service access via `app_id`.

## Core principle

The network should answer well:
> find this exact node

and answer poorly:
> list all nodes and their real endpoints

## Layers

1. Identity
2. Bootstrap
3. Presence/rendezvous
4. Transport/session
5. Routing/path
6. Service

## IDs

```text
node_id = BLAKE3-256(node_public_key)
app_id  = BLAKE3-256(node_id || app_namespace || app_name)
```

## Topology

Hybrid small-world overlay with:
- stable neighbors
- random long-links
- latency-good neighbors
- relay-capable neighbors
