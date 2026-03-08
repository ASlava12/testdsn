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

## Milestone 2 conservative choices

- For handshake transcript hashing, Milestone 2 currently uses `BLAKE3(label_len || label || body_len || body)` over ordered entries, with `u32` big-endian lengths and deterministic JSON body bytes.
- The server signature covers `ClientHello` plus the unsigned `ServerHello`.
- The hello transcript used for session key derivation covers `ClientHello` plus the signed `ServerHello`.
- The client finish signature covers `ClientHello`, the signed `ServerHello`, and the unsigned `ClientFinish`.
- The current key schedule uses `HKDF-SHA256(salt = hello_transcript_hash, ikm = x25519_shared_secret)` with info labels:
- `overlay-mvp/client-to-server`
- `overlay-mvp/server-to-client`
- `overlay-mvp/client-finish-key`
- `overlay-mvp/client-finish-nonce`
- `ClientFinish` confirmation currently encrypts the hello transcript hash with ChaCha20-Poly1305 using AAD `overlay-mvp-client-finish`.
