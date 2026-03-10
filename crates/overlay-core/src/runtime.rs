//! Long-running node runtime orchestration with Milestone 12 launch hardening.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use thiserror::Error;

use crate::{
    bootstrap::{BootstrapProvider, BootstrapProviderError, BootstrapResponse},
    config::{ConfigError, OverlayConfig},
    crypto::sign::{Ed25519SigningKey, ED25519_SECRET_KEY_LEN},
    error::{PresenceVerificationError, RecordEncodingError},
    identity::{derive_node_id, NodeId},
    metrics::{LogComponent, LogContext, MetricsSnapshot, Observability},
    peer::{PeerStore, PeerStoreError},
    relay::{RelayCleanupSummary, RelayError, RelayManager, RelayUsageSnapshot},
    rendezvous::{PublishPresence, RendezvousError, RendezvousStore, VerifiedPublishPresence},
    routing::{
        HysteresisConfig, PathProbe, PathProbeTracker, PathState, RouteDecision, RouteSelector,
        RoutingError, DEFAULT_MAX_IN_FLIGHT_PATH_PROBES_PER_PATH,
        DEFAULT_PATH_PROBE_TIMEOUT_MULTIPLIER,
    },
    service::{ServiceError, ServiceRegistry},
    session::{
        ReplayCache, ReplayCacheConfig, ReplayCacheError, SessionError, SessionEvent,
        SessionIoAction, SessionIoActionKind, SessionManager, SessionRunnerInput, SessionState,
    },
    transport::{TransportRunner, TransportRunnerError},
};

const PRESENCE_REFRESH_DIVISOR: u64 = 2;
const MIN_BOOTSTRAP_RETRY_INTERVAL_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRuntimeState {
    Init,
    Bootstrapping,
    Running,
    Degraded,
    ShuttingDown,
}

