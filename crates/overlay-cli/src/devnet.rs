use std::{
    io::{self, Write},
    path::Path,
};

use overlay_core::{
    crypto::{kex::X25519StaticSecret, sign::Ed25519SigningKey},
    identity::{derive_app_id, derive_node_id, AppId, NodeId},
    records::{IntroTicket, PresenceRecord, RelayHint, ServiceRecord},
    relay::{build_reachability_plan, IntroResponseStatus, ResolveIntro},
    rendezvous::{LookupNode, LookupResponse, PublishDisposition, PublishPresence},
    routing::{PathMetrics, PathState},
    runtime::{NodeRuntime, NodeRuntimeState},
    service::{
        GetServiceRecord, LocalServicePolicy, OpenAppSession, OpenAppSessionStatus,
        ServiceRecordResponseStatus,
    },
    session::{
        ClientHandshake, HandshakeConfig, ServerHandshake, SessionRunnerInput, SessionState,
    },
    transport::{TcpTransport, TransportClass},
};
use serde_json::json;

const START_UNIX_MS: u64 = 1_700_100_000_000;
const CLIENT_SESSION_ID: u64 = 101;
const SERVER_SESSION_ID: u64 = 201;
const RELAY_TUNNEL_ID: u64 = 7_001;
const PRESENCE_TTL_S: u64 = 600;
const REQUESTER_BINDING: &[u8] = b"devnet-node-a";
const SERVICE_NAMESPACE: &str = "devnet";
const SERVICE_NAME: &str = "terminal";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SmokeOptions {
    pub soak_seconds: u64,
    pub status_interval_seconds: Option<u64>,
}

pub fn run_smoke(devnet_dir: &Path) -> Result<(), String> {
    run_smoke_with_options(devnet_dir, SmokeOptions::default())
}

pub fn run_smoke_with_options(devnet_dir: &Path, options: SmokeOptions) -> Result<(), String> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    run_smoke_with_writer_and_options(devnet_dir, &mut stdout, options).map(|_| ())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SmokeReport {
    node_a_id: NodeId,
    node_b_id: NodeId,
    node_c_id: NodeId,
    relay_node_id: NodeId,
    service_app_id: AppId,
    service_session_id: u64,
    relay_tunnel_id: u64,
}

struct DevnetNode {
    name: &'static str,
    runtime: NodeRuntime,
}

#[cfg(test)]
fn run_smoke_with_writer(devnet_dir: &Path, writer: &mut dyn Write) -> Result<SmokeReport, String> {
    run_smoke_with_writer_and_options(devnet_dir, writer, SmokeOptions::default())
}

