use overlay_core::{
    crypto::sign::Ed25519SigningKey,
    records::PresenceRecord,
    rendezvous::{
        LookupNode, LookupResponse, PublishDisposition, PublishPresence, RendezvousConfig,
        RendezvousStore,
    },
    REPOSITORY_STAGE,
};

#[test]
fn publish_lookup_flow_tracks_current_stage_boundary() {
    let now_unix_s = 1_700_000_000;
    let signing_key = Ed25519SigningKey::from_seed([42_u8; 32]);
    let record = sample_signed_presence_record(&signing_key, 9, 3, now_unix_s + 600);
    let node_id = record.node_id;
    let mut store = RendezvousStore::new(RendezvousConfig::default())
        .expect("default rendezvous config should be valid");
    let verified_publish = PublishPresence {
        record: record.clone(),
    }
    .verify_with_public_key(&signing_key.public_key())
    .expect("integration publish should verify before store handoff");

    let ack = store
        .publish_verified(verified_publish, now_unix_s)
        .expect("presence publish should succeed");
    assert_eq!(ack.disposition, PublishDisposition::Stored);

    let mut state = store
        .lookup_state(4)
        .expect("lookup state should be created");
    let response = store.lookup(LookupNode { node_id }, now_unix_s + 1, &mut state);

    assert_eq!(REPOSITORY_STAGE, "milestone-17-operator-runtime");
    match response {
        LookupResponse::Result(result) => {
            assert_eq!(result.node_id, node_id);
            assert_eq!(result.record, record);
            assert_eq!(result.remaining_budget, 3);
        }
        LookupResponse::NotFound(not_found) => {
            panic!("expected fresh record lookup to succeed, got {not_found:?}")
        }
    }
}

fn sample_signed_presence_record(
    signing_key: &Ed25519SigningKey,
    epoch: u64,
    sequence: u64,
    expires_at_unix_s: u64,
) -> PresenceRecord {
    let mut record = PresenceRecord {
        version: 1,
        node_id: overlay_core::identity::derive_node_id(signing_key.public_key().as_bytes()),
        epoch,
        expires_at_unix_s,
        sequence,
        transport_classes: vec!["quic".to_string()],
        reachability_mode: "direct".to_string(),
        locator_commitment: vec![1_u8, 2, 3, 4],
        encrypted_contact_blobs: vec![vec![5_u8, 6, 7]],
        relay_hint_refs: Vec::new(),
        intro_policy: "allow".to_string(),
        capability_requirements: vec!["service-host".to_string()],
        signature: Vec::new(),
    };
    let body = record
        .canonical_body_bytes()
        .expect("integration presence body should serialize");
    record.signature = signing_key.sign(&body).as_bytes().to_vec();
    record
}
