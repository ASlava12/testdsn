use overlay_core::{
    crypto::sign::Ed25519SigningKey,
    identity::{derive_app_id, derive_node_id},
    records::ServiceRecord,
    service::{
        GetServiceRecord, LocalServicePolicy, OpenAppSession, OpenAppSessionStatus, ServiceConfig,
        ServiceRecordResponseStatus, ServiceRegistry,
    },
    REPOSITORY_STAGE,
};

#[test]
fn service_open_flow_tracks_current_stage_boundary() {
    let now_unix_ms = 1_700_000_000_123;
    let signing_key = Ed25519SigningKey::from_seed([55_u8; 32]);
    let record = sample_signed_service_record(&signing_key, "terminal");
    let app_id = record.app_id;
    let mut registry =
        ServiceRegistry::new(ServiceConfig::default()).expect("default config should be valid");

    registry
        .register_verified(
            record
                .clone()
                .verify_with_public_key(&signing_key.public_key())
                .expect("integration service record should verify before registry handoff"),
            LocalServicePolicy::allow_all(),
        )
        .expect("service registration should succeed");

    let response = registry.resolve(GetServiceRecord { app_id });
    let opened = registry.open_app_session(
        OpenAppSession {
            app_id,
            reachability_ref: record.reachability_ref.clone(),
        },
        now_unix_ms,
    );

    assert_eq!(REPOSITORY_STAGE, "milestone-9-hardening");
    assert_eq!(response.status, ServiceRecordResponseStatus::Found);
    assert_eq!(response.record, Some(record.clone()));
    assert_eq!(opened.status, OpenAppSessionStatus::Opened);
    assert_eq!(registry.registered_service_count(), 1);
    assert_eq!(registry.open_session_count(), 1);
    assert_eq!(
        registry
            .session(
                opened
                    .session_id
                    .expect("opened service session should have id")
            )
            .expect("opened service session should be stored")
            .node_id,
        record.node_id
    );
}

fn sample_signed_service_record(
    signing_key: &Ed25519SigningKey,
    service_name: &str,
) -> ServiceRecord {
    let node_id = derive_node_id(signing_key.public_key().as_bytes());
    let mut record = ServiceRecord {
        version: 1,
        node_id,
        app_id: derive_app_id(&node_id, "chat", service_name),
        service_name: service_name.to_string(),
        service_version: "1.0.0".to_string(),
        auth_mode: "none".to_string(),
        policy: vec![1, 2, 3, 4],
        reachability_ref: vec![0xAA, 0xBB],
        metadata_commitment: vec![0xCC, 0xDD],
        signature: Vec::new(),
    };
    let body = record
        .canonical_body_bytes()
        .expect("integration service body should serialize");
    record.signature = signing_key.sign(&body).as_bytes().to_vec();
    record
}