fn run_smoke_with_writer_and_options(
    devnet_dir: &Path,
    writer: &mut dyn Write,
    options: SmokeOptions,
) -> Result<SmokeReport, String> {
    let mut node_a = load_node(devnet_dir, "node-a")?;
    let mut node_b = load_node(devnet_dir, "node-b")?;
    let mut node_c = load_node(devnet_dir, "node-c")?;
    let mut relay = load_node(devnet_dir, "node-relay")?;

    startup_node(&mut node_a, START_UNIX_MS, writer)?;
    startup_node(&mut node_b, START_UNIX_MS + 100, writer)?;
    startup_node(&mut node_c, START_UNIX_MS + 200, writer)?;
    startup_node(&mut relay, START_UNIX_MS + 300, writer)?;

    establish_placeholder_session(
        &mut node_a,
        &mut node_b,
        CLIENT_SESSION_ID,
        SERVER_SESSION_ID,
        START_UNIX_MS + 1_000,
        writer,
    )?;

    let presence_record = build_signed_presence_record(
        node_b.runtime.context().signing_key(),
        unix_ms_to_s(START_UNIX_MS) + PRESENCE_TTL_S,
        relay.runtime.context().node_id(),
    )?;
    let verified_presence = PublishPresence {
        record: presence_record.clone(),
    }
    .verify_with_public_key(&node_b.runtime.context().signing_key().public_key())
    .map_err(|error| format!("failed to verify node-b presence record: {error}"))?;

    let publish_ack = node_b
        .runtime
        .context_mut()
        .rendezvous_mut()
        .publish_verified(
            verified_presence.clone(),
            unix_ms_to_s(START_UNIX_MS + 2_000),
        )
        .map_err(|error| format!("node-b presence publish failed: {error}"))?;
    if publish_ack.disposition != PublishDisposition::Stored {
        return Err(format!(
            "node-b presence publish expected stored disposition, got {:?}",
            publish_ack.disposition
        ));
    }
    node_b.runtime.context_mut().set_local_presence(
        verified_presence.clone(),
        unix_ms_to_s(START_UNIX_MS + 2_000),
    );
    node_a
        .runtime
        .context_mut()
        .rendezvous_mut()
        .publish_verified(verified_presence, unix_ms_to_s(START_UNIX_MS + 2_001))
        .map_err(|error| format!("node-a lookup store ingest failed: {error}"))?;
    write_step(
        writer,
        json!({
            "step": "publish_presence",
            "node": node_b.name,
            "node_id": presence_record.node_id.to_string(),
            "disposition": "stored",
            "reachability_mode": presence_record.reachability_mode,
        }),
    )?;

    let lookup_timestamp_unix_s = unix_ms_to_s(START_UNIX_MS + 3_000);
    let mut lookup_state = node_a
        .runtime
        .context()
        .rendezvous()
        .lookup_state(4)
        .map_err(|error| format!("failed to create lookup state for node-a: {error}"))?;
    let lookup_response = node_a.runtime.context_mut().rendezvous_mut().lookup(
        LookupNode {
            node_id: node_b.runtime.context().node_id(),
        },
        lookup_timestamp_unix_s,
        &mut lookup_state,
    );
    let looked_up_presence = match lookup_response {
        LookupResponse::Result(result) => {
            write_step(
                writer,
                json!({
                    "step": "lookup_node",
                    "node": node_a.name,
                    "target_node": node_b.name,
                    "target_node_id": result.node_id.to_string(),
                    "remaining_budget": result.remaining_budget,
                }),
            )?;
            result.record
        }
        LookupResponse::NotFound(not_found) => {
            return Err(format!(
                "node-a lookup for node-b failed with reason {:?}",
                not_found.reason
            ));
        }
    };

    let service_record =
        build_signed_service_record(node_b.runtime.context().signing_key(), SERVICE_NAME)?;
    let verified_service_record = service_record
        .clone()
        .verify_with_public_key(&node_b.runtime.context().signing_key().public_key())
        .map_err(|error| format!("failed to verify node-b service record: {error}"))?;
    node_b
        .runtime
        .context_mut()
        .service_registry_mut()
        .register_verified(verified_service_record, LocalServicePolicy::allow_all())
        .map_err(|error| format!("node-b service registration failed: {error}"))?;
    let resolved_service = node_b
        .runtime
        .context()
        .service_registry()
        .resolve(GetServiceRecord {
            app_id: service_record.app_id,
        });
    if resolved_service.status != ServiceRecordResponseStatus::Found {
        return Err(format!(
            "node-b service resolve expected found status, got {:?}",
            resolved_service.status
        ));
    }
    let open_result = node_b
        .runtime
        .context_mut()
        .service_registry_mut()
        .open_app_session(
            OpenAppSession {
                app_id: service_record.app_id,
                reachability_ref: service_record.reachability_ref.clone(),
            },
            START_UNIX_MS + 4_000,
        );
    if open_result.status != OpenAppSessionStatus::Opened {
        return Err(format!(
            "node-b open app session expected opened status, got {:?}",
            open_result.status
        ));
    }
    let service_session_id = open_result
        .session_id
        .ok_or_else(|| "opened service session did not return a session id".to_string())?;
    write_step(
        writer,
        json!({
            "step": "open_service",
            "client_node": node_a.name,
            "target_node": node_b.name,
            "service_name": SERVICE_NAME,
            "app_id": service_record.app_id.to_string(),
            "session_id": service_session_id,
        }),
    )?;

    let relay_node_id = relay.runtime.context().node_id();
    let node_b_id = node_b.runtime.context().node_id();
    let relay_hint = RelayHint {
        relay_node_id,
        relay_transport_class: "tcp".to_string(),
        relay_score: 90,
        relay_policy: vec![1_u8],
        expiry: unix_ms_to_s(START_UNIX_MS) + PRESENCE_TTL_S,
    };
    let intro_ticket = build_intro_ticket(
        node_b.runtime.context().signing_key(),
        REQUESTER_BINDING,
        unix_ms_to_s(START_UNIX_MS) + 300,
    )?;
    let plan = build_reachability_plan(
        &looked_up_presence,
        std::slice::from_ref(&relay_hint),
        &intro_ticket,
        REQUESTER_BINDING,
        unix_ms_to_s(START_UNIX_MS + 5_000),
    )
    .map_err(|error| format!("relay reachability planning failed: {error}"))?;
    if plan.direct_attempts != vec![TransportClass::Quic, TransportClass::Tcp] {
        return Err(format!(
            "relay reachability plan expected direct attempts [quic, tcp], got {:?}",
            plan.direct_attempts
        ));
    }
    let fallback = plan.relay_fallbacks.first().ok_or_else(|| {
        "relay reachability plan did not produce a fallback candidate".to_string()
    })?;
    write_step(
        writer,
        json!({
            "step": "relay_fallback_planned",
            "client_node": node_a.name,
            "target_node": node_b.name,
            "relay_node": relay.name,
            "forced_direct_failure": true,
            "direct_attempts": plan
                .direct_attempts
                .iter()
                .map(|transport| transport.as_str())
                .collect::<Vec<_>>(),
            "fallback_transport": fallback.relay_transport_class.as_str(),
        }),
    )?;

    let resolve_intro = ResolveIntro {
        relay_node_id: fallback.relay_node_id,
        intro_ticket: intro_ticket.into_inner(),
    }
    .verify_with_public_key(&node_b.runtime.context().signing_key().public_key())
    .map_err(|error| format!("resolve-intro verification failed: {error}"))?;
    let intro_response = relay
        .runtime
        .context_mut()
        .relay_manager_mut()
        .process_resolve_intro(
            relay_node_id,
            resolve_intro,
            REQUESTER_BINDING,
            unix_ms_to_s(START_UNIX_MS + 5_100),
        );
    if intro_response.status != IntroResponseStatus::Forwarded {
        return Err(format!(
            "relay intro expected forwarded status, got {:?}",
            intro_response.status
        ));
    }
    let tunnel = relay
        .runtime
        .context_mut()
        .relay_manager_mut()
        .bind_tunnel(
            RELAY_TUNNEL_ID,
            relay_node_id,
            node_b_id,
            unix_ms_to_s(START_UNIX_MS + 5_200),
        )
        .map_err(|error| format!("relay tunnel bind failed: {error}"))?;
    relay
        .runtime
        .context_mut()
        .relay_manager_mut()
        .note_relayed_bytes(relay_node_id, 1_024, unix_ms_to_s(START_UNIX_MS + 5_201))
        .map_err(|error| format!("relay byte accounting failed: {error}"))?;
    write_step(
        writer,
        json!({
            "step": "relay_fallback_bound",
            "client_node": node_a.name,
            "target_node": node_b.name,
            "relay_node": relay.name,
            "relay_node_id": relay.runtime.context().node_id().to_string(),
            "tunnel_id": tunnel.tunnel_id,
        }),
    )?;

    write_step(
        writer,
        json!({
            "step": "smoke_complete",
            "node_a_id": node_a.runtime.context().node_id().to_string(),
            "node_b_id": node_b.runtime.context().node_id().to_string(),
            "node_c_id": node_c.runtime.context().node_id().to_string(),
            "relay_node_id": relay.runtime.context().node_id().to_string(),
            "service_app_id": service_record.app_id.to_string(),
            "service_session_id": service_session_id,
            "relay_tunnel_id": tunnel.tunnel_id,
        }),
    )?;

    run_long_soak(
        &mut node_a,
        &mut node_b,
        &mut node_c,
        &mut relay,
        START_UNIX_MS + 6_000,
        options,
        writer,
    )?;

    Ok(SmokeReport {
        node_a_id: node_a.runtime.context().node_id(),
        node_b_id: node_b.runtime.context().node_id(),
        node_c_id: node_c.runtime.context().node_id(),
        relay_node_id: relay.runtime.context().node_id(),
        service_app_id: service_record.app_id,
        service_session_id,
        relay_tunnel_id: tunnel.tunnel_id,
    })
}

