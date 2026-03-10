use overlay_core::{
    crypto::sign::Ed25519SigningKey,
    identity::derive_node_id,
    records::{IntroTicket, PresenceRecord, RelayHint},
    relay::{
        build_reachability_plan, IntroResponseStatus, RelayConfig, RelayManager, ResolveIntro,
    },
    REPOSITORY_STAGE,
};

#[test]
fn relay_fallback_plan_tracks_current_stage_boundary() {
    let now_unix_s = 1_700_000_000;
    let requester_binding = b"client-binding";
    let target_signing_key = Ed25519SigningKey::from_seed([77_u8; 32]);
    let target_node_id = derive_node_id(target_signing_key.public_key().as_bytes());
    let target_presence = sample_presence_record(target_node_id, now_unix_s + 600);
    let verified_ticket =
        sample_intro_ticket(&target_signing_key, requester_binding, now_unix_s + 300);
    let relay_hints = vec![
        RelayHint {
            relay_node_id: overlay_core::identity::NodeId::from_bytes([11_u8; 32]),
            relay_transport_class: "tcp".to_string(),
            relay_score: 40,
            relay_policy: vec![1_u8],
            expiry: now_unix_s + 600,
        },
        RelayHint {
            relay_node_id: overlay_core::identity::NodeId::from_bytes([12_u8; 32]),
            relay_transport_class: "quic".to_string(),
            relay_score: 90,
            relay_policy: vec![2_u8],
            expiry: now_unix_s + 600,
        },
    ];
    let mut relay_manager = RelayManager::new(RelayConfig::default().with_relay_mode(true))
        .expect("relay config should be valid");

    let plan = build_reachability_plan(
        &target_presence,
        &relay_hints,
        &verified_ticket,
        requester_binding,
        now_unix_s,
    )
    .expect("relay fallback plan should be created");

    assert_eq!(REPOSITORY_STAGE, "milestone-14-launch-gate");
    assert_eq!(
        plan.direct_attempts,
        vec![
            overlay_core::transport::TransportClass::Quic,
            overlay_core::transport::TransportClass::Tcp
        ]
    );
    assert_eq!(plan.relay_fallbacks.len(), 2);
    assert_eq!(
        plan.relay_fallbacks[0].relay_node_id,
        relay_hints[1].relay_node_id
    );

    let resolve_intro = ResolveIntro {
        relay_node_id: plan.relay_fallbacks[0].relay_node_id,
        intro_ticket: verified_ticket.into_inner(),
    }
    .verify_with_public_key(&target_signing_key.public_key())
    .expect("resolve intro request should verify before relay handling");
    let intro_response = relay_manager.process_resolve_intro(
        plan.relay_fallbacks[0].relay_node_id,
        resolve_intro,
        requester_binding,
        now_unix_s,
    );

    assert_eq!(intro_response.status, IntroResponseStatus::Forwarded);
    let tunnel = relay_manager
        .bind_tunnel(
            7,
            plan.relay_fallbacks[0].relay_node_id,
            target_node_id,
            now_unix_s,
        )
        .expect("fallback relay bind should fit quota");
    relay_manager
        .note_relayed_bytes(tunnel.relay_node_id, 1024, now_unix_s)
        .expect("relay byte accounting should fit quota");

    assert_eq!(tunnel.target_node_id, target_node_id);
    assert_eq!(relay_manager.active_tunnel_count(), 1);
}

fn sample_presence_record(
    node_id: overlay_core::identity::NodeId,
    expires_at_unix_s: u64,
) -> PresenceRecord {
    PresenceRecord {
        version: 1,
        node_id,
        epoch: 9,
        expires_at_unix_s,
        sequence: 3,
        transport_classes: vec!["quic".to_string(), "relay".to_string(), "tcp".to_string()],
        reachability_mode: "hybrid".to_string(),
        locator_commitment: vec![1_u8, 2, 3, 4],
        encrypted_contact_blobs: vec![vec![5_u8, 6, 7]],
        relay_hint_refs: Vec::new(),
        intro_policy: "allow".to_string(),
        capability_requirements: vec!["service-host".to_string()],
        signature: vec![8_u8; 64],
    }
}

fn sample_intro_ticket(
    signing_key: &Ed25519SigningKey,
    requester_binding: &[u8],
    expires_at_unix_s: u64,
) -> overlay_core::records::VerifiedIntroTicket {
    let mut ticket = IntroTicket {
        ticket_id: vec![1_u8, 2, 3, 4],
        target_node_id: derive_node_id(signing_key.public_key().as_bytes()),
        requester_binding: requester_binding.to_vec(),
        scope: "relay-intro".to_string(),
        issued_at_unix_s: 1_700_000_000,
        expires_at_unix_s,
        nonce: vec![9_u8, 8, 7, 6],
        signature: Vec::new(),
    };
    let body = ticket
        .canonical_body_bytes()
        .expect("intro ticket body should serialize");
    ticket.signature = signing_key.sign(&body).as_bytes().to_vec();
    ticket
        .verify_with_public_key(&signing_key.public_key())
        .expect("intro ticket should verify")
}
