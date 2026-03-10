use overlay_core::{
    crypto::{kex::X25519StaticSecret, sign::Ed25519SigningKey},
    session::{
        ClientHandshake, HandshakeConfig, ServerHandshake, SessionEventKind, SessionIoActionKind,
        SessionManager, SessionRunnerInput, SessionState,
    },
    transport::{TcpTransport, TransportBufferConfig, TransportPollEvent},
};

#[test]
fn handshake_outcome_can_flow_into_session_manager_runner_boundary() {
    let config = HandshakeConfig::default();
    let client_signing_key = Ed25519SigningKey::from_seed([7_u8; 32]);
    let server_signing_key = Ed25519SigningKey::from_seed([9_u8; 32]);
    let client_ephemeral_secret = X25519StaticSecret::from_bytes([1_u8; 32]);
    let server_ephemeral_secret = X25519StaticSecret::from_bytes([2_u8; 32]);

    let (client_handshake, client_hello) =
        ClientHandshake::start(config, client_signing_key, client_ephemeral_secret);
    let (server_handshake, server_hello) = ServerHandshake::accept(
        config,
        server_signing_key.clone(),
        server_ephemeral_secret,
        &client_hello,
    )
    .expect("server should accept the client hello");
    let (client_finish, client_outcome) = client_handshake
        .handle_server_hello(&server_hello)
        .expect("client should accept the server hello");
    let server_outcome = server_handshake
        .handle_client_finish(&client_finish)
        .expect("server should accept the client finish");

    assert_eq!(
        client_outcome.transcript_hash,
        server_outcome.transcript_hash
    );
    assert_eq!(client_outcome.session_keys, server_outcome.session_keys);

    let mut manager = SessionManager::with_node_id(91, client_hello.client_node_id);
    manager
        .begin_open(100, &TcpTransport)
        .expect("session open should enter opening");

    let established = manager
        .handle_runner_input(
            120,
            SessionRunnerInput::HandshakeSucceeded {
                outcome: client_outcome,
            },
        )
        .expect("runner handshake success should establish the session");
    let observed_input = SessionRunnerInput::from_transport_poll_event(
        TransportPollEvent::FrameReceived {
            bytes: vec![1_u8; 96],
        },
        TransportBufferConfig {
            max_buffer_bytes: 96,
        },
    )
    .expect("bounded frame should translate into runner input")
    .expect("frame event should produce runner input");
    let observed = manager
        .handle_runner_input(140, observed_input)
        .expect("runner frame delivery should refresh the session");
    let closed_input = SessionRunnerInput::from_transport_poll_event(
        TransportPollEvent::Closed,
        TransportBufferConfig {
            max_buffer_bytes: 96,
        },
    )
    .expect("closed event should translate into runner input")
    .expect("closed event should produce runner input");
    let closed = manager
        .handle_runner_input(180, closed_input)
        .expect("runner close should close the session");

    assert_eq!(established.event, SessionEventKind::OpenSucceeded);
    assert_eq!(observed.event, SessionEventKind::ActivityObserved);
    assert_eq!(closed.event, SessionEventKind::Closed);
    assert_eq!(manager.state(), SessionState::Closed);
    assert_eq!(
        manager
            .drain_io_actions()
            .into_iter()
            .map(|action| action.action)
            .collect::<Vec<_>>(),
        vec![SessionIoActionKind::BeginHandshake]
    );
}