fn load_node(devnet_dir: &Path, name: &'static str) -> Result<DevnetNode, String> {
    let config_path = devnet_dir.join("configs").join(format!("{name}.json"));
    let mut runtime = NodeRuntime::from_config_path(&config_path).map_err(|error| {
        format!(
            "failed to load {name} config {}: {error}",
            config_path.display()
        )
    })?;
    runtime.context_mut().config_mut().tcp_listener_addr = None;
    Ok(DevnetNode { name, runtime })
}

fn startup_node(
    node: &mut DevnetNode,
    timestamp_unix_ms: u64,
    writer: &mut dyn Write,
) -> Result<(), String> {
    node.runtime
        .startup(timestamp_unix_ms)
        .map_err(|error| format!("{} startup failed: {error}", node.name))?;
    let snapshot = node.runtime.snapshot();
    if !matches!(
        snapshot.state,
        NodeRuntimeState::Running | NodeRuntimeState::Degraded
    ) {
        return Err(format!(
            "{} startup reached unexpected state {:?}",
            node.name, snapshot.state
        ));
    }
    write_step(
        writer,
        json!({
            "step": "startup",
            "node": node.name,
            "node_id": snapshot.node_id.to_string(),
            "state": snapshot.state,
            "active_peers": snapshot.active_peers,
        }),
    )
}