impl NodeRuntimeState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Bootstrapping => "bootstrapping",
            Self::Running => "running",
            Self::Degraded => "degraded",
            Self::ShuttingDown => "shutting_down",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTickSummary {
    pub timestamp_unix_ms: u64,
    pub state: NodeRuntimeState,
    pub session_events: Vec<SessionEvent>,
    pub session_io_actions: Vec<SessionIoAction>,
    pub scheduled_path_probes: Vec<PathProbe>,
    pub route_decision: Option<RouteDecision>,
    pub presence_refreshed: bool,
    pub bootstrap_retry_attempted: bool,
    pub bootstrap_sources_accepted: usize,
    pub managed_sessions_reaped: usize,
    pub stale_service_sessions_pruned: usize,
    pub stale_relay_tunnels_pruned: usize,
    pub stale_path_probes_pruned: usize,
    pub replay_entries_pruned: usize,
    pub stale_published_records_pruned: usize,
    pub stale_negative_cache_entries_pruned: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NodeRuntimeSnapshot {
    pub state: NodeRuntimeState,
    pub node_id: NodeId,
    pub active_peers: usize,
    pub managed_sessions: usize,
    pub tracked_paths: usize,
    pub selected_path_id: Option<u64>,
    pub published_records: usize,
    pub negative_cache_entries: usize,
    pub open_service_sessions: usize,
    pub recent_logs: usize,
    pub last_tick_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct RuntimeCleanupTotals {
    pub replay_entries_pruned: u64,
    pub stale_published_records_pruned: u64,
    pub stale_negative_cache_entries_pruned: u64,
    pub stale_service_sessions_pruned: u64,
    pub stale_relay_tunnels_pruned: u64,
    pub stale_path_probes_pruned: u64,
    pub managed_sessions_reaped: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct BootstrapRuntimeStatus {
    pub configured_sources: usize,
    pub last_attempt_unix_ms: Option<u64>,
    pub last_success_unix_ms: Option<u64>,
    pub last_accepted_sources: usize,
    pub total_attempts: u64,
    pub total_successes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NodeRuntimeResourceLimits {
    pub max_managed_sessions: usize,
    pub max_tracked_paths: usize,
    pub max_published_records: usize,
    pub max_negative_cache_entries: usize,
    pub max_registered_services: usize,
    pub max_open_service_sessions: usize,
    pub max_transport_buffer_bytes: usize,
    pub max_relay_tunnels: usize,
    pub max_relay_intro_requests_per_minute: usize,
    pub max_replay_entries: usize,
    pub max_log_entries: usize,
    pub max_in_flight_path_probes_per_path: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NodeRuntimeMaintenancePolicy {
    pub bootstrap_retry_interval_ms: u64,
    pub stale_service_session_age_ms: u64,
    pub stale_relay_tunnel_age_s: u64,
    pub path_probe_timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeRuntimeHealthSnapshot {
    pub runtime: NodeRuntimeSnapshot,
    pub total_peers: usize,
    pub metrics: MetricsSnapshot,
    pub relay: RelayUsageSnapshot,
    pub resource_limits: NodeRuntimeResourceLimits,
    pub maintenance_policy: NodeRuntimeMaintenancePolicy,
    pub cleanup_totals: RuntimeCleanupTotals,
    pub bootstrap: BootstrapRuntimeStatus,
}

#[derive(Debug, Error)]
pub enum NodeRuntimeError {
    #[error("node runtime operation '{operation}' is invalid while state is {state:?}")]
    InvalidState {
        state: NodeRuntimeState,
        operation: &'static str,
    },
    #[error(
        "managed-session supervisor would exceed max_managed_sessions ({max_managed_sessions})"
    )]
    SessionLimitExceeded { max_managed_sessions: usize },
    #[error("tracked-path store would exceed max_tracked_paths ({max_tracked_paths})")]
    PathLimitExceeded { max_tracked_paths: usize },
    #[error("managed session {correlation_id} already exists")]
    DuplicateSession { correlation_id: u64 },
    #[error("failed to read config file {path}: {source}")]
    ConfigRead { path: PathBuf, source: io::Error },
    #[error("failed to parse config file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to read node key file {path}: {source}")]
    NodeKeyRead { path: PathBuf, source: io::Error },
    #[error(
        "node key file {path} must be exactly {expected} raw bytes or {expected_hex} hex characters, got {actual} bytes"
    )]
    InvalidNodeKeyLength {
        path: PathBuf,
        expected: usize,
        expected_hex: usize,
        actual: usize,
    },
    #[error("node key file {path} contains invalid hex data")]
    InvalidNodeKeyHex { path: PathBuf },
    #[error(transparent)]
    ConfigValidation(#[from] ConfigError),
    #[error(transparent)]
    RecordEncoding(#[from] RecordEncodingError),
    #[error(transparent)]
    PresenceVerification(#[from] PresenceVerificationError),
    #[error(transparent)]
    PeerStore(#[from] PeerStoreError),
    #[error(transparent)]
    Rendezvous(#[from] RendezvousError),
    #[error(transparent)]
    Relay(#[from] RelayError),
    #[error(transparent)]
    Routing(#[from] RoutingError),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    ReplayCache(#[from] ReplayCacheError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("local presence refresh sequence exhausted for node {node_id}")]
    LocalPresenceSequenceExhausted { node_id: NodeId },
}

#[derive(Clone)]
pub struct NodeContext {
    config: OverlayConfig,
    signing_key: Ed25519SigningKey,
    node_id: NodeId,
    observability: Observability,
    peer_store: PeerStore,
    rendezvous: RendezvousStore,
    relay_manager: RelayManager,
    path_probe_tracker: PathProbeTracker,
    route_selector: RouteSelector,
    path_states: BTreeMap<u64, PathState>,
    service_registry: ServiceRegistry,
    replay_cache: ReplayCache,
    local_presence: Option<VerifiedPublishPresence>,
    next_presence_refresh_unix_s: Option<u64>,
}

impl NodeContext {
    pub fn new(
        config: OverlayConfig,
        signing_key: Ed25519SigningKey,
    ) -> Result<Self, NodeRuntimeError> {
        let config = config.validate()?;
        let node_id = derive_node_id(signing_key.public_key().as_bytes());
        Ok(Self {
            peer_store: PeerStore::new(config.peer_store_config())?,
            rendezvous: RendezvousStore::new(config.rendezvous_config())?,
            relay_manager: RelayManager::new(config.relay_config())?,
            path_probe_tracker: PathProbeTracker::new(config.path_probe_config())?,
            route_selector: RouteSelector::new(HysteresisConfig::default())?,
            service_registry: ServiceRegistry::new(config.service_config())?,
            replay_cache: ReplayCache::new(ReplayCacheConfig::default())?,
            config,
            signing_key,
            node_id,
            observability: Observability::default(),
            path_states: BTreeMap::new(),
            local_presence: None,
            next_presence_refresh_unix_s: None,
        })
    }

    pub fn config(&self) -> &OverlayConfig {
        &self.config
    }

    pub fn signing_key(&self) -> &Ed25519SigningKey {
        &self.signing_key
    }

    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn observability(&self) -> &Observability {
        &self.observability
    }

    pub fn observability_mut(&mut self) -> &mut Observability {
        &mut self.observability
    }

    pub fn peer_store(&self) -> &PeerStore {
        &self.peer_store
    }

    pub fn rendezvous(&self) -> &RendezvousStore {
        &self.rendezvous
    }

    pub fn rendezvous_mut(&mut self) -> &mut RendezvousStore {
        &mut self.rendezvous
    }

    pub fn replay_cache(&self) -> &ReplayCache {
        &self.replay_cache
    }

    pub fn replay_cache_mut(&mut self) -> &mut ReplayCache {
        &mut self.replay_cache
    }

    pub fn relay_manager(&self) -> &RelayManager {
        &self.relay_manager
    }

    pub fn relay_manager_mut(&mut self) -> &mut RelayManager {
        &mut self.relay_manager
    }

    pub fn service_registry(&self) -> &ServiceRegistry {
        &self.service_registry
    }

    pub fn service_registry_mut(&mut self) -> &mut ServiceRegistry {
        &mut self.service_registry
    }

    pub fn set_local_presence(&mut self, record: VerifiedPublishPresence, now_unix_s: u64) {
        self.local_presence = Some(record);
        self.next_presence_refresh_unix_s = Some(now_unix_s);
    }

    fn presence_refresh_interval_s(&self) -> u64 {
        (self.config.presence_ttl_s / PRESENCE_REFRESH_DIVISOR).max(1)
    }
}

struct ManagedSession {
    session: SessionManager,
    transport: Box<dyn TransportRunner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BootstrapSource {
    File { path: PathBuf },
    Unsupported { source: String },
}

pub struct NodeRuntime {
    state: NodeRuntimeState,
    context: NodeContext,
    bootstrap_sources: Vec<BootstrapSource>,
    managed_sessions: BTreeMap<u64, ManagedSession>,
    max_managed_sessions: usize,
    next_correlation_id: u64,
    last_tick_unix_ms: Option<u64>,
    cleanup_totals: RuntimeCleanupTotals,
    bootstrap_status: BootstrapRuntimeStatus,
}

impl NodeRuntime {
    pub fn new(context: NodeContext) -> Self {
        let max_managed_sessions = context.config.max_total_neighbors;
        let configured_sources = context.config.bootstrap_sources.len();
        let mut runtime = Self {
            state: NodeRuntimeState::Init,
            context,
            bootstrap_sources: Vec::new(),
            managed_sessions: BTreeMap::new(),
            max_managed_sessions,
            next_correlation_id: 1,
            last_tick_unix_ms: None,
            cleanup_totals: RuntimeCleanupTotals::default(),
            bootstrap_status: BootstrapRuntimeStatus {
                configured_sources,
                ..BootstrapRuntimeStatus::default()
            },
        };
        runtime.log_state_transition(current_unix_ms(), NodeRuntimeState::Init);
        runtime
    }

    pub fn from_config_path(config_path: impl AsRef<Path>) -> Result<Self, NodeRuntimeError> {
        let config_path = config_path.as_ref();
        let config_bytes =
            fs::read(config_path).map_err(|source| NodeRuntimeError::ConfigRead {
                path: config_path.to_path_buf(),
                source,
            })?;
        let config = serde_json::from_slice::<OverlayConfig>(&config_bytes).map_err(|source| {
            NodeRuntimeError::ConfigParse {
                path: config_path.to_path_buf(),
                source,
            }
        })?;
        let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        let signing_key = load_node_signing_key(&resolve_path(base_dir, &config.node_key_path))?;
        let bootstrap_sources = config
            .bootstrap_sources
            .iter()
            .map(|source| resolve_bootstrap_source(base_dir, source))
            .collect();
        let context = NodeContext::new(config, signing_key)?;
        let mut runtime = Self::new(context);
        runtime.bootstrap_sources = bootstrap_sources;
        runtime.bootstrap_status.configured_sources = runtime.bootstrap_sources.len();
        Ok(runtime)
    }

    pub const fn state(&self) -> NodeRuntimeState {
        self.state
    }

    pub fn context(&self) -> &NodeContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut NodeContext {
        &mut self.context
    }

    pub fn managed_session_count(&self) -> usize {
        self.managed_sessions.len()
    }

    pub fn snapshot(&self) -> NodeRuntimeSnapshot {
        NodeRuntimeSnapshot {
            state: self.state,
            node_id: self.context.node_id,
            active_peers: self.context.peer_store.active_neighbors().count(),
            managed_sessions: self.managed_sessions.len(),
            tracked_paths: self.context.path_states.len(),
            selected_path_id: self.context.route_selector.current_path_id(),
            published_records: self.context.rendezvous.published_record_count(),
            negative_cache_entries: self.context.rendezvous.negative_cache_len(),
            open_service_sessions: self.context.service_registry.open_session_count(),
            recent_logs: self.context.observability.logs().len(),
            last_tick_unix_ms: self.last_tick_unix_ms,
        }
    }

    pub fn health_snapshot(&self) -> NodeRuntimeHealthSnapshot {
        NodeRuntimeHealthSnapshot {
            runtime: self.snapshot(),
            total_peers: self.context.peer_store.neighbor_count(),
            metrics: self.context.observability.metrics().clone(),
            relay: self.context.relay_manager.usage_snapshot(),
            resource_limits: self.resource_limits(),
            maintenance_policy: self.maintenance_policy(),
            cleanup_totals: self.cleanup_totals,
            bootstrap: self.bootstrap_status,
        }
    }

    pub fn startup_now(&mut self) -> Result<(), NodeRuntimeError> {
        self.startup(current_unix_ms())
    }

    pub fn startup(&mut self, timestamp_unix_ms: u64) -> Result<(), NodeRuntimeError> {
        self.ensure_state("startup", self.state == NodeRuntimeState::Init)?;
        self.log_state_transition(timestamp_unix_ms, NodeRuntimeState::Bootstrapping);

        let now_unix_s = unix_ms_to_s(timestamp_unix_ms);
        self.refresh_bootstrap_sources(timestamp_unix_ms, now_unix_s)?;
        self.sync_active_peer_gauge();
        self.sync_operating_state(timestamp_unix_ms);
        Ok(())
    }

    pub fn tick_now(&mut self) -> Result<RuntimeTickSummary, NodeRuntimeError> {
        self.tick(current_unix_ms())
    }

    pub fn tick(&mut self, timestamp_unix_ms: u64) -> Result<RuntimeTickSummary, NodeRuntimeError> {
        self.ensure_state(
            "tick",
            matches!(
                self.state,
                NodeRuntimeState::Running | NodeRuntimeState::Degraded
            ),
        )?;

        let now_unix_s = unix_ms_to_s(timestamp_unix_ms);
        let replay_before = self.context.replay_cache.observed_count();
        self.context
            .replay_cache
            .prune_expired_entries(timestamp_unix_ms);
        let replay_after = self.context.replay_cache.observed_count();

        let published_before = self.context.rendezvous.published_record_count();
        let negative_before = self.context.rendezvous.negative_cache_len();
        self.context.rendezvous.prune_expired(now_unix_s);
        let published_after_cleanup = self.context.rendezvous.published_record_count();
        let negative_after_cleanup = self.context.rendezvous.negative_cache_len();

        let stale_service_sessions_pruned = self.prune_stale_service_sessions(timestamp_unix_ms);
        let relay_cleanup = self.prune_stale_relay_state(timestamp_unix_ms, now_unix_s);
        let stale_path_probes_pruned = self.expire_stale_path_probes(timestamp_unix_ms);
        let bootstrap_sources_accepted =
            self.retry_bootstrap_if_due(timestamp_unix_ms, now_unix_s)?;
        let bootstrap_retry_attempted = self.bootstrap_status.last_attempt_unix_ms
            == Some(timestamp_unix_ms)
            && self.bootstrap_status.total_attempts > 1;
        self.sync_active_peer_gauge();
        self.sync_operating_state(timestamp_unix_ms);

        let presence_refreshed = self.refresh_presence_if_due(timestamp_unix_ms, now_unix_s)?;
        let route_decision = self.evaluate_routes(timestamp_unix_ms, now_unix_s);
        let scheduled_path_probes = self.schedule_path_probes(timestamp_unix_ms)?;
        let (session_events, session_io_actions, managed_sessions_reaped) =
            self.poll_managed_sessions(timestamp_unix_ms)?;

        self.context.observability.push_log(
            allocate_log_context(
                &mut self.next_correlation_id,
                self.context.node_id,
                timestamp_unix_ms,
            ),
            LogComponent::Runtime,
            "tick",
            if self.state == NodeRuntimeState::Degraded {
                "degraded"
            } else {
                "ok"
            },
        );
        self.last_tick_unix_ms = Some(timestamp_unix_ms);

        let replay_entries_pruned = replay_before.saturating_sub(replay_after);
        let stale_published_records_pruned =
            published_before.saturating_sub(published_after_cleanup);
        let stale_negative_cache_entries_pruned =
            negative_before.saturating_sub(negative_after_cleanup);
        self.cleanup_totals.replay_entries_pruned = self
            .cleanup_totals
            .replay_entries_pruned
            .saturating_add(replay_entries_pruned as u64);
        self.cleanup_totals.stale_published_records_pruned = self
            .cleanup_totals
            .stale_published_records_pruned
            .saturating_add(stale_published_records_pruned as u64);
        self.cleanup_totals.stale_negative_cache_entries_pruned = self
            .cleanup_totals
            .stale_negative_cache_entries_pruned
            .saturating_add(stale_negative_cache_entries_pruned as u64);
        self.cleanup_totals.stale_service_sessions_pruned = self
            .cleanup_totals
            .stale_service_sessions_pruned
            .saturating_add(stale_service_sessions_pruned as u64);
        self.cleanup_totals.stale_relay_tunnels_pruned = self
            .cleanup_totals
            .stale_relay_tunnels_pruned
            .saturating_add(relay_cleanup.stale_tunnels_pruned as u64);
        self.cleanup_totals.stale_path_probes_pruned = self
            .cleanup_totals
            .stale_path_probes_pruned
            .saturating_add(stale_path_probes_pruned as u64);
        self.cleanup_totals.managed_sessions_reaped = self
            .cleanup_totals
            .managed_sessions_reaped
            .saturating_add(managed_sessions_reaped as u64);

        Ok(RuntimeTickSummary {
            timestamp_unix_ms,
            state: self.state,
            session_events,
            session_io_actions,
            scheduled_path_probes,
            route_decision,
            presence_refreshed,
            bootstrap_retry_attempted,
            bootstrap_sources_accepted,
            managed_sessions_reaped,
            stale_service_sessions_pruned,
            stale_relay_tunnels_pruned: relay_cleanup.stale_tunnels_pruned,
            stale_path_probes_pruned,
            replay_entries_pruned,
            stale_published_records_pruned,
            stale_negative_cache_entries_pruned,
        })
    }

    pub fn shutdown_now(&mut self) -> Result<(), NodeRuntimeError> {
        self.shutdown(current_unix_ms())
    }

    pub fn shutdown(&mut self, timestamp_unix_ms: u64) -> Result<(), NodeRuntimeError> {
        self.ensure_state(
            "shutdown",
            !matches!(self.state, NodeRuntimeState::ShuttingDown),
        )?;
        self.log_state_transition(timestamp_unix_ms, NodeRuntimeState::ShuttingDown);

        let node_id = self.context.node_id;
        for managed in self.managed_sessions.values_mut() {
            match managed.session.state() {
                SessionState::Opening | SessionState::Established | SessionState::Degraded => {
                    let event = managed
                        .session
                        .begin_close(timestamp_unix_ms, Some("runtime shutdown".to_string()))?;
                    event.record_with_observability(node_id, &mut self.context.observability);
                    for action in managed.session.drain_io_actions() {
                        apply_session_io_action(&mut *managed.transport, &action);
                    }
                    let closed = managed.session.mark_closed(timestamp_unix_ms)?;
                    closed.record_with_observability(node_id, &mut self.context.observability);
                }
                SessionState::Closing => {
                    let closed = managed.session.mark_closed(timestamp_unix_ms)?;
                    closed.record_with_observability(node_id, &mut self.context.observability);
                }
                SessionState::Idle | SessionState::Closed => {}
            }
        }

        self.managed_sessions.clear();
        SessionManager::sync_established_session_gauge(
            std::iter::empty::<&SessionManager>(),
            &mut self.context.observability,
        );
        Ok(())
    }

    pub fn open_placeholder_session(
        &mut self,
        correlation_id: u64,
        mut transport: Box<dyn TransportRunner>,
        timestamp_unix_ms: u64,
    ) -> Result<SessionEvent, NodeRuntimeError> {
        self.ensure_state(
            "open_placeholder_session",
            !matches!(self.state, NodeRuntimeState::ShuttingDown),
        )?;
        if self.managed_sessions.contains_key(&correlation_id) {
            return Err(NodeRuntimeError::DuplicateSession { correlation_id });
        }
        if self.managed_sessions.len() == self.max_managed_sessions {
            return Err(NodeRuntimeError::SessionLimitExceeded {
                max_managed_sessions: self.max_managed_sessions,
            });
        }

        ignore_unsupported_runner_op(transport.begin_open(correlation_id));
        let mut session = SessionManager::with_node_id(correlation_id, self.context.node_id);
        let event = session.begin_open(timestamp_unix_ms, transport.as_ref())?;
        event.record_with_observability(self.context.node_id, &mut self.context.observability);
        self.managed_sessions
            .insert(correlation_id, ManagedSession { session, transport });
        SessionManager::sync_established_session_gauge(
            self.managed_sessions
                .values()
                .map(|managed| &managed.session),
            &mut self.context.observability,
        );

        Ok(event)
    }

    pub fn managed_session(&self, correlation_id: u64) -> Option<&SessionManager> {
        self.managed_sessions
            .get(&correlation_id)
            .map(|managed| &managed.session)
    }

    pub fn managed_session_mut(&mut self, correlation_id: u64) -> Option<&mut SessionManager> {
        self.managed_sessions
            .get_mut(&correlation_id)
            .map(|managed| &mut managed.session)
    }

    pub fn upsert_path_state(&mut self, path_state: PathState) -> Result<(), NodeRuntimeError> {
        if !self.context.path_states.contains_key(&path_state.path_id)
            && self.context.path_states.len() == self.max_managed_sessions
        {
            return Err(NodeRuntimeError::PathLimitExceeded {
                max_tracked_paths: self.max_managed_sessions,
            });
        }

        self.context
            .path_states
            .insert(path_state.path_id, path_state);
        Ok(())
    }

    fn resource_limits(&self) -> NodeRuntimeResourceLimits {
        let rendezvous = self.context.rendezvous.config();
        let service = self.context.service_registry.config();
        let relay = self.context.relay_manager.config();
        let replay_cache = self.context.replay_cache.config();
        NodeRuntimeResourceLimits {
            max_managed_sessions: self.max_managed_sessions,
            max_tracked_paths: self.max_managed_sessions,
            max_published_records: rendezvous.max_published_records,
            max_negative_cache_entries: rendezvous.max_negative_cache_entries,
            max_registered_services: service.max_registered_services,
            max_open_service_sessions: service.max_open_service_sessions,
            max_transport_buffer_bytes: self.context.config.max_transport_buffer_bytes,
            max_relay_tunnels: relay.max_concurrent_relay_tunnels,
            max_relay_intro_requests_per_minute: relay.max_intro_requests_per_minute,
            max_replay_entries: replay_cache.max_entries,
            max_log_entries: self.context.observability.max_log_entries(),
            max_in_flight_path_probes_per_path: DEFAULT_MAX_IN_FLIGHT_PATH_PROBES_PER_PATH,
        }
    }

    fn maintenance_policy(&self) -> NodeRuntimeMaintenancePolicy {
        NodeRuntimeMaintenancePolicy {
            bootstrap_retry_interval_ms: self.bootstrap_retry_interval_ms(),
            stale_service_session_age_ms: self.stale_service_session_age_ms(),
            stale_relay_tunnel_age_s: self.stale_relay_tunnel_age_s(),
            path_probe_timeout_ms: self.path_probe_timeout_ms(),
        }
    }

    fn bootstrap_retry_interval_ms(&self) -> u64 {
        let quarter_presence_ttl_ms = self.context.config.presence_ttl_s.saturating_mul(1_000) / 4;
        MIN_BOOTSTRAP_RETRY_INTERVAL_MS
            .max(self.context.config.path_probe_interval_ms)
            .max(quarter_presence_ttl_ms)
    }

    fn stale_service_session_age_ms(&self) -> u64 {
        self.context.config.presence_ttl_s.saturating_mul(1_000)
    }

    fn stale_relay_tunnel_age_s(&self) -> u64 {
        self.context.config.presence_ttl_s
    }

    fn path_probe_timeout_ms(&self) -> u64 {
        self.context
            .path_probe_tracker
            .config()
            .path_probe_interval_ms
            .saturating_mul(DEFAULT_PATH_PROBE_TIMEOUT_MULTIPLIER)
    }

    fn refresh_bootstrap_sources(
        &mut self,
        timestamp_unix_ms: u64,
        now_unix_s: u64,
    ) -> Result<usize, NodeRuntimeError> {
        let node_id = self.context.node_id;
        let mut accepted_sources = 0usize;
        self.bootstrap_status.last_attempt_unix_ms = Some(timestamp_unix_ms);
        self.bootstrap_status.total_attempts =
            self.bootstrap_status.total_attempts.saturating_add(1);

        for source in &self.bootstrap_sources {
            let response = source.fetch_validated_response_with_observability(
                now_unix_s,
                &mut self.context.observability,
                allocate_log_context(&mut self.next_correlation_id, node_id, timestamp_unix_ms),
            );
            let Ok(response) = response else {
                continue;
            };

            if self
                .context
                .peer_store
                .ingest_bootstrap_response_with_observability(
                    response,
                    now_unix_s,
                    &mut self.context.observability,
                    allocate_log_context(&mut self.next_correlation_id, node_id, timestamp_unix_ms),
                )
                .is_ok()
            {
                accepted_sources = accepted_sources.saturating_add(1);
            }
        }

        self.bootstrap_status.last_accepted_sources = accepted_sources;
        if accepted_sources > 0 {
            self.bootstrap_status.last_success_unix_ms = Some(timestamp_unix_ms);
            self.bootstrap_status.total_successes =
                self.bootstrap_status.total_successes.saturating_add(1);
        }

        Ok(accepted_sources)
    }

    fn retry_bootstrap_if_due(
        &mut self,
        timestamp_unix_ms: u64,
        now_unix_s: u64,
    ) -> Result<usize, NodeRuntimeError> {
        if self.state != NodeRuntimeState::Degraded {
            return Ok(0);
        }

        let retry_due = self
            .bootstrap_status
            .last_attempt_unix_ms
            .map(|last_attempt_unix_ms| {
                timestamp_unix_ms.saturating_sub(last_attempt_unix_ms)
                    >= self.bootstrap_retry_interval_ms()
            })
            .unwrap_or(true);
        if !retry_due {
            return Ok(0);
        }

        let accepted_sources = self.refresh_bootstrap_sources(timestamp_unix_ms, now_unix_s)?;
        self.context.observability.push_log(
            allocate_log_context(
                &mut self.next_correlation_id,
                self.context.node_id,
                timestamp_unix_ms,
            ),
            LogComponent::Runtime,
            "bootstrap_retry",
            if accepted_sources > 0 {
                "accepted"
            } else {
                "unavailable"
            },
        );
        Ok(accepted_sources)
    }

    fn sync_active_peer_gauge(&mut self) {
        self.context
            .observability
            .set_active_peers(self.context.peer_store.active_neighbors().count());
    }

    fn sync_operating_state(&mut self, timestamp_unix_ms: u64) {
        let next_state = if self.context.peer_store.active_neighbors().next().is_some() {
            NodeRuntimeState::Running
        } else {
            NodeRuntimeState::Degraded
        };
        if self.state != next_state {
            self.log_state_transition(timestamp_unix_ms, next_state);
        }
    }

    fn prune_stale_service_sessions(&mut self, timestamp_unix_ms: u64) -> usize {
        let pruned = self
            .context
            .service_registry
            .prune_stale_sessions(timestamp_unix_ms, self.stale_service_session_age_ms());
        if pruned > 0 {
            self.context.observability.push_log(
                allocate_log_context(
                    &mut self.next_correlation_id,
                    self.context.node_id,
                    timestamp_unix_ms,
                ),
                LogComponent::Service,
                "cleanup_open_sessions",
                "pruned",
            );
        }
        pruned
    }

    fn prune_stale_relay_state(
        &mut self,
        timestamp_unix_ms: u64,
        now_unix_s: u64,
    ) -> RelayCleanupSummary {
        let cleanup = self
            .context
            .relay_manager
            .prune_stale_state(now_unix_s, self.stale_relay_tunnel_age_s());
        if cleanup.stale_tunnels_pruned > 0 {
            self.context.observability.push_log(
                allocate_log_context(
                    &mut self.next_correlation_id,
                    self.context.node_id,
                    timestamp_unix_ms,
                ),
                LogComponent::Relay,
                "cleanup_relay_state",
                "pruned",
            );
        }
        cleanup
    }

    fn expire_stale_path_probes(&mut self, timestamp_unix_ms: u64) -> usize {
        let expired = self
            .context
            .path_probe_tracker
            .expire_stale_probes(timestamp_unix_ms);
        for expired_probe in &expired {
            if let Some(path_state) = self.context.path_states.get_mut(&expired_probe.path_id) {
                path_state.observe_probe_feedback(expired_probe.feedback);
            }
            self.context.observability.observe_probe_feedback(
                expired_probe.feedback.obs_rtt_ms,
                expired_probe.feedback.loss_ppm,
            );
            self.context.observability.push_log(
                allocate_log_context(
                    &mut self.next_correlation_id,
                    self.context.node_id,
                    timestamp_unix_ms,
                ),
                LogComponent::Routing,
                "probe_feedback",
                "expired",
            );
        }
        expired.len()
    }

    fn ensure_state(
        &self,
        operation: &'static str,
        predicate: bool,
    ) -> Result<(), NodeRuntimeError> {
        if predicate {
            Ok(())
        } else {
            Err(NodeRuntimeError::InvalidState {
                state: self.state,
                operation,
            })
        }
    }

    fn log_state_transition(&mut self, timestamp_unix_ms: u64, new_state: NodeRuntimeState) {
        self.state = new_state;
        self.context.observability.push_log(
            allocate_log_context(
                &mut self.next_correlation_id,
                self.context.node_id,
                timestamp_unix_ms,
            ),
            LogComponent::Runtime,
            "state_transition",
            new_state.as_str(),
        );
    }

    fn refresh_presence_if_due(
        &mut self,
        timestamp_unix_ms: u64,
        now_unix_s: u64,
    ) -> Result<bool, NodeRuntimeError> {
        let Some(next_presence_refresh_unix_s) = self.context.next_presence_refresh_unix_s else {
            return Ok(false);
        };
        if now_unix_s < next_presence_refresh_unix_s {
            return Ok(false);
        }

        self.context.next_presence_refresh_unix_s =
            Some(now_unix_s.saturating_add(self.context.presence_refresh_interval_s()));

        let Some(record) = self.refreshed_local_presence(now_unix_s)? else {
            self.context.observability.push_log(
                allocate_log_context(
                    &mut self.next_correlation_id,
                    self.context.node_id,
                    timestamp_unix_ms,
                ),
                LogComponent::Runtime,
                "presence_refresh",
                "skipped",
            );
            return Ok(false);
        };

        match self.context.rendezvous.publish_verified_with_observability(
            record.clone(),
            now_unix_s,
            &mut self.context.observability,
            allocate_log_context(
                &mut self.next_correlation_id,
                self.context.node_id,
                timestamp_unix_ms,
            ),
        ) {
            Ok(_) => {
                self.context.local_presence = Some(record);
                Ok(true)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn refreshed_local_presence(
        &self,
        now_unix_s: u64,
    ) -> Result<Option<VerifiedPublishPresence>, NodeRuntimeError> {
        let Some(record) = self.context.local_presence.clone() else {
            return Ok(None);
        };

        let mut record = record.into_record();
        if record.sequence == u64::MAX {
            return Err(NodeRuntimeError::LocalPresenceSequenceExhausted {
                node_id: self.context.node_id,
            });
        }

        record.sequence += 1;
        record.expires_at_unix_s = now_unix_s.saturating_add(self.context.config.presence_ttl_s);
        let body = record.canonical_body_bytes()?;
        record.signature = self.context.signing_key.sign(&body).as_bytes().to_vec();

        PublishPresence { record }
            .verify_with_public_key(&self.context.signing_key.public_key())
            .map(Some)
            .map_err(Into::into)
    }

    fn evaluate_routes(
        &mut self,
        timestamp_unix_ms: u64,
        now_unix_s: u64,
    ) -> Option<RouteDecision> {
        if self.context.path_states.is_empty() {
            return None;
        }

        let candidates = self
            .context
            .path_states
            .values()
            .copied()
            .collect::<Vec<_>>();
        Some(self.context.route_selector.evaluate_with_observability(
            now_unix_s,
            &candidates,
            &mut self.context.observability,
            allocate_log_context(
                &mut self.next_correlation_id,
                self.context.node_id,
                timestamp_unix_ms,
            ),
        ))
    }

    fn schedule_path_probes(
        &mut self,
        timestamp_unix_ms: u64,
    ) -> Result<Vec<PathProbe>, NodeRuntimeError> {
        let path_ids = self.context.path_states.keys().copied().collect::<Vec<_>>();
        let mut scheduled = Vec::new();

        for path_id in path_ids {
            match self
                .context
                .path_probe_tracker
                .begin_probe(path_id, timestamp_unix_ms)
            {
                Ok(Some(probe)) => {
                    self.context.observability.push_log(
                        allocate_log_context(
                            &mut self.next_correlation_id,
                            self.context.node_id,
                            timestamp_unix_ms,
                        ),
                        LogComponent::Routing,
                        "probe_tick",
                        "scheduled",
                    );
                    scheduled.push(probe);
                }
                Ok(None) => {}
                Err(error) => {
                    self.context.observability.push_log(
                        allocate_log_context(
                            &mut self.next_correlation_id,
                            self.context.node_id,
                            timestamp_unix_ms,
                        ),
                        LogComponent::Routing,
                        "probe_tick",
                        "rejected",
                    );
                    return Err(error.into());
                }
            }
        }

        Ok(scheduled)
    }

    fn poll_managed_sessions(
        &mut self,
        timestamp_unix_ms: u64,
    ) -> Result<(Vec<SessionEvent>, Vec<SessionIoAction>, usize), NodeRuntimeError> {
        let mut session_events = Vec::new();
        let mut session_io_actions = Vec::new();
        let mut closed_ids = Vec::new();
        let transport_buffer_config = self.context.config.transport_buffer_config();
        let node_id = self.context.node_id;

        for (correlation_id, managed) in self.managed_sessions.iter_mut() {
            match managed.transport.poll_event(timestamp_unix_ms) {
                Ok(Some(event)) => {
                    match SessionRunnerInput::from_transport_poll_event(
                        event,
                        transport_buffer_config,
                    ) {
                        Ok(Some(input)) => {
                            let session_event =
                                managed.session.handle_runner_input_with_replay_cache(
                                    timestamp_unix_ms,
                                    input,
                                    &mut self.context.replay_cache,
                                )?;
                            session_event.record_with_observability(
                                node_id,
                                &mut self.context.observability,
                            );
                            session_events.push(session_event);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let session_event =
                                managed.session.fail(timestamp_unix_ms, error.to_string())?;
                            session_event.record_with_observability(
                                node_id,
                                &mut self.context.observability,
                            );
                            session_events.push(session_event);
                        }
                    }
                }
                Ok(None) => {}
                Err(TransportRunnerError::UnsupportedOperation { .. }) => {}
            }

            for event in managed.session.poll_timers(timestamp_unix_ms)? {
                event.record_with_observability(node_id, &mut self.context.observability);
                session_events.push(event);
            }

            let actions = managed.session.drain_io_actions();
            for action in &actions {
                apply_session_io_action(&mut *managed.transport, action);
            }
            session_io_actions.extend(actions);

            if managed.session.state() == SessionState::Closed {
                closed_ids.push(*correlation_id);
            }
        }

        let reaped_sessions = closed_ids.len();
        for correlation_id in closed_ids {
            self.managed_sessions.remove(&correlation_id);
        }
        SessionManager::sync_established_session_gauge(
            self.managed_sessions
                .values()
                .map(|managed| &managed.session),
            &mut self.context.observability,
        );

        Ok((session_events, session_io_actions, reaped_sessions))
    }
}

impl BootstrapSource {
    fn fetch_validated_response_with_observability(
        &self,
        now_unix_s: u64,
        observability: &mut Observability,
        context: LogContext,
    ) -> Result<BootstrapResponse, BootstrapProviderError> {
        match self {
            Self::File { path } => BootstrapFileProvider { path: path.clone() }
                .fetch_validated_response_with_observability(now_unix_s, observability, context),
            Self::Unsupported { source } => UnsupportedBootstrapProvider {
                source: source.clone(),
            }
            .fetch_validated_response_with_observability(now_unix_s, observability, context),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootstrapFileProvider {
    path: PathBuf,
}

impl BootstrapProvider for BootstrapFileProvider {
    fn provider_name(&self) -> &'static str {
        "file"
    }

    fn fetch_response(&self) -> Result<BootstrapResponse, BootstrapProviderError> {
        let bytes = fs::read(&self.path).map_err(|error| {
            BootstrapProviderError::Unavailable(format!(
                "could not read bootstrap source {}: {error}",
                self.path.display()
            ))
        })?;
        serde_json::from_slice(&bytes).map_err(|error| {
            BootstrapProviderError::Unavailable(format!(
                "could not parse bootstrap source {}: {error}",
                self.path.display()
            ))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnsupportedBootstrapProvider {
    source: String,
}

impl BootstrapProvider for UnsupportedBootstrapProvider {
    fn provider_name(&self) -> &'static str {
        "unsupported"
    }

    fn fetch_response(&self) -> Result<BootstrapResponse, BootstrapProviderError> {
        Err(BootstrapProviderError::Unavailable(format!(
            "bootstrap source '{}' is not supported by the Milestone 10 runtime",
            self.source
        )))
    }
}

fn allocate_log_context(
    next_correlation_id: &mut u64,
    node_id: NodeId,
    timestamp_unix_ms: u64,
) -> LogContext {
    let correlation_id = *next_correlation_id;
    *next_correlation_id = next_correlation_id.saturating_add(1);
    LogContext {
        timestamp_unix_ms,
        node_id,
        correlation_id,
    }
}

fn apply_session_io_action(transport: &mut dyn TransportRunner, action: &SessionIoAction) {
    match action.action {
        SessionIoActionKind::StartClose => {
            ignore_unsupported_runner_op(transport.begin_close(action.correlation_id));
        }
        SessionIoActionKind::AbortTransport => {
            ignore_unsupported_runner_op(transport.abort(action.correlation_id));
        }
        SessionIoActionKind::BeginHandshake | SessionIoActionKind::SendKeepalive => {}
    }
}

fn ignore_unsupported_runner_op(result: Result<(), TransportRunnerError>) {
    match result {
        Ok(()) | Err(TransportRunnerError::UnsupportedOperation { .. }) => {}
    }
}

fn resolve_bootstrap_source(base_dir: &Path, source: &str) -> BootstrapSource {
    let trimmed = source.trim();
    if let Some(path) = trimmed.strip_prefix("file:") {
        return BootstrapSource::File {
            path: resolve_path(base_dir, Path::new(path.trim())),
        };
    }

    let path = Path::new(trimmed);
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") && !trimmed.contains("://") {
        BootstrapSource::File {
            path: resolve_path(base_dir, path),
        }
    } else {
        BootstrapSource::Unsupported {
            source: trimmed.to_string(),
        }
    }
}

fn resolve_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn load_node_signing_key(path: &Path) -> Result<Ed25519SigningKey, NodeRuntimeError> {
    let bytes = fs::read(path).map_err(|source| NodeRuntimeError::NodeKeyRead {
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.len() == ED25519_SECRET_KEY_LEN {
        let mut seed = [0_u8; ED25519_SECRET_KEY_LEN];
        seed.copy_from_slice(&bytes);
        return Ok(Ed25519SigningKey::from_seed(seed));
    }

    let trimmed = std::str::from_utf8(&bytes)
        .ok()
        .map(str::trim)
        .unwrap_or_default();
    if trimmed.len() == ED25519_SECRET_KEY_LEN * 2 {
        let decoded =
            decode_hex_seed(trimmed).ok_or_else(|| NodeRuntimeError::InvalidNodeKeyHex {
                path: path.to_path_buf(),
            })?;
        return Ok(Ed25519SigningKey::from_seed(decoded));
    }

    Err(NodeRuntimeError::InvalidNodeKeyLength {
        path: path.to_path_buf(),
        expected: ED25519_SECRET_KEY_LEN,
        expected_hex: ED25519_SECRET_KEY_LEN * 2,
        actual: bytes.len(),
    })
}

fn decode_hex_seed(input: &str) -> Option<[u8; ED25519_SECRET_KEY_LEN]> {
    let mut decoded = [0_u8; ED25519_SECRET_KEY_LEN];
    let bytes = input.as_bytes();
    for (index, chunk) in bytes.chunks_exact(2).enumerate() {
        decoded[index] = (hex_value(chunk[0])? << 4) | hex_value(chunk[1])?;
    }
    Some(decoded)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn current_unix_ms() -> u64 {
    unix_duration_ms(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}

fn unix_duration_ms(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn unix_ms_to_s(timestamp_unix_ms: u64) -> u64 {
    timestamp_unix_ms / 1_000
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        env, fs, io,
        path::{Path, PathBuf},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use serde::Serialize;

    use super::{unix_ms_to_s, NodeContext, NodeRuntime, NodeRuntimeState, RuntimeTickSummary};
    use crate::{
        bootstrap::{
            BootstrapNetworkParams, BootstrapPeer, BootstrapPeerRole, BootstrapResponse,
            BOOTSTRAP_SCHEMA_VERSION,
        },
        config::OverlayConfig,
        crypto::{aead::ChaCha20Poly1305Key, hash::Blake3Digest, sign::Ed25519SigningKey},
        identity::{derive_app_id, derive_node_id, NodeId},
        records::{PresenceRecord, ServiceRecord},
        rendezvous::VerifiedPublishPresence,
        routing::{PathMetrics, PathState, RouteDecision},
        service::{LocalServicePolicy, OpenAppSession, OpenAppSessionStatus},
        session::{HandshakeOutcome, SessionIoActionKind},
        transport::{Transport, TransportClass, TransportPollEvent, TransportRunner},
        wire::MAX_FRAME_BODY_LEN,
        REPOSITORY_STAGE,
    };

    const START_UNIX_MS: u64 = 1_700_000_000_000;

    #[test]
    fn startup_loads_config_key_and_bootstraps_runtime() {
        let dir = unique_test_dir("runtime-startup");
        let key_path = dir.join("node.key");
        let config_path = dir.join("overlay-config.json");
        let bootstrap_path = dir.join("bootstrap.json");
        let signing_key = Ed25519SigningKey::from_seed([31_u8; 32]);

        fs::write(&key_path, signing_key.as_bytes()).expect("key file should be written");
        write_json(&bootstrap_path, &sample_bootstrap_response())
            .expect("bootstrap file should be written");
        write_json(
            &config_path,
            &sample_config("node.key", vec!["bootstrap.json".to_string()]),
        )
        .expect("config file should be written");

        let mut runtime =
            NodeRuntime::from_config_path(&config_path).expect("runtime should load from config");
        runtime
            .startup(START_UNIX_MS)
            .expect("startup should succeed");

        assert_eq!(REPOSITORY_STAGE, "milestone-9-hardening");
        assert_eq!(runtime.state(), NodeRuntimeState::Running);
        assert_eq!(
            runtime.context().node_id(),
            derive_node_id(signing_key.public_key().as_bytes())
        );
        assert_eq!(runtime.snapshot().active_peers, 3);
        assert!(runtime
            .context()
            .observability()
            .logs()
            .iter()
            .any(
                |entry| entry.component == crate::metrics::LogComponent::Runtime
                    && entry.event == "state_transition"
                    && entry.result == "running"
            ));
    }

    #[test]
    fn tick_orchestrates_sessions_presence_probes_and_cleanup() {
        let signing_key = Ed25519SigningKey::from_seed([41_u8; 32]);
        let mut runtime = NodeRuntime::new(
            NodeContext::new(
                sample_config("node.key", vec!["static:seed-a".to_string()]),
                signing_key.clone(),
            )
            .expect("context should initialize"),
        );
        runtime
            .startup(START_UNIX_MS)
            .expect("startup should degrade, not fail");
        assert_eq!(runtime.state(), NodeRuntimeState::Degraded);

        runtime
            .open_placeholder_session(7, Box::new(MockTransportRunner::default()), START_UNIX_MS)
            .expect("session should open");
        runtime
            .managed_session_mut(7)
            .expect("managed session should exist")
            .mark_established(START_UNIX_MS + 10)
            .expect("session should establish");

        let local_presence =
            verified_presence_record(&signing_key, 2, 2, unix_ms_to_s(START_UNIX_MS) + 900);
        runtime
            .context_mut()
            .set_local_presence(local_presence, unix_ms_to_s(START_UNIX_MS));

        let stale_presence =
            verified_presence_record(&signing_key, 1, 1, unix_ms_to_s(START_UNIX_MS) + 5);
        runtime
            .context_mut()
            .rendezvous_mut()
            .publish_verified(stale_presence, unix_ms_to_s(START_UNIX_MS))
            .expect("stale-at-future-time record should publish before expiry");

        let missing_node_id = NodeId::from_bytes([88_u8; 32]);
        let mut lookup_state = runtime
            .context()
            .rendezvous()
            .lookup_state(1)
            .expect("lookup state should be built");
        runtime.context_mut().rendezvous_mut().lookup(
            crate::rendezvous::LookupNode {
                node_id: missing_node_id,
            },
            unix_ms_to_s(START_UNIX_MS),
            &mut lookup_state,
        );

        runtime
            .context_mut()
            .replay_cache_mut()
            .observe_outcome(&sample_handshake_outcome(), START_UNIX_MS)
            .expect("replay cache should accept sample outcome");
        runtime
            .managed_session_mut(7)
            .expect("managed session should exist")
            .record_activity(
                START_UNIX_MS + 380_000,
                Some("test tick alignment".to_string()),
            )
            .expect("activity should refresh session timers");

        runtime
            .upsert_path_state(PathState {
                path_id: 5,
                metrics: PathMetrics {
                    est_rtt_ms: 40,
                    obs_rtt_ms: 40,
                    jitter_ms: 5,
                    loss_ppm: 0,
                    relay_hops: 0,
                    censorship_risk_level: 0,
                    diversity_bonus: 1,
                },
            })
            .expect("path should be tracked");

        let summary = runtime
            .tick(START_UNIX_MS + 400_000)
            .expect("tick should succeed");

        assert_tick_summary(&summary);
        assert_eq!(runtime.context().rendezvous().negative_cache_len(), 0);
        assert_eq!(runtime.context().rendezvous().published_record_count(), 1);
        assert_eq!(runtime.snapshot().selected_path_id, Some(5));
    }

    #[test]
    fn bootstrap_retry_recovers_degraded_runtime_once_source_is_available() {
        let dir = unique_test_dir("runtime-bootstrap-retry");
        let key_path = dir.join("node.key");
        let config_path = dir.join("overlay-config.json");
        let bootstrap_path = dir.join("bootstrap.json");
        let signing_key = Ed25519SigningKey::from_seed([61_u8; 32]);
        let mut config = sample_config("node.key", vec!["bootstrap.json".to_string()]);
        config.presence_ttl_s = 20;

        fs::write(&key_path, signing_key.as_bytes()).expect("key file should be written");
        write_json(&config_path, &config).expect("config file should be written");

        let mut runtime =
            NodeRuntime::from_config_path(&config_path).expect("runtime should load from config");
        runtime
            .startup(START_UNIX_MS)
            .expect("startup should degrade while bootstrap file is missing");
        assert_eq!(runtime.state(), NodeRuntimeState::Degraded);

        let early_retry = runtime
            .tick(START_UNIX_MS + 4_999)
            .expect("tick before retry window should succeed");
        assert!(!early_retry.bootstrap_retry_attempted);
        assert_eq!(runtime.state(), NodeRuntimeState::Degraded);

        write_json(&bootstrap_path, &sample_bootstrap_response())
            .expect("bootstrap file should be written before retry");
        let recovered = runtime
            .tick(START_UNIX_MS + 5_000)
            .expect("retry tick should succeed");

        assert!(recovered.bootstrap_retry_attempted);
        assert_eq!(recovered.bootstrap_sources_accepted, 1);
        assert_eq!(runtime.state(), NodeRuntimeState::Running);
        assert_eq!(runtime.snapshot().active_peers, 3);
        assert_eq!(runtime.health_snapshot().bootstrap.total_attempts, 2);
        assert_eq!(runtime.health_snapshot().bootstrap.total_successes, 1);
    }

    #[test]
    fn tick_prunes_stale_service_sessions_relay_tunnels_and_path_probes() {
        let dir = unique_test_dir("runtime-cleanup");
        let key_path = dir.join("node.key");
        let config_path = dir.join("overlay-config.json");
        let bootstrap_path = dir.join("bootstrap.json");
        let signing_key = Ed25519SigningKey::from_seed([62_u8; 32]);
        let mut config = sample_config("node.key", vec!["bootstrap.json".to_string()]);
        config.relay_mode = true;

        fs::write(&key_path, signing_key.as_bytes()).expect("key file should be written");
        write_json(&bootstrap_path, &sample_bootstrap_response())
            .expect("bootstrap file should be written");
        write_json(&config_path, &config).expect("config file should be written");

        let mut runtime =
            NodeRuntime::from_config_path(&config_path).expect("runtime should load from config");
        runtime
            .startup(START_UNIX_MS)
            .expect("startup should succeed with bootstrap file");
        runtime
            .upsert_path_state(sample_path_state(7))
            .expect("path should be tracked");
        let initial_tick = runtime
            .tick(START_UNIX_MS + 1_000)
            .expect("initial tick should schedule a probe");
        assert_eq!(initial_tick.scheduled_path_probes.len(), 1);

        let service_record = sample_signed_service_record(&signing_key, "terminal");
        runtime
            .context_mut()
            .service_registry_mut()
            .register_verified(
                service_record
                    .clone()
                    .verify_with_public_key(&signing_key.public_key())
                    .expect("service record should verify"),
                LocalServicePolicy::allow_all(),
            )
            .expect("service should register");
        let open = runtime
            .context_mut()
            .service_registry_mut()
            .open_app_session(
                OpenAppSession {
                    app_id: service_record.app_id,
                    reachability_ref: service_record.reachability_ref.clone(),
                },
                START_UNIX_MS + 2_000,
            );
        assert_eq!(open.status, OpenAppSessionStatus::Opened);
        let local_node_id = runtime.context().node_id();
        runtime
            .context_mut()
            .relay_manager_mut()
            .bind_tunnel(
                91,
                local_node_id,
                NodeId::from_bytes([99_u8; 32]),
                unix_ms_to_s(START_UNIX_MS + 2_000),
            )
            .expect("relay tunnel should bind");

        let cleanup_tick = runtime
            .tick(START_UNIX_MS + 122_000)
            .expect("cleanup tick should succeed");

        assert_eq!(cleanup_tick.stale_service_sessions_pruned, 1);
        assert_eq!(cleanup_tick.stale_relay_tunnels_pruned, 1);
        assert_eq!(cleanup_tick.stale_path_probes_pruned, 1);
        assert_eq!(runtime.context().service_registry().open_session_count(), 0);
        assert_eq!(runtime.context().relay_manager().active_tunnel_count(), 0);
        assert_eq!(
            runtime.context().observability().metrics().probe_loss_ratio,
            Some(1_000_000)
        );
        assert_eq!(
            runtime
                .health_snapshot()
                .cleanup_totals
                .stale_service_sessions_pruned,
            1
        );
        assert_eq!(
            runtime
                .health_snapshot()
                .cleanup_totals
                .stale_relay_tunnels_pruned,
            1
        );
    }

    #[test]
    fn timed_out_sessions_are_reaped_from_runtime() {
        let signing_key = Ed25519SigningKey::from_seed([63_u8; 32]);
        let mut runtime = NodeRuntime::new(
            NodeContext::new(
                sample_config("node.key", vec!["static:seed-a".to_string()]),
                signing_key,
            )
            .expect("context should initialize"),
        );
        runtime
            .startup(START_UNIX_MS)
            .expect("startup should degrade, not fail");
        runtime
            .open_placeholder_session(23, Box::new(MockTransportRunner::default()), START_UNIX_MS)
            .expect("session should open");
        runtime
            .managed_session_mut(23)
            .expect("managed session should exist")
            .mark_established(START_UNIX_MS + 10)
            .expect("session should establish");

        let degraded = runtime
            .tick(START_UNIX_MS + 80_000)
            .expect("idle timeout tick should succeed");
        assert!(degraded
            .session_events
            .iter()
            .any(|event| event.event == crate::session::SessionEventKind::Degraded));

        let reaped = runtime
            .tick(START_UNIX_MS + 110_000)
            .expect("degraded timeout tick should succeed");
        assert_eq!(reaped.managed_sessions_reaped, 1);
        assert_eq!(runtime.managed_session_count(), 0);
        assert_eq!(
            runtime
                .health_snapshot()
                .cleanup_totals
                .managed_sessions_reaped,
            1
        );
    }

    #[test]
    fn local_presence_refresh_rolls_expiry_forward_past_original_ttl() {
        let signing_key = Ed25519SigningKey::from_seed([64_u8; 32]);
        let mut runtime = NodeRuntime::new(
            NodeContext::new(
                sample_config("node.key", vec!["static:seed-a".to_string()]),
                signing_key.clone(),
            )
            .expect("context should initialize"),
        );
        runtime
            .startup(START_UNIX_MS)
            .expect("startup should degrade, not fail");

        runtime.context_mut().set_local_presence(
            verified_presence_record(
                &signing_key,
                4,
                9,
                unix_ms_to_s(START_UNIX_MS).saturating_add(60),
            ),
            unix_ms_to_s(START_UNIX_MS),
        );

        runtime
            .tick(START_UNIX_MS + 1_000)
            .expect("first refresh tick should succeed");
        runtime
            .tick(START_UNIX_MS + 121_000)
            .expect("refresh should continue past the original expiry");

        let mut lookup_state = runtime
            .context()
            .rendezvous()
            .lookup_state(1)
            .expect("lookup state should build");
        let lookup = runtime.context_mut().rendezvous_mut().lookup(
            crate::rendezvous::LookupNode {
                node_id: derive_node_id(signing_key.public_key().as_bytes()),
            },
            unix_ms_to_s(START_UNIX_MS + 121_000),
            &mut lookup_state,
        );
        let crate::rendezvous::LookupResponse::Result(result) = lookup else {
            panic!("refreshed local presence should remain available");
        };
        assert!(result.record.expires_at_unix_s > unix_ms_to_s(START_UNIX_MS + 121_000));
        assert!(result.record.sequence > 9);
    }

    #[test]
    fn shutdown_closes_managed_sessions_gracefully() {
        let signing_key = Ed25519SigningKey::from_seed([51_u8; 32]);
        let mut runtime = NodeRuntime::new(
            NodeContext::new(
                sample_config("node.key", vec!["static:seed-a".to_string()]),
                signing_key,
            )
            .expect("context should initialize"),
        );
        runtime
            .startup(START_UNIX_MS)
            .expect("startup should degrade, not fail");
        runtime
            .open_placeholder_session(11, Box::new(MockTransportRunner::default()), START_UNIX_MS)
            .expect("session should open");
        runtime
            .managed_session_mut(11)
            .expect("managed session should exist")
            .mark_established(START_UNIX_MS + 10)
            .expect("session should establish");

        runtime
            .shutdown(START_UNIX_MS + 20)
            .expect("shutdown should succeed");

        assert_eq!(runtime.state(), NodeRuntimeState::ShuttingDown);
        assert_eq!(runtime.managed_session_count(), 0);
        assert_eq!(
            runtime
                .context()
                .observability()
                .metrics()
                .established_sessions,
            0
        );
        assert!(runtime
            .context()
            .observability()
            .logs()
            .iter()
            .any(
                |entry| entry.component == crate::metrics::LogComponent::Runtime
                    && entry.event == "state_transition"
                    && entry.result == "shutting_down"
            ));
    }

    fn assert_tick_summary(summary: &RuntimeTickSummary) {
        assert_eq!(summary.state, NodeRuntimeState::Degraded);
        assert!(summary.presence_refreshed);
        assert!(summary.bootstrap_retry_attempted);
        assert_eq!(summary.bootstrap_sources_accepted, 0);
        assert_eq!(summary.managed_sessions_reaped, 0);
        assert_eq!(summary.stale_service_sessions_pruned, 0);
        assert_eq!(summary.stale_relay_tunnels_pruned, 0);
        assert_eq!(summary.stale_path_probes_pruned, 0);
        assert_eq!(summary.replay_entries_pruned, 1);
        assert_eq!(summary.stale_published_records_pruned, 1);
        assert_eq!(summary.stale_negative_cache_entries_pruned, 1);
        assert_eq!(summary.scheduled_path_probes.len(), 1);
        assert!(matches!(
            summary.route_decision,
            Some(RouteDecision::SelectedInitial { path_id: 5, .. })
        ));
        assert!(summary
            .session_events
            .iter()
            .any(|event| event.event == crate::session::SessionEventKind::KeepaliveDue));
        assert!(summary
            .session_io_actions
            .iter()
            .any(|action| action.action == SessionIoActionKind::BeginHandshake));
        assert!(summary
            .session_io_actions
            .iter()
            .any(|action| action.action == SessionIoActionKind::SendKeepalive));
    }

    #[derive(Debug, Default)]
    struct MockTransportRunner {
        poll_events: VecDeque<TransportPollEvent>,
    }

    impl Transport for MockTransportRunner {
        fn transport_class(&self) -> TransportClass {
            TransportClass::Tcp
        }

        fn adapter_name(&self) -> &'static str {
            "mock-runtime"
        }

        fn is_placeholder(&self) -> bool {
            false
        }
    }

    impl TransportRunner for MockTransportRunner {
        fn begin_open(
            &mut self,
            _correlation_id: u64,
        ) -> Result<(), crate::transport::TransportRunnerError> {
            Ok(())
        }

        fn begin_close(
            &mut self,
            _correlation_id: u64,
        ) -> Result<(), crate::transport::TransportRunnerError> {
            Ok(())
        }

        fn abort(
            &mut self,
            _correlation_id: u64,
        ) -> Result<(), crate::transport::TransportRunnerError> {
            Ok(())
        }

        fn poll_event(
            &mut self,
            _now_unix_ms: u64,
        ) -> Result<Option<TransportPollEvent>, crate::transport::TransportRunnerError> {
            Ok(self.poll_events.pop_front())
        }
    }

    fn sample_config(node_key_path: &str, bootstrap_sources: Vec<String>) -> OverlayConfig {
        OverlayConfig {
            node_key_path: PathBuf::from(node_key_path),
            bootstrap_sources,
            max_total_neighbors: 8,
            max_presence_records: 64,
            max_service_records: 32,
            presence_ttl_s: 120,
            epoch_duration_s: 60,
            path_probe_interval_ms: 1_000,
            max_transport_buffer_bytes: 65_536,
            relay_mode: false,
            log_level: crate::config::LogLevel::Info,
        }
    }

    fn sample_bootstrap_response() -> BootstrapResponse {
        BootstrapResponse {
            version: BOOTSTRAP_SCHEMA_VERSION,
            generated_at_unix_s: unix_ms_to_s(START_UNIX_MS),
            expires_at_unix_s: unix_ms_to_s(START_UNIX_MS) + 600,
            network_params: BootstrapNetworkParams {
                network_id: "overlay-mvp".to_string(),
            },
            epoch_duration_s: 900,
            presence_ttl_s: 1_800,
            max_frame_body_len: MAX_FRAME_BODY_LEN,
            handshake_version: crate::session::HANDSHAKE_VERSION,
            peers: vec![
                bootstrap_peer([1_u8; 32], &["tcp"], BootstrapPeerRole::Standard),
                bootstrap_peer([2_u8; 32], &["quic"], BootstrapPeerRole::Standard),
                bootstrap_peer([3_u8; 32], &["relay"], BootstrapPeerRole::Relay),
            ],
            bridge_hints: Vec::new(),
        }
    }

    fn bootstrap_peer(
        node_id_bytes: [u8; 32],
        transport_classes: &[&str],
        observed_role: BootstrapPeerRole,
    ) -> BootstrapPeer {
        BootstrapPeer {
            node_id: NodeId::from_bytes(node_id_bytes),
            transport_classes: transport_classes
                .iter()
                .map(|class| (*class).to_string())
                .collect(),
            capabilities: if observed_role == BootstrapPeerRole::Relay {
                vec!["relay-forward".to_string()]
            } else {
                Vec::new()
            },
            dial_hints: vec!["tcp://node".to_string()],
            observed_role,
        }
    }

    fn verified_presence_record(
        signing_key: &Ed25519SigningKey,
        epoch: u64,
        sequence: u64,
        expires_at_unix_s: u64,
    ) -> VerifiedPublishPresence {
        let mut record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(signing_key.public_key().as_bytes()),
            epoch,
            expires_at_unix_s,
            sequence,
            transport_classes: vec!["tcp".to_string()],
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
            .expect("presence record should serialize");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();

        crate::rendezvous::PublishPresence { record }
            .verify_with_public_key(&signing_key.public_key())
            .expect("presence record should verify")
    }

    fn sample_path_state(path_id: u64) -> PathState {
        PathState {
            path_id,
            metrics: PathMetrics {
                est_rtt_ms: 40,
                obs_rtt_ms: 40,
                jitter_ms: 5,
                loss_ppm: 0,
                relay_hops: 0,
                censorship_risk_level: 0,
                diversity_bonus: 1,
            },
        }
    }

    fn sample_signed_service_record(
        signing_key: &Ed25519SigningKey,
        service_name: &str,
    ) -> ServiceRecord {
        let node_id = derive_node_id(signing_key.public_key().as_bytes());
        let mut record = ServiceRecord {
            version: 1,
            node_id,
            app_id: derive_app_id(&node_id, "devnet", service_name),
            service_name: service_name.to_string(),
            service_version: "1.0.0".to_string(),
            auth_mode: "none".to_string(),
            policy: vec![1_u8, 2, 3],
            reachability_ref: b"runtime-reachability".to_vec(),
            metadata_commitment: b"runtime-metadata".to_vec(),
            signature: Vec::new(),
        };
        let body = record
            .canonical_body_bytes()
            .expect("service record should serialize");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();
        record
    }

    fn sample_handshake_outcome() -> HandshakeOutcome {
        HandshakeOutcome {
            peer_node_id: NodeId::from_bytes([9_u8; 32]),
            transcript_hash: [7_u8; 32] as Blake3Digest,
            session_keys: crate::session::SessionKeys {
                client_to_server_key: ChaCha20Poly1305Key::from_bytes([1_u8; 32]),
                server_to_client_key: ChaCha20Poly1305Key::from_bytes([2_u8; 32]),
            },
        }
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "overlay-runtime-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn write_json<T>(path: &Path, value: &T) -> io::Result<()>
    where
        T: Serialize,
    {
        let bytes = serde_json::to_vec(value).expect("json should serialize");
        fs::write(path, bytes)
    }
}
