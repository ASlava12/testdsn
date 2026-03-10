use overlay_core::{
    bootstrap::{
        BootstrapNetworkParams, BootstrapPeer, BootstrapPeerRole, BootstrapProvider,
        BootstrapResponse, StaticBootstrapProvider, BOOTSTRAP_SCHEMA_VERSION,
    },
    identity::NodeId,
    metrics::{LogContext, Observability},
    peer::{PeerStore, PeerStoreConfig},
    session::HANDSHAKE_VERSION,
    wire::MAX_FRAME_BODY_LEN,
    REPOSITORY_STAGE,
};

#[test]
fn bootstrap_smoke_tracks_current_stage_boundary() {
    let provider = StaticBootstrapProvider::new(sample_response());
    let node_id = NodeId::from_bytes([99_u8; 32]);
    let mut observability = Observability::default();
    let response = provider
        .fetch_validated_response_with_observability(
            1_700_000_100,
            &mut observability,
            LogContext {
                timestamp_unix_ms: 1_700_000_100_000,
                node_id,
                correlation_id: 71,
            },
        )
        .expect("bootstrap response should validate");
    let mut store = PeerStore::new(PeerStoreConfig {
        max_neighbors: 3,
        max_relay_neighbors: 1,
        max_neighbors_per_transport: 1,
    })
    .expect("peer store config should be valid");
    let active = store
        .ingest_bootstrap_response_with_observability(
            response,
            1_700_000_100,
            &mut observability,
            LogContext {
                timestamp_unix_ms: 1_700_000_100_100,
                node_id,
                correlation_id: 72,
            },
        )
        .expect("validated bootstrap response should seed the peer store");

    assert_eq!(REPOSITORY_STAGE, "milestone-14-launch-gate");
    assert_eq!(active.len(), 3);
    assert_eq!(store.active_neighbors().count(), 3);
    assert_eq!(observability.metrics().active_peers, 3);
    assert_eq!(observability.logs().len(), 2);
    assert_eq!(observability.logs()[0].event, "bootstrap_fetch");
    assert_eq!(observability.logs()[0].result, "accepted");
    assert_eq!(observability.logs()[1].event, "bootstrap_ingest");
    assert_eq!(observability.logs()[1].result, "accepted");
    assert_eq!(
        store
            .active_neighbors()
            .filter(|neighbor| neighbor.is_relay_capable())
            .count(),
        1
    );
    assert!(store
        .active_neighbors()
        .map(|neighbor| neighbor.selected_transport_class.as_deref())
        .collect::<Vec<_>>()
        .contains(&Some("relay")));
}

fn sample_response() -> BootstrapResponse {
    BootstrapResponse {
        version: BOOTSTRAP_SCHEMA_VERSION,
        generated_at_unix_s: 1_700_000_000,
        expires_at_unix_s: 1_700_000_900,
        network_params: BootstrapNetworkParams {
            network_id: "overlay-mvp".to_string(),
        },
        epoch_duration_s: 900,
        presence_ttl_s: 1_800,
        max_frame_body_len: MAX_FRAME_BODY_LEN,
        handshake_version: HANDSHAKE_VERSION,
        peers: vec![
            peer(
                [1_u8; 32],
                &["tcp"],
                &[],
                BootstrapPeerRole::Standard,
                &["tcp://node-a"],
            ),
            peer(
                [2_u8; 32],
                &["quic"],
                &[],
                BootstrapPeerRole::Standard,
                &["quic://node-b"],
            ),
            peer(
                [3_u8; 32],
                &["ws"],
                &[],
                BootstrapPeerRole::Standard,
                &["https://node-c"],
            ),
            peer(
                [4_u8; 32],
                &["relay"],
                &["relay-forward"],
                BootstrapPeerRole::Relay,
                &["relay://node-d"],
            ),
        ],
        bridge_hints: Vec::new(),
    }
}

fn peer(
    node_id_bytes: [u8; 32],
    transport_classes: &[&str],
    capabilities: &[&str],
    observed_role: BootstrapPeerRole,
    dial_hints: &[&str],
) -> BootstrapPeer {
    BootstrapPeer {
        node_id: NodeId::from_bytes(node_id_bytes),
        transport_classes: transport_classes
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        capabilities: capabilities
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        dial_hints: dial_hints
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        observed_role,
    }
}