fn establish_placeholder_session(
    client: &mut DevnetNode,
    server: &mut DevnetNode,
    client_session_id: u64,
    server_session_id: u64,
    timestamp_unix_ms: u64,
    writer: &mut dyn Write,
) -> Result<(), String> {
    client
        .runtime
        .open_placeholder_session(client_session_id, Box::new(TcpTransport), timestamp_unix_ms)
        .map_err(|error| format!("{} open placeholder session failed: {error}", client.name))?;
    server
        .runtime
        .open_placeholder_session(server_session_id, Box::new(TcpTransport), timestamp_unix_ms)
        .map_err(|error| format!("{} open placeholder session failed: {error}", server.name))?;

    let handshake_config = HandshakeConfig::default();
    let client_signing_key = client.runtime.context().signing_key().clone();
    let server_signing_key = server.runtime.context().signing_key().clone();
    let client_ephemeral_secret = X25519StaticSecret::from_bytes([11_u8; 32]);
    let server_ephemeral_secret = X25519StaticSecret::from_bytes([12_u8; 32]);
    let (client_handshake, client_hello) = ClientHandshake::start(
        handshake_config,
        client_signing_key,
        client_ephemeral_secret,
    );
    let (server_handshake, server_hello) = ServerHandshake::accept(
        handshake_config,
        server_signing_key,
        server_ephemeral_secret,
        &client_hello,
    )
    .map_err(|error| format!("server handshake accept failed: {error}"))?;
    let (client_outcome, server_outcome) = {
        let (client_finish, client_outcome) =
            client_handshake
                .handle_server_hello(&server_hello)
                .map_err(|error| format!("client handshake completion failed: {error}"))?;
        let server_outcome = server_handshake
            .handle_client_finish(&client_finish)
            .map_err(|error| format!("server handshake completion failed: {error}"))?;
        (client_outcome, server_outcome)
    };

    client
        .runtime
        .managed_session_mut(client_session_id)
        .ok_or_else(|| format!("{} missing client session after open", client.name))?
        .handle_runner_input(
            timestamp_unix_ms + 10,
            SessionRunnerInput::HandshakeSucceeded {
                outcome: client_outcome,
            },
        )
        .map_err(|error| format!("{} session establishment failed: {error}", client.name))?;
    server
        .runtime
        .managed_session_mut(server_session_id)
        .ok_or_else(|| format!("{} missing server session after open", server.name))?
        .handle_runner_input(
            timestamp_unix_ms + 20,
            SessionRunnerInput::HandshakeSucceeded {
                outcome: server_outcome,
            },
        )
        .map_err(|error| format!("{} session establishment failed: {error}", server.name))?;

    client
        .runtime
        .tick(timestamp_unix_ms + 30)
        .map_err(|error| {
            format!(
                "{} tick after session establishment failed: {error}",
                client.name
            )
        })?;
    server
        .runtime
        .tick(timestamp_unix_ms + 40)
        .map_err(|error| {
            format!(
                "{} tick after session establishment failed: {error}",
                server.name
            )
        })?;

    let client_state = client
        .runtime
        .managed_session(client_session_id)
        .ok_or_else(|| format!("{} missing client session after handshake", client.name))?
        .state();
    let server_state = server
        .runtime
        .managed_session(server_session_id)
        .ok_or_else(|| format!("{} missing server session after handshake", server.name))?
        .state();
    if client_state != SessionState::Established || server_state != SessionState::Established {
        return Err(format!(
            "handshake did not establish both sessions: client={client_state:?} server={server_state:?}"
        ));
    }

    write_step(
        writer,
        json!({
            "step": "session_established",
            "client_node": client.name,
            "server_node": server.name,
            "client_node_id": client.runtime.context().node_id().to_string(),
            "server_node_id": server.runtime.context().node_id().to_string(),
            "transport": "tcp",
        }),
    )
}

