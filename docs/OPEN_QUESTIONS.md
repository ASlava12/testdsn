# Open Questions

This file exists so Codex does not silently invent protocol details.

## Currently underspecified areas

1. Canonical binary encoding for all records and messages.
2. Exact transcript hash layout for the handshake.
3. Full key schedule details beyond MVP wrappers.
4. Exact relay quota defaults for tiny/std/relay profiles.
5. Concrete constants for path score weights and hysteresis.
6. Concrete bootstrap response schema beyond the current skeleton.
7. Final encoding of transport classes and capabilities.

## Rule

If a task requires one of these areas and no user instruction resolves it:
- choose the smallest conservative implementation;
- document the choice in the final report;
- update this file if a new gap is discovered.
