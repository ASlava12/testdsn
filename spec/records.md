# Records

## NodeRecord
Fields:
- version
- node_id
- node_public_key
- created_at_unix_s
- flags
- supported_transports
- supported_kex
- supported_signatures
- anti_sybil_proof
- signature

Validation:
- `node_id == BLAKE3-256(node_public_key)`
- signature valid over canonical serialized body
- `supported_transports`, `supported_kex`, and `supported_signatures` must use
  allowed lowercase string enums
- those arrays must be deduplicated and sorted lexicographically before
  signing or hashing

## PresenceRecord
Fields:
- version
- node_id
- epoch
- expires_at_unix_s
- sequence
- transport_classes
- reachability_mode
- locator_commitment
- encrypted_contact_blobs[]
- relay_hint_refs[]
- intro_policy
- capability_requirements
- signature

Conflict resolution:
1. reject invalid signature
2. reject expired record as fresh result
3. higher epoch wins
4. same epoch: higher sequence wins
5. same epoch + same sequence: byte-identical only

## ServiceRecord
Fields:
- version
- node_id
- app_id
- service_name
- service_version
- auth_mode
- policy
- reachability_ref
- metadata_commitment
- signature

## RelayHint
Fields:
- relay_node_id
- relay_transport_class
- relay_score
- relay_policy
- expiry

## IntroTicket
Fields:
- ticket_id
- target_node_id
- requester_binding
- scope
- issued_at_unix_s
- expires_at_unix_s
- nonce
- signature