fn run_long_soak(
    node_a: &mut DevnetNode,
    node_b: &mut DevnetNode,
    node_c: &mut DevnetNode,
    relay: &mut DevnetNode,
    start_unix_ms: u64,
    options: SmokeOptions,
    writer: &mut dyn Write,
) -> Result<(), String> {
    if options.soak_seconds == 0 {
        return Ok(());
    }

    node_a
        .runtime
        .upsert_path_state(sample_soak_path_state())
        .map_err(|error| format!("{} soak path setup failed: {error}", node_a.name))?;
    write_step(
        writer,
        json!({
            "step": "soak_started",
            "soak_seconds": options.soak_seconds,
            "status_interval_seconds": options.status_interval_seconds,
        }),
    )?;

    for elapsed_s in 1..=options.soak_seconds {
        let tick_unix_ms = start_unix_ms.saturating_add(elapsed_s.saturating_mul(1_000));
        tick_node(node_a, tick_unix_ms, 0)?;
        tick_node(node_b, tick_unix_ms, 10)?;
        tick_node(node_c, tick_unix_ms, 20)?;
        tick_node(relay, tick_unix_ms, 30)?;

        if options
            .status_interval_seconds
            .map(|interval| elapsed_s % interval == 0)
            .unwrap_or(false)
        {
            write_health_step(writer, node_a, elapsed_s)?;
            write_health_step(writer, node_b, elapsed_s)?;
            write_health_step(writer, node_c, elapsed_s)?;
            write_health_step(writer, relay, elapsed_s)?;
        }
    }

    let node_a_health = node_a.runtime.health_snapshot();
    let node_b_health = node_b.runtime.health_snapshot();
    let relay_health = relay.runtime.health_snapshot();
    if node_a_health.runtime.managed_sessions != 0 || node_b_health.runtime.managed_sessions != 0 {
        return Err(format!(
            "stale sessions were not fully reaped: node-a={} node-b={}",
            node_a_health.runtime.managed_sessions, node_b_health.runtime.managed_sessions
        ));
    }
    if node_b_health.runtime.open_service_sessions != 0 {
        return Err(format!(
            "stale service sessions were not pruned on {}",
            node_b.name
        ));
    }
    if relay_health.relay.active_tunnels != 0 {
        return Err(format!(
            "stale relay tunnels were not pruned on {}",
            relay.name
        ));
    }
    if node_a_health.cleanup_totals.stale_path_probes_pruned == 0 {
        return Err("path-probe cleanup did not prune any stale probes during soak".to_string());
    }
    if node_b_health.metrics.publish_presence_total == 0 {
        return Err("presence refresh did not republish during soak".to_string());
    }

    write_step(
        writer,
        json!({
            "step": "soak_complete",
            "soak_seconds": options.soak_seconds,
            "node_a_health": node_a_health,
            "node_b_health": node_b_health,
            "relay_health": relay_health,
        }),
    )
}

fn tick_node(node: &mut DevnetNode, base_unix_ms: u64, offset_ms: u64) -> Result<(), String> {
    node.runtime
        .tick(base_unix_ms.saturating_add(offset_ms))
        .map(|_| ())
        .map_err(|error| format!("{} soak tick failed: {error}", node.name))
}

fn write_health_step(
    writer: &mut dyn Write,
    node: &DevnetNode,
    elapsed_s: u64,
) -> Result<(), String> {
    write_step(
        writer,
        json!({
            "step": "runtime_status",
            "node": node.name,
            "elapsed_s": elapsed_s,
            "health": node.runtime.health_snapshot(),
        }),
    )
}

fn sample_soak_path_state() -> PathState {
    PathState {
        path_id: 9_001,
        metrics: PathMetrics {
            est_rtt_ms: 45,
            obs_rtt_ms: 45,
            jitter_ms: 3,
            loss_ppm: 0,
            relay_hops: 0,
            censorship_risk_level: 0,
            diversity_bonus: 1,
        },
    }
}

fn build_signed_presence_record(
    signing_key: &Ed25519SigningKey,
    expires_at_unix_s: u64,
    relay_node_id: NodeId,
) -> Result<PresenceRecord, String> {
    let mut record = PresenceRecord {
        version: 1,
        node_id: derive_node_id(signing_key.public_key().as_bytes()),
        epoch: 11,
        expires_at_unix_s,
        sequence: 1,
        transport_classes: vec!["quic".to_string(), "relay".to_string(), "tcp".to_string()],
        reachability_mode: "hybrid".to_string(),
        locator_commitment: b"devnet/node-b".to_vec(),
        encrypted_contact_blobs: vec![b"tcp://127.0.0.1:4102".to_vec()],
        relay_hint_refs: vec![relay_node_id.as_bytes().to_vec()],
        intro_policy: "allow".to_string(),
        capability_requirements: vec!["service-host".to_string()],
        signature: Vec::new(),
    };
    let body = record
        .canonical_body_bytes()
        .map_err(|error| format!("failed to encode presence body: {error}"))?;
    record.signature = signing_key.sign(&body).as_bytes().to_vec();
    Ok(record)
}

fn build_signed_service_record(
    signing_key: &Ed25519SigningKey,
    service_name: &str,
) -> Result<ServiceRecord, String> {
    let node_id = derive_node_id(signing_key.public_key().as_bytes());
    let mut record = ServiceRecord {
        version: 1,
        node_id,
        app_id: derive_app_id(&node_id, SERVICE_NAMESPACE, service_name),
        service_name: service_name.to_string(),
        service_version: "1.0.0".to_string(),
        auth_mode: "none".to_string(),
        policy: vec![1_u8, 2, 3, 4],
        reachability_ref: b"devnet-reachability/node-b".to_vec(),
        metadata_commitment: b"devnet-metadata/node-b".to_vec(),
        signature: Vec::new(),
    };
    let body = record
        .canonical_body_bytes()
        .map_err(|error| format!("failed to encode service body: {error}"))?;
    record.signature = signing_key.sign(&body).as_bytes().to_vec();
    Ok(record)
}

fn build_intro_ticket(
    signing_key: &Ed25519SigningKey,
    requester_binding: &[u8],
    expires_at_unix_s: u64,
) -> Result<overlay_core::records::VerifiedIntroTicket, String> {
    let mut ticket = IntroTicket {
        ticket_id: b"devnet-intro-ticket".to_vec(),
        target_node_id: derive_node_id(signing_key.public_key().as_bytes()),
        requester_binding: requester_binding.to_vec(),
        scope: "relay-intro".to_string(),
        issued_at_unix_s: unix_ms_to_s(START_UNIX_MS),
        expires_at_unix_s,
        nonce: b"devnet-nonce".to_vec(),
        signature: Vec::new(),
    };
    let body = ticket
        .canonical_body_bytes()
        .map_err(|error| format!("failed to encode intro ticket body: {error}"))?;
    ticket.signature = signing_key.sign(&body).as_bytes().to_vec();
    ticket
        .verify_with_public_key(&signing_key.public_key())
        .map_err(|error| format!("failed to verify intro ticket: {error}"))
}

fn unix_ms_to_s(timestamp_unix_ms: u64) -> u64 {
    timestamp_unix_ms / 1_000
}

fn write_step(writer: &mut dyn Write, value: serde_json::Value) -> Result<(), String> {
    writeln!(writer, "{}", value).map_err(|error| format!("failed to write smoke output: {error}"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        run_smoke_with_writer, run_smoke_with_writer_and_options, SmokeOptions, SERVICE_NAME,
    };

    #[test]
    fn repo_devnet_smoke_flow_succeeds() {
        let mut output = Vec::new();
        let report = run_smoke_with_writer(&repo_devnet_dir(), &mut output)
            .expect("sample devnet smoke flow should succeed");
        assert_ne!(report.node_a_id, report.node_b_id);
        assert_ne!(report.node_b_id, report.relay_node_id);
        assert_ne!(report.service_session_id, 0);
        assert_eq!(report.relay_tunnel_id, 7_001);

        let rendered = String::from_utf8(output).expect("smoke output should be utf-8");
        assert!(rendered.contains("\"step\":\"startup\""));
        assert!(rendered.contains("\"step\":\"session_established\""));
        assert!(rendered.contains("\"step\":\"publish_presence\""));
        assert!(rendered.contains("\"step\":\"lookup_node\""));
        assert!(rendered.contains("\"step\":\"open_service\""));
        assert!(rendered.contains("\"step\":\"relay_fallback_bound\""));
        assert!(rendered.contains(SERVICE_NAME));
    }

    #[test]
    fn repo_devnet_soak_exercises_runtime_health_and_cleanup() {
        let mut output = Vec::new();
        let report = run_smoke_with_writer_and_options(
            &repo_devnet_dir(),
            &mut output,
            SmokeOptions {
                soak_seconds: 660,
                status_interval_seconds: Some(220),
            },
        )
        .expect("sample devnet soak flow should succeed");
        assert_ne!(report.service_session_id, 0);

        let rendered = String::from_utf8(output).expect("smoke output should be utf-8");
        assert!(rendered.contains("\"step\":\"soak_started\""));
        assert!(rendered.contains("\"step\":\"runtime_status\""));
        assert!(rendered.contains("\"step\":\"soak_complete\""));
        assert!(rendered.contains("\"stale_path_probes_pruned\":"));
        assert!(rendered.contains("\"open_service_sessions\":0"));
    }

    fn repo_devnet_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("devnet")
    }
}
