use std::collections::{BTreeMap, VecDeque};

use serde::Serialize;
use thiserror::Error;

use crate::{
    crypto::hash::Blake3Digest,
    identity::NodeId,
    metrics::{LogComponent, LogContext, Observability},
    transport::{Transport, TransportClass},
};

use super::handshake::{HandshakeOutcome, SessionKeys};

const SESSION_COMPONENT: &str = "session";
pub const MAX_SESSION_EVENT_LOG_LEN: usize = 64;
pub const MAX_SESSION_IO_ACTION_QUEUE_LEN: usize = 32;
pub const DEFAULT_REPLAY_CACHE_ENTRIES: usize = 1_024;
pub const DEFAULT_REPLAY_WINDOW_MS: u64 = 300_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Idle,
    Opening,
    Established,
    Degraded,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventKind {
    OpenStarted,
    OpenSucceeded,
    ActivityObserved,
    KeepaliveDue,
    Degraded,
    Recovered,
    CloseStarted,
    Closed,
    TimedOut,
    Failed,
}

impl SessionEventKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OpenStarted => "open_started",
            Self::OpenSucceeded => "open_succeeded",
            Self::ActivityObserved => "activity_observed",
            Self::KeepaliveDue => "keepalive_due",
            Self::Degraded => "degraded",
            Self::Recovered => "recovered",
            Self::CloseStarted => "close_started",
            Self::Closed => "closed",
            Self::TimedOut => "timed_out",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventResult {
    Accepted,
    Ok,
    Degraded,
    Timeout,
    Error,
}

impl SessionEventResult {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Ok => "ok",
            Self::Degraded => "degraded",
            Self::Timeout => "timeout",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    BeginOpen,
    MarkEstablished,
    HandleRunnerInput,
    RecordActivity,
    MarkDegraded,
    MarkRecovered,
    BeginClose,
    MarkClosed,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionIoActionKind {
    BeginHandshake,
    SendKeepalive,
    StartClose,
    AbortTransport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTimerKind {
    Open,
    Keepalive,
    Idle,
    Degraded,
    Close,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    #[error("invalid session transition: cannot {action:?} from {state:?}")]
    InvalidStateTransition {
        state: SessionState,
        action: SessionAction,
    },
    #[error("invalid session timing config: {field} must be non-zero")]
    ZeroTimingValue { field: &'static str },
    #[error(
        "invalid session timing config: idle_timeout_ms ({idle_timeout_ms}) must exceed keepalive_interval_ms ({keepalive_interval_ms})"
    )]
    IdleTimeoutNotAfterKeepalive {
        keepalive_interval_ms: u64,
        idle_timeout_ms: u64,
    },
    #[error(transparent)]
    ReplayCache(#[from] ReplayCacheError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReplayCacheConfig {
    pub max_entries: usize,
    pub replay_window_ms: u64,
}

impl Default for ReplayCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_REPLAY_CACHE_ENTRIES,
            replay_window_ms: DEFAULT_REPLAY_WINDOW_MS,
        }
    }
}

impl ReplayCacheConfig {
    pub fn validate(self) -> Result<Self, ReplayCacheError> {
        for (field, value) in [
            ("max_entries", self.max_entries as u64),
            ("replay_window_ms", self.replay_window_ms),
        ] {
            if value == 0 {
                return Err(ReplayCacheError::ZeroLimit { field });
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ReplayCacheError {
    #[error("replay cache limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
    #[error("handshake transcript replay detected for peer {peer_node_id}")]
    ReplayDetected { peer_node_id: NodeId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReplayCacheEntry {
    transcript_hash: Blake3Digest,
    peer_node_id: NodeId,
    observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCache {
    config: ReplayCacheConfig,
    entries: VecDeque<ReplayCacheEntry>,
    seen: BTreeMap<Blake3Digest, ReplayCacheEntry>,
}

impl ReplayCache {
    pub fn new(config: ReplayCacheConfig) -> Result<Self, ReplayCacheError> {
        Ok(Self {
            config: config.validate()?,
            entries: VecDeque::new(),
            seen: BTreeMap::new(),
        })
    }

    pub const fn config(&self) -> ReplayCacheConfig {
        self.config
    }

    pub fn observed_count(&self) -> usize {
        self.entries.len()
    }

    pub fn observe_outcome(
        &mut self,
        outcome: &HandshakeOutcome,
        now_unix_ms: u64,
    ) -> Result<(), ReplayCacheError> {
        self.prune_expired(now_unix_ms);

        if self.seen.contains_key(&outcome.transcript_hash) {
            return Err(ReplayCacheError::ReplayDetected {
                peer_node_id: outcome.peer_node_id,
            });
        }

        if self.entries.len() == self.config.max_entries {
            self.evict_oldest();
        }

        let entry = ReplayCacheEntry {
            transcript_hash: outcome.transcript_hash,
            peer_node_id: outcome.peer_node_id,
            observed_at_unix_ms: now_unix_ms,
        };
        self.entries.push_back(entry);
        self.seen.insert(entry.transcript_hash, entry);
        Ok(())
    }

    fn prune_expired(&mut self, now_unix_ms: u64) {
        while let Some(entry) = self.entries.front().copied() {
            if now_unix_ms.saturating_sub(entry.observed_at_unix_ms) < self.config.replay_window_ms
            {
                break;
            }
            self.entries.pop_front();
            self.seen.remove(&entry.transcript_hash);
        }
    }

    fn evict_oldest(&mut self) {
        if let Some(entry) = self.entries.pop_front() {
            self.seen.remove(&entry.transcript_hash);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SessionTransportBinding {
    pub transport_class: TransportClass,
    pub adapter_name: &'static str,
    pub is_placeholder: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SessionTimingConfig {
    pub open_timeout_ms: u64,
    pub keepalive_interval_ms: u64,
    pub idle_timeout_ms: u64,
    pub degraded_timeout_ms: u64,
    pub close_timeout_ms: u64,
}

impl SessionTimingConfig {
    pub fn validate(self) -> Result<Self, SessionError> {
        for (field, value) in [
            ("open_timeout_ms", self.open_timeout_ms),
            ("keepalive_interval_ms", self.keepalive_interval_ms),
            ("idle_timeout_ms", self.idle_timeout_ms),
            ("degraded_timeout_ms", self.degraded_timeout_ms),
            ("close_timeout_ms", self.close_timeout_ms),
        ] {
            if value == 0 {
                return Err(SessionError::ZeroTimingValue { field });
            }
        }

        if self.idle_timeout_ms <= self.keepalive_interval_ms {
            return Err(SessionError::IdleTimeoutNotAfterKeepalive {
                keepalive_interval_ms: self.keepalive_interval_ms,
                idle_timeout_ms: self.idle_timeout_ms,
            });
        }

        Ok(self)
    }
}

impl Default for SessionTimingConfig {
    fn default() -> Self {
        Self {
            open_timeout_ms: 10_000,
            keepalive_interval_ms: 15_000,
            idle_timeout_ms: 45_000,
            degraded_timeout_ms: 30_000,
            close_timeout_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct SessionTimerSchedule {
    pub open_deadline_unix_ms: Option<u64>,
    pub keepalive_due_unix_ms: Option<u64>,
    pub idle_deadline_unix_ms: Option<u64>,
    pub degraded_deadline_unix_ms: Option<u64>,
    pub close_deadline_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionSecurityContext {
    pub peer_node_id: NodeId,
    pub transcript_hash: Blake3Digest,
    pub session_keys: SessionKeys,
}

impl From<HandshakeOutcome> for SessionSecurityContext {
    fn from(outcome: HandshakeOutcome) -> Self {
        Self {
            peer_node_id: outcome.peer_node_id,
            transcript_hash: outcome.transcript_hash,
            session_keys: outcome.session_keys,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionEvent {
    pub timestamp_unix_ms: u64,
    pub node_id: Option<NodeId>,
    pub peer_node_id: Option<NodeId>,
    pub correlation_id: u64,
    pub component: &'static str,
    pub event: SessionEventKind,
    pub result: SessionEventResult,
    pub previous_state: SessionState,
    pub new_state: SessionState,
    pub transport: Option<SessionTransportBinding>,
    pub timer: Option<SessionTimerKind>,
    pub detail: Option<String>,
}

impl SessionEvent {
    pub fn record_with_observability(
        &self,
        fallback_node_id: NodeId,
        observability: &mut Observability,
    ) {
        observability.push_log(
            LogContext {
                timestamp_unix_ms: self.timestamp_unix_ms,
                node_id: self.node_id.unwrap_or(fallback_node_id),
                correlation_id: self.correlation_id,
            },
            LogComponent::Session,
            self.event.as_str(),
            self.result.as_str(),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionIoAction {
    pub timestamp_unix_ms: u64,
    pub correlation_id: u64,
    pub action: SessionIoActionKind,
    pub transport: Option<SessionTransportBinding>,
    pub peer_node_id: Option<NodeId>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRunnerInput {
    FrameReceived { byte_len: usize },
    HandshakeSucceeded { outcome: HandshakeOutcome },
    TransportClosed { detail: Option<String> },
    TransportFailed { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionManager {
    correlation_id: u64,
    node_id: Option<NodeId>,
    state: SessionState,
    timing: SessionTimingConfig,
    timers: SessionTimerSchedule,
    transport: Option<SessionTransportBinding>,
    security: Option<SessionSecurityContext>,
    events: Vec<SessionEvent>,
    io_actions: Vec<SessionIoAction>,
}

impl SessionManager {
    pub fn new(correlation_id: u64) -> Self {
        Self::with_timing(correlation_id, SessionTimingConfig::default())
            .expect("default timing config must be valid")
    }

    pub fn with_timing(
        correlation_id: u64,
        timing: SessionTimingConfig,
    ) -> Result<Self, SessionError> {
        Ok(Self {
            correlation_id,
            node_id: None,
            state: SessionState::Idle,
            timing: timing.validate()?,
            timers: SessionTimerSchedule::default(),
            transport: None,
            security: None,
            events: Vec::new(),
            io_actions: Vec::new(),
        })
    }

    pub fn with_node_id(correlation_id: u64, node_id: NodeId) -> Self {
        Self::with_node_id_and_timing(correlation_id, node_id, SessionTimingConfig::default())
            .expect("default timing config must be valid")
    }

    pub fn with_node_id_and_timing(
        correlation_id: u64,
        node_id: NodeId,
        timing: SessionTimingConfig,
    ) -> Result<Self, SessionError> {
        let mut manager = Self::with_timing(correlation_id, timing)?;
        manager.node_id = Some(node_id);
        Ok(manager)
    }

    pub const fn state(&self) -> SessionState {
        self.state
    }

    pub const fn correlation_id(&self) -> u64 {
        self.correlation_id
    }

    pub const fn timing(&self) -> SessionTimingConfig {
        self.timing
    }

    pub const fn timers(&self) -> SessionTimerSchedule {
        self.timers
    }

    pub const fn active_transport(&self) -> Option<SessionTransportBinding> {
        self.transport
    }

    pub const fn security(&self) -> Option<SessionSecurityContext> {
        self.security
    }

    pub fn events(&self) -> &[SessionEvent] {
        &self.events
    }

    pub fn io_actions(&self) -> &[SessionIoAction] {
        &self.io_actions
    }

    pub fn drain_io_actions(&mut self) -> Vec<SessionIoAction> {
        std::mem::take(&mut self.io_actions)
    }

    pub fn sync_established_session_gauge<'a, I>(sessions: I, observability: &mut Observability)
    where
        I: IntoIterator<Item = &'a SessionManager>,
    {
        let established_sessions = sessions
            .into_iter()
            .filter(|session| {
                matches!(
                    session.state,
                    SessionState::Established | SessionState::Degraded
                )
            })
            .count();
        observability.set_established_sessions(established_sessions);
    }

    pub fn begin_open(
        &mut self,
        timestamp_unix_ms: u64,
        transport: &dyn Transport,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::BeginOpen,
            matches!(self.state, SessionState::Idle | SessionState::Closed),
        )?;

        self.transport = Some(SessionTransportBinding {
            transport_class: transport.transport_class(),
            adapter_name: transport.adapter_name(),
            is_placeholder: transport.is_placeholder(),
        });
        self.security = None;
        self.timers = SessionTimerSchedule {
            open_deadline_unix_ms: Some(
                timestamp_unix_ms.saturating_add(self.timing.open_timeout_ms),
            ),
            ..SessionTimerSchedule::default()
        };
        self.queue_io_action(
            timestamp_unix_ms,
            SessionIoActionKind::BeginHandshake,
            Some("begin handshake on selected transport".to_string()),
        );

        Ok(self.record_event(
            timestamp_unix_ms,
            SessionEventKind::OpenStarted,
            SessionEventResult::Accepted,
            SessionState::Opening,
            None,
            Some("opening session over placeholder transport".to_string()),
        ))
    }

    pub fn handle_runner_input(
        &mut self,
        timestamp_unix_ms: u64,
        input: SessionRunnerInput,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::HandleRunnerInput,
            matches!(
                (self.state, &input),
                (
                    SessionState::Opening,
                    SessionRunnerInput::HandshakeSucceeded { .. }
                        | SessionRunnerInput::TransportClosed { .. }
                        | SessionRunnerInput::TransportFailed { .. }
                ) | (
                    SessionState::Established,
                    SessionRunnerInput::FrameReceived { .. }
                        | SessionRunnerInput::TransportClosed { .. }
                        | SessionRunnerInput::TransportFailed { .. }
                ) | (
                    SessionState::Degraded,
                    SessionRunnerInput::FrameReceived { .. }
                        | SessionRunnerInput::TransportClosed { .. }
                        | SessionRunnerInput::TransportFailed { .. }
                ) | (
                    SessionState::Closing,
                    SessionRunnerInput::TransportClosed { .. }
                        | SessionRunnerInput::TransportFailed { .. }
                )
            ),
        )?;

        match input {
            SessionRunnerInput::FrameReceived { byte_len } => self.record_activity(
                timestamp_unix_ms,
                Some(format!("runner delivered {byte_len} bytes")),
            ),
            SessionRunnerInput::HandshakeSucceeded { outcome } => {
                self.mark_established_with_handshake(timestamp_unix_ms, outcome)
            }
            SessionRunnerInput::TransportClosed { detail } => {
                self.handle_transport_closed(timestamp_unix_ms, detail)
            }
            SessionRunnerInput::TransportFailed { detail } => self.fail(timestamp_unix_ms, detail),
        }
    }

    pub fn handle_runner_input_with_replay_cache(
        &mut self,
        timestamp_unix_ms: u64,
        input: SessionRunnerInput,
        replay_cache: &mut ReplayCache,
    ) -> Result<SessionEvent, SessionError> {
        match input {
            SessionRunnerInput::HandshakeSucceeded { outcome } => self
                .mark_established_with_handshake_and_replay_cache(
                    timestamp_unix_ms,
                    outcome,
                    replay_cache,
                ),
            other => self.handle_runner_input(timestamp_unix_ms, other),
        }
    }

    pub fn mark_established(
        &mut self,
        timestamp_unix_ms: u64,
    ) -> Result<SessionEvent, SessionError> {
        self.mark_established_internal(timestamp_unix_ms, None)
    }

    pub fn mark_established_with_handshake(
        &mut self,
        timestamp_unix_ms: u64,
        outcome: HandshakeOutcome,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::MarkEstablished,
            self.state == SessionState::Opening,
        )?;
        self.security = Some(SessionSecurityContext::from(outcome));
        self.mark_established_internal(
            timestamp_unix_ms,
            Some("handshake bound to peer identity".to_string()),
        )
    }

    pub fn mark_established_with_handshake_and_replay_cache(
        &mut self,
        timestamp_unix_ms: u64,
        outcome: HandshakeOutcome,
        replay_cache: &mut ReplayCache,
    ) -> Result<SessionEvent, SessionError> {
        replay_cache.observe_outcome(&outcome, timestamp_unix_ms)?;
        self.mark_established_with_handshake(timestamp_unix_ms, outcome)
    }

    pub fn record_activity(
        &mut self,
        timestamp_unix_ms: u64,
        detail: Option<String>,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::RecordActivity,
            matches!(
                self.state,
                SessionState::Established | SessionState::Degraded
            ),
        )?;

        self.refresh_liveness_timers(timestamp_unix_ms);

        Ok(self.record_event(
            timestamp_unix_ms,
            SessionEventKind::ActivityObserved,
            SessionEventResult::Ok,
            self.state,
            None,
            detail,
        ))
    }

    pub fn mark_degraded(
        &mut self,
        timestamp_unix_ms: u64,
        detail: impl Into<String>,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::MarkDegraded,
            self.state == SessionState::Established,
        )?;

        self.schedule_degraded_timers(timestamp_unix_ms);

        Ok(self.record_event(
            timestamp_unix_ms,
            SessionEventKind::Degraded,
            SessionEventResult::Degraded,
            SessionState::Degraded,
            None,
            Some(detail.into()),
        ))
    }

    pub fn mark_recovered(
        &mut self,
        timestamp_unix_ms: u64,
        detail: impl Into<String>,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::MarkRecovered,
            self.state == SessionState::Degraded,
        )?;

        self.schedule_established_timers(timestamp_unix_ms);

        Ok(self.record_event(
            timestamp_unix_ms,
            SessionEventKind::Recovered,
            SessionEventResult::Ok,
            SessionState::Established,
            None,
            Some(detail.into()),
        ))
    }

    pub fn begin_close(
        &mut self,
        timestamp_unix_ms: u64,
        detail: Option<String>,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::BeginClose,
            matches!(
                self.state,
                SessionState::Opening | SessionState::Established | SessionState::Degraded
            ),
        )?;

        self.timers = SessionTimerSchedule {
            close_deadline_unix_ms: Some(
                timestamp_unix_ms.saturating_add(self.timing.close_timeout_ms),
            ),
            ..SessionTimerSchedule::default()
        };
        self.queue_io_action(
            timestamp_unix_ms,
            SessionIoActionKind::StartClose,
            detail.clone(),
        );

        Ok(self.record_event(
            timestamp_unix_ms,
            SessionEventKind::CloseStarted,
            SessionEventResult::Accepted,
            SessionState::Closing,
            None,
            detail,
        ))
    }

    pub fn mark_closed(&mut self, timestamp_unix_ms: u64) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::MarkClosed,
            self.state == SessionState::Closing,
        )?;

        let event = self.record_closed_event(timestamp_unix_ms, None);
        self.clear_runtime_state();

        Ok(event)
    }

    pub fn fail(
        &mut self,
        timestamp_unix_ms: u64,
        detail: impl Into<String>,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::Fail,
            matches!(
                self.state,
                SessionState::Opening
                    | SessionState::Established
                    | SessionState::Degraded
                    | SessionState::Closing
            ),
        )?;

        let event = self.record_event(
            timestamp_unix_ms,
            SessionEventKind::Failed,
            SessionEventResult::Error,
            SessionState::Closed,
            None,
            Some(detail.into()),
        );
        self.queue_io_action(
            timestamp_unix_ms,
            SessionIoActionKind::AbortTransport,
            event.detail.clone(),
        );
        self.clear_runtime_state();

        Ok(event)
    }

    pub fn poll_timers(
        &mut self,
        timestamp_unix_ms: u64,
    ) -> Result<Vec<SessionEvent>, SessionError> {
        let mut events = Vec::new();

        match self.state {
            SessionState::Idle | SessionState::Closed => {}
            SessionState::Opening => {
                if self.timer_due(self.timers.open_deadline_unix_ms, timestamp_unix_ms) {
                    events.push(self.timeout_and_close(
                        timestamp_unix_ms,
                        SessionTimerKind::Open,
                        "session open timeout exceeded",
                    ));
                }
            }
            SessionState::Established => {
                if self.timer_due(self.timers.idle_deadline_unix_ms, timestamp_unix_ms) {
                    self.schedule_degraded_timers(timestamp_unix_ms);
                    events.push(self.record_event(
                        timestamp_unix_ms,
                        SessionEventKind::Degraded,
                        SessionEventResult::Degraded,
                        SessionState::Degraded,
                        Some(SessionTimerKind::Idle),
                        Some("idle timeout exceeded; session degraded".to_string()),
                    ));
                } else if self.timer_due(self.timers.keepalive_due_unix_ms, timestamp_unix_ms) {
                    self.timers.keepalive_due_unix_ms =
                        Some(timestamp_unix_ms.saturating_add(self.timing.keepalive_interval_ms));
                    self.queue_io_action(
                        timestamp_unix_ms,
                        SessionIoActionKind::SendKeepalive,
                        Some("keepalive due".to_string()),
                    );
                    events.push(self.record_event(
                        timestamp_unix_ms,
                        SessionEventKind::KeepaliveDue,
                        SessionEventResult::Ok,
                        SessionState::Established,
                        Some(SessionTimerKind::Keepalive),
                        Some("keepalive due".to_string()),
                    ));
                }
            }
            SessionState::Degraded => {
                if self.timer_due(self.timers.degraded_deadline_unix_ms, timestamp_unix_ms) {
                    events.push(self.timeout_and_close(
                        timestamp_unix_ms,
                        SessionTimerKind::Degraded,
                        "degraded session timeout exceeded",
                    ));
                } else if self.timer_due(self.timers.keepalive_due_unix_ms, timestamp_unix_ms) {
                    self.timers.keepalive_due_unix_ms =
                        Some(timestamp_unix_ms.saturating_add(self.timing.keepalive_interval_ms));
                    self.queue_io_action(
                        timestamp_unix_ms,
                        SessionIoActionKind::SendKeepalive,
                        Some("keepalive due".to_string()),
                    );
                    events.push(self.record_event(
                        timestamp_unix_ms,
                        SessionEventKind::KeepaliveDue,
                        SessionEventResult::Ok,
                        SessionState::Degraded,
                        Some(SessionTimerKind::Keepalive),
                        Some("keepalive due".to_string()),
                    ));
                }
            }
            SessionState::Closing => {
                if self.timer_due(self.timers.close_deadline_unix_ms, timestamp_unix_ms) {
                    events.push(self.timeout_and_close(
                        timestamp_unix_ms,
                        SessionTimerKind::Close,
                        "session close timeout exceeded",
                    ));
                }
            }
        }

        Ok(events)
    }

    fn mark_established_internal(
        &mut self,
        timestamp_unix_ms: u64,
        detail: Option<String>,
    ) -> Result<SessionEvent, SessionError> {
        self.ensure_state(
            SessionAction::MarkEstablished,
            self.state == SessionState::Opening,
        )?;

        self.schedule_established_timers(timestamp_unix_ms);

        Ok(self.record_event(
            timestamp_unix_ms,
            SessionEventKind::OpenSucceeded,
            SessionEventResult::Ok,
            SessionState::Established,
            None,
            detail,
        ))
    }

    fn handle_transport_closed(
        &mut self,
        timestamp_unix_ms: u64,
        detail: Option<String>,
    ) -> Result<SessionEvent, SessionError> {
        match self.state {
            SessionState::Opening => self.fail(
                timestamp_unix_ms,
                detail
                    .unwrap_or_else(|| "transport closed before session establishment".to_string()),
            ),
            SessionState::Established | SessionState::Degraded | SessionState::Closing => {
                let event = self.record_closed_event(timestamp_unix_ms, detail);
                self.clear_runtime_state();
                Ok(event)
            }
            SessionState::Idle | SessionState::Closed => {
                Err(SessionError::InvalidStateTransition {
                    state: self.state,
                    action: SessionAction::HandleRunnerInput,
                })
            }
        }
    }

    fn ensure_state(&self, action: SessionAction, predicate: bool) -> Result<(), SessionError> {
        if predicate {
            return Ok(());
        }

        Err(SessionError::InvalidStateTransition {
            state: self.state,
            action,
        })
    }

    fn record_event(
        &mut self,
        timestamp_unix_ms: u64,
        event: SessionEventKind,
        result: SessionEventResult,
        new_state: SessionState,
        timer: Option<SessionTimerKind>,
        detail: Option<String>,
    ) -> SessionEvent {
        let session_event = SessionEvent {
            timestamp_unix_ms,
            node_id: self.node_id,
            peer_node_id: self.security.map(|security| security.peer_node_id),
            correlation_id: self.correlation_id,
            component: SESSION_COMPONENT,
            event,
            result,
            previous_state: self.state,
            new_state,
            transport: self.transport,
            timer,
            detail,
        };
        self.state = new_state;
        self.push_event(session_event.clone());
        session_event
    }

    fn record_closed_event(
        &mut self,
        timestamp_unix_ms: u64,
        detail: Option<String>,
    ) -> SessionEvent {
        self.record_event(
            timestamp_unix_ms,
            SessionEventKind::Closed,
            SessionEventResult::Ok,
            SessionState::Closed,
            None,
            detail,
        )
    }

    fn schedule_established_timers(&mut self, timestamp_unix_ms: u64) {
        self.timers = SessionTimerSchedule {
            keepalive_due_unix_ms: Some(
                timestamp_unix_ms.saturating_add(self.timing.keepalive_interval_ms),
            ),
            idle_deadline_unix_ms: Some(
                timestamp_unix_ms.saturating_add(self.timing.idle_timeout_ms),
            ),
            ..SessionTimerSchedule::default()
        };
    }

    fn schedule_degraded_timers(&mut self, timestamp_unix_ms: u64) {
        self.timers = SessionTimerSchedule {
            keepalive_due_unix_ms: Some(
                timestamp_unix_ms.saturating_add(self.timing.keepalive_interval_ms),
            ),
            degraded_deadline_unix_ms: Some(
                timestamp_unix_ms.saturating_add(self.timing.degraded_timeout_ms),
            ),
            ..SessionTimerSchedule::default()
        };
    }

    fn refresh_liveness_timers(&mut self, timestamp_unix_ms: u64) {
        self.timers.keepalive_due_unix_ms =
            Some(timestamp_unix_ms.saturating_add(self.timing.keepalive_interval_ms));

        match self.state {
            SessionState::Established => {
                self.timers.idle_deadline_unix_ms =
                    Some(timestamp_unix_ms.saturating_add(self.timing.idle_timeout_ms));
            }
            SessionState::Degraded => {
                self.timers.degraded_deadline_unix_ms =
                    Some(timestamp_unix_ms.saturating_add(self.timing.degraded_timeout_ms));
            }
            _ => {}
        }
    }

    fn timeout_and_close(
        &mut self,
        timestamp_unix_ms: u64,
        timer: SessionTimerKind,
        detail: &'static str,
    ) -> SessionEvent {
        self.queue_io_action(
            timestamp_unix_ms,
            SessionIoActionKind::AbortTransport,
            Some(detail.to_string()),
        );
        let event = self.record_event(
            timestamp_unix_ms,
            SessionEventKind::TimedOut,
            SessionEventResult::Timeout,
            SessionState::Closed,
            Some(timer),
            Some(detail.to_string()),
        );
        self.clear_runtime_state();
        event
    }

    fn queue_io_action(
        &mut self,
        timestamp_unix_ms: u64,
        action: SessionIoActionKind,
        detail: Option<String>,
    ) {
        self.push_io_action(SessionIoAction {
            timestamp_unix_ms,
            correlation_id: self.correlation_id,
            action,
            transport: self.transport,
            peer_node_id: self.security.map(|security| security.peer_node_id),
            detail,
        });
    }

    fn push_event(&mut self, event: SessionEvent) {
        push_bounded(&mut self.events, event, MAX_SESSION_EVENT_LOG_LEN);
    }

    fn push_io_action(&mut self, action: SessionIoAction) {
        push_bounded(
            &mut self.io_actions,
            action,
            MAX_SESSION_IO_ACTION_QUEUE_LEN,
        );
    }

    fn clear_runtime_state(&mut self) {
        self.timers = SessionTimerSchedule::default();
        self.transport = None;
        self.security = None;
    }

    fn timer_due(&self, deadline: Option<u64>, timestamp_unix_ms: u64) -> bool {
        matches!(deadline, Some(deadline) if timestamp_unix_ms >= deadline)
    }
}

fn push_bounded<T>(items: &mut Vec<T>, item: T, limit: usize) {
    if items.len() == limit {
        items.remove(0);
    }
    items.push(item);
}

#[cfg(test)]
mod tests {
    use super::{
        ReplayCache, ReplayCacheConfig, ReplayCacheError, SessionAction, SessionError,
        SessionEventKind, SessionEventResult, SessionIoActionKind, SessionManager,
        SessionRunnerInput, SessionState, SessionTimerKind, SessionTimingConfig,
        MAX_SESSION_EVENT_LOG_LEN, MAX_SESSION_IO_ACTION_QUEUE_LEN,
    };
    use crate::{
        crypto::{aead::ChaCha20Poly1305Key, hash::Blake3Digest},
        identity::NodeId,
        metrics::Observability,
        session::{HandshakeOutcome, SessionKeys},
        transport::{TcpTransport, TransportClass},
    };

    const TEST_TIMING: SessionTimingConfig = SessionTimingConfig {
        open_timeout_ms: 10,
        keepalive_interval_ms: 20,
        idle_timeout_ms: 60,
        degraded_timeout_ms: 30,
        close_timeout_ms: 15,
    };

    #[test]
    fn open_transition_moves_session_to_established() {
        let mut manager =
            SessionManager::with_timing(7, TEST_TIMING).expect("timing config should be valid");

        let open_started = manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        let open_succeeded = manager
            .mark_established(120)
            .expect("opening should transition to established");

        assert_eq!(manager.state(), SessionState::Established);
        assert_eq!(open_started.event, SessionEventKind::OpenStarted);
        assert_eq!(open_started.result, SessionEventResult::Accepted);
        assert_eq!(open_started.previous_state, SessionState::Idle);
        assert_eq!(open_started.new_state, SessionState::Opening);
        assert_eq!(
            open_started
                .transport
                .expect("transport binding should be present")
                .transport_class,
            TransportClass::Tcp
        );
        assert_eq!(open_succeeded.event, SessionEventKind::OpenSucceeded);
        assert_eq!(open_succeeded.result, SessionEventResult::Ok);
        assert_eq!(open_succeeded.previous_state, SessionState::Opening);
        assert_eq!(open_succeeded.new_state, SessionState::Established);
        assert_eq!(
            manager.timers().keepalive_due_unix_ms,
            Some(120 + TEST_TIMING.keepalive_interval_ms)
        );
        assert_eq!(
            manager.timers().idle_deadline_unix_ms,
            Some(120 + TEST_TIMING.idle_timeout_ms)
        );
        let io_actions = manager.drain_io_actions();
        assert_eq!(io_actions.len(), 1);
        assert_eq!(io_actions[0].action, SessionIoActionKind::BeginHandshake);
        assert_eq!(
            io_actions[0]
                .transport
                .expect("transport binding should be present")
                .transport_class,
            TransportClass::Tcp
        );
        assert!(manager.io_actions().is_empty());
    }

    #[test]
    fn established_session_can_bind_handshake_outcome() {
        let mut manager =
            SessionManager::with_timing(8, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");

        let open_succeeded = manager
            .mark_established_with_handshake(120, handshake_outcome())
            .expect("opening should bind handshake context");

        let security = manager
            .security()
            .expect("handshake security context should be present");
        assert_eq!(manager.state(), SessionState::Established);
        assert_eq!(security.peer_node_id, NodeId::from_bytes([9_u8; 32]));
        assert_eq!(security.transcript_hash, [7_u8; 32] as Blake3Digest);
        assert_eq!(
            open_succeeded.peer_node_id,
            Some(NodeId::from_bytes([9_u8; 32]))
        );
        assert_eq!(
            open_succeeded.detail.as_deref(),
            Some("handshake bound to peer identity")
        );
        assert_eq!(
            manager
                .drain_io_actions()
                .into_iter()
                .map(|action| action.action)
                .collect::<Vec<_>>(),
            vec![SessionIoActionKind::BeginHandshake]
        );
    }

    #[test]
    fn keepalive_due_emits_event_without_leaving_established_state() {
        let mut manager =
            SessionManager::with_timing(9, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");

        let events = manager
            .poll_timers(140)
            .expect("timer polling should succeed");

        assert_eq!(manager.state(), SessionState::Established);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, SessionEventKind::KeepaliveDue);
        assert_eq!(events[0].timer, Some(SessionTimerKind::Keepalive));
        assert_eq!(events[0].previous_state, SessionState::Established);
        assert_eq!(events[0].new_state, SessionState::Established);
        let io_actions = manager.drain_io_actions();
        assert_eq!(io_actions.len(), 2);
        assert_eq!(io_actions[1].action, SessionIoActionKind::SendKeepalive);
        assert_eq!(
            manager.timers().keepalive_due_unix_ms,
            Some(140 + TEST_TIMING.keepalive_interval_ms)
        );
    }

    #[test]
    fn activity_refreshes_established_deadlines() {
        let mut manager =
            SessionManager::with_timing(10, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");

        let observed = manager
            .record_activity(150, Some("frame received".to_string()))
            .expect("activity should refresh established timers");

        assert_eq!(observed.event, SessionEventKind::ActivityObserved);
        assert_eq!(observed.previous_state, SessionState::Established);
        assert_eq!(observed.new_state, SessionState::Established);
        assert_eq!(
            manager.timers().keepalive_due_unix_ms,
            Some(150 + TEST_TIMING.keepalive_interval_ms)
        );
        assert_eq!(
            manager.timers().idle_deadline_unix_ms,
            Some(150 + TEST_TIMING.idle_timeout_ms)
        );
    }

    #[test]
    fn degraded_transition_moves_established_session_to_degraded() {
        let mut manager =
            SessionManager::with_timing(11, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");

        let degraded = manager
            .mark_degraded(140, "probe loss exceeded threshold")
            .expect("established should transition to degraded");

        assert_eq!(manager.state(), SessionState::Degraded);
        assert_eq!(degraded.event, SessionEventKind::Degraded);
        assert_eq!(degraded.result, SessionEventResult::Degraded);
        assert_eq!(degraded.previous_state, SessionState::Established);
        assert_eq!(degraded.new_state, SessionState::Degraded);
        assert_eq!(
            degraded.detail.as_deref(),
            Some("probe loss exceeded threshold")
        );
        assert_eq!(
            manager.timers().degraded_deadline_unix_ms,
            Some(140 + TEST_TIMING.degraded_timeout_ms)
        );
    }

    #[test]
    fn recovery_transition_moves_degraded_session_back_to_established() {
        let mut manager =
            SessionManager::with_timing(111, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");
        manager
            .mark_degraded(140, "temporary loss spike")
            .expect("established should transition to degraded");

        let recovered = manager
            .mark_recovered(155, "path metrics stabilized")
            .expect("degraded should transition back to established");

        assert_eq!(manager.state(), SessionState::Established);
        assert_eq!(recovered.event, SessionEventKind::Recovered);
        assert_eq!(recovered.result, SessionEventResult::Ok);
        assert_eq!(recovered.previous_state, SessionState::Degraded);
        assert_eq!(recovered.new_state, SessionState::Established);
        assert_eq!(recovered.detail.as_deref(), Some("path metrics stabilized"));
        assert_eq!(
            manager.timers().keepalive_due_unix_ms,
            Some(155 + TEST_TIMING.keepalive_interval_ms)
        );
        assert_eq!(
            manager.timers().idle_deadline_unix_ms,
            Some(155 + TEST_TIMING.idle_timeout_ms)
        );
        assert_eq!(manager.timers().degraded_deadline_unix_ms, None);
        assert_eq!(
            manager
                .drain_io_actions()
                .into_iter()
                .map(|action| action.action)
                .collect::<Vec<_>>(),
            vec![SessionIoActionKind::BeginHandshake]
        );
    }

    #[test]
    fn idle_timeout_degrades_established_session() {
        let mut manager =
            SessionManager::with_timing(12, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");

        let events = manager
            .poll_timers(180)
            .expect("timer polling should succeed");

        assert_eq!(manager.state(), SessionState::Degraded);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, SessionEventKind::Degraded);
        assert_eq!(events[0].timer, Some(SessionTimerKind::Idle));
        assert_eq!(events[0].previous_state, SessionState::Established);
        assert_eq!(events[0].new_state, SessionState::Degraded);
    }

    #[test]
    fn close_transition_moves_session_to_closed() {
        let mut manager =
            SessionManager::with_timing(13, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");

        let close_started = manager
            .begin_close(160, Some("application requested close".to_string()))
            .expect("established should transition to closing");
        let closed = manager
            .mark_closed(180)
            .expect("closing should transition to closed");

        assert_eq!(manager.state(), SessionState::Closed);
        assert!(manager.active_transport().is_none());
        assert!(manager.security().is_none());
        assert_eq!(close_started.event, SessionEventKind::CloseStarted);
        assert_eq!(close_started.previous_state, SessionState::Established);
        assert_eq!(close_started.new_state, SessionState::Closing);
        assert_eq!(closed.event, SessionEventKind::Closed);
        assert_eq!(closed.result, SessionEventResult::Ok);
        assert_eq!(closed.previous_state, SessionState::Closing);
        assert_eq!(closed.new_state, SessionState::Closed);
        assert_eq!(
            manager
                .drain_io_actions()
                .into_iter()
                .map(|action| action.action)
                .collect::<Vec<_>>(),
            vec![
                SessionIoActionKind::BeginHandshake,
                SessionIoActionKind::StartClose,
            ]
        );
    }

    #[test]
    fn degraded_timeout_closes_session() {
        let mut manager =
            SessionManager::with_timing(14, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");
        manager
            .mark_degraded(140, "path quality degraded")
            .expect("established should transition to degraded");

        let events = manager
            .poll_timers(170)
            .expect("timer polling should succeed");

        assert_eq!(manager.state(), SessionState::Closed);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, SessionEventKind::TimedOut);
        assert_eq!(events[0].result, SessionEventResult::Timeout);
        assert_eq!(events[0].timer, Some(SessionTimerKind::Degraded));
        assert!(manager.active_transport().is_none());
        assert_eq!(
            manager
                .drain_io_actions()
                .into_iter()
                .map(|action| action.action)
                .collect::<Vec<_>>(),
            vec![
                SessionIoActionKind::BeginHandshake,
                SessionIoActionKind::AbortTransport,
            ]
        );
    }

    #[test]
    fn error_transition_closes_session_and_clears_transport() {
        let mut manager =
            SessionManager::with_timing(15, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");

        let failed = manager
            .fail(110, "handshake timed out")
            .expect("opening should transition to closed on failure");

        assert_eq!(manager.state(), SessionState::Closed);
        assert!(manager.active_transport().is_none());
        assert!(manager.security().is_none());
        assert_eq!(failed.event, SessionEventKind::Failed);
        assert_eq!(failed.result, SessionEventResult::Error);
        assert_eq!(failed.previous_state, SessionState::Opening);
        assert_eq!(failed.new_state, SessionState::Closed);
        assert_eq!(failed.detail.as_deref(), Some("handshake timed out"));
        assert_eq!(
            manager
                .drain_io_actions()
                .into_iter()
                .map(|action| action.action)
                .collect::<Vec<_>>(),
            vec![
                SessionIoActionKind::BeginHandshake,
                SessionIoActionKind::AbortTransport,
            ]
        );
    }

    #[test]
    fn open_timeout_closes_opening_session() {
        let mut manager =
            SessionManager::with_timing(16, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");

        let events = manager
            .poll_timers(110)
            .expect("timer polling should succeed");

        assert_eq!(manager.state(), SessionState::Closed);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, SessionEventKind::TimedOut);
        assert_eq!(events[0].timer, Some(SessionTimerKind::Open));
        assert_eq!(
            manager
                .drain_io_actions()
                .into_iter()
                .map(|action| action.action)
                .collect::<Vec<_>>(),
            vec![
                SessionIoActionKind::BeginHandshake,
                SessionIoActionKind::AbortTransport,
            ]
        );
    }

    #[test]
    fn rejects_invalid_timing_config() {
        let error = SessionManager::with_timing(
            17,
            SessionTimingConfig {
                keepalive_interval_ms: 20,
                idle_timeout_ms: 20,
                ..TEST_TIMING
            },
        )
        .expect_err("idle timeout must exceed keepalive interval");

        assert!(matches!(
            error,
            super::SessionError::IdleTimeoutNotAfterKeepalive {
                keepalive_interval_ms: 20,
                idle_timeout_ms: 20,
            }
        ));
    }

    #[test]
    fn rejects_recovery_from_non_degraded_state() {
        let mut manager =
            SessionManager::with_timing(171, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");

        let error = manager
            .mark_recovered(130, "should fail")
            .expect_err("only degraded sessions can recover");

        assert!(matches!(
            error,
            SessionError::InvalidStateTransition {
                state: SessionState::Established,
                action: SessionAction::MarkRecovered,
            }
        ));
        assert_eq!(manager.state(), SessionState::Established);
    }

    #[test]
    fn invalid_handshake_binding_does_not_leak_security_context() {
        let mut manager =
            SessionManager::with_timing(18, TEST_TIMING).expect("timing config should be valid");

        let error = manager
            .mark_established_with_handshake(120, handshake_outcome())
            .expect_err("idle sessions cannot bind handshake outcomes");

        assert!(matches!(
            error,
            SessionError::InvalidStateTransition {
                state: SessionState::Idle,
                action: SessionAction::MarkEstablished,
            }
        ));
        assert!(manager.security().is_none());
        assert!(manager.io_actions().is_empty());
    }

    #[test]
    fn runner_input_can_bind_handshake_and_track_frame_activity() {
        let mut manager =
            SessionManager::with_timing(19, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");

        let established = manager
            .handle_runner_input(
                120,
                SessionRunnerInput::HandshakeSucceeded {
                    outcome: handshake_outcome(),
                },
            )
            .expect("runner handshake result should establish the session");
        let observed = manager
            .handle_runner_input(150, SessionRunnerInput::FrameReceived { byte_len: 128 })
            .expect("runner frame input should refresh liveness");

        assert_eq!(established.event, SessionEventKind::OpenSucceeded);
        assert_eq!(manager.state(), SessionState::Established);
        assert_eq!(observed.event, SessionEventKind::ActivityObserved);
        assert_eq!(
            observed.detail.as_deref(),
            Some("runner delivered 128 bytes")
        );
    }

    #[test]
    fn runner_reported_transport_close_closes_established_session() {
        let mut manager =
            SessionManager::with_timing(20, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        manager
            .mark_established(120)
            .expect("opening should transition to established");

        let closed = manager
            .handle_runner_input(
                180,
                SessionRunnerInput::TransportClosed {
                    detail: Some("peer closed transport".to_string()),
                },
            )
            .expect("runner close should close the session");

        assert_eq!(manager.state(), SessionState::Closed);
        assert_eq!(closed.event, SessionEventKind::Closed);
        assert_eq!(closed.detail.as_deref(), Some("peer closed transport"));
        assert!(manager.active_transport().is_none());
    }

    #[test]
    fn runner_reported_transport_failure_closes_opening_session() {
        let mut manager =
            SessionManager::with_timing(21, TEST_TIMING).expect("timing config should be valid");
        manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");

        let failed = manager
            .handle_runner_input(
                110,
                SessionRunnerInput::TransportFailed {
                    detail: "dial failed".to_string(),
                },
            )
            .expect("runner failure should fail the session");

        assert_eq!(manager.state(), SessionState::Closed);
        assert_eq!(failed.event, SessionEventKind::Failed);
        assert_eq!(failed.detail.as_deref(), Some("dial failed"));
    }

    #[test]
    fn event_log_is_bounded() {
        let mut manager =
            SessionManager::with_timing(22, TEST_TIMING).expect("timing config should be valid");
        let cycles = MAX_SESSION_EVENT_LOG_LEN + 8;

        for cycle in 0..cycles {
            let timestamp = 100 + (cycle as u64) * 10;
            manager
                .begin_open(timestamp, &TcpTransport)
                .expect("open should transition from idle to opening");
            manager
                .fail(timestamp + 1, format!("failure-{cycle}"))
                .expect("failure should close the session");
        }

        assert_eq!(manager.events().len(), MAX_SESSION_EVENT_LOG_LEN);
        let dropped_cycles = ((2 * cycles) - MAX_SESSION_EVENT_LOG_LEN) / 2;
        let expected_last_failure = format!("failure-{}", cycles - 1);
        assert_eq!(manager.events()[0].event, SessionEventKind::OpenStarted);
        assert_eq!(
            manager.events()[0].timestamp_unix_ms,
            100 + (dropped_cycles as u64) * 10
        );
        assert_eq!(
            manager
                .events()
                .last()
                .expect("bounded event log should keep latest event")
                .detail
                .as_deref(),
            Some(expected_last_failure.as_str())
        );
        assert!(!manager
            .events()
            .iter()
            .any(|event| event.detail.as_deref() == Some("failure-0")));
    }

    #[test]
    fn io_action_queue_is_bounded_until_drained() {
        let mut manager =
            SessionManager::with_timing(23, TEST_TIMING).expect("timing config should be valid");
        let cycles = MAX_SESSION_IO_ACTION_QUEUE_LEN + 8;

        for cycle in 0..cycles {
            let timestamp = 100 + (cycle as u64) * 10;
            manager
                .begin_open(timestamp, &TcpTransport)
                .expect("open should transition from idle to opening");
            manager
                .fail(timestamp + 1, format!("failure-{cycle}"))
                .expect("failure should close the session");
        }

        assert_eq!(manager.io_actions().len(), MAX_SESSION_IO_ACTION_QUEUE_LEN);
        let dropped_cycles = ((2 * cycles) - MAX_SESSION_IO_ACTION_QUEUE_LEN) / 2;
        let expected_last_failure = format!("failure-{}", cycles - 1);
        assert_eq!(
            manager
                .io_actions()
                .first()
                .expect("bounded io queue should retain oldest surviving action")
                .action,
            SessionIoActionKind::BeginHandshake
        );
        assert_eq!(
            manager
                .io_actions()
                .first()
                .expect("bounded io queue should retain oldest surviving action")
                .timestamp_unix_ms,
            100 + (dropped_cycles as u64) * 10
        );
        assert_eq!(
            manager
                .io_actions()
                .first()
                .expect("bounded io queue should retain oldest surviving action")
                .detail
                .as_deref(),
            Some("begin handshake on selected transport")
        );
        assert_eq!(
            manager
                .io_actions()
                .last()
                .expect("bounded io queue should keep latest action")
                .detail
                .as_deref(),
            Some(expected_last_failure.as_str())
        );
        assert!(!manager
            .io_actions()
            .iter()
            .any(|action| action.detail.as_deref() == Some("failure-0")));
    }

    #[test]
    fn session_events_can_be_exported_to_observability() {
        let fallback_node_id = NodeId::from_bytes([55_u8; 32]);
        let mut manager =
            SessionManager::with_timing(24, TEST_TIMING).expect("timing config should be valid");
        let mut observability = Observability::default();

        let open_started = manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        let open_succeeded = manager
            .mark_established(120)
            .expect("opening should transition to established");
        let keepalive = manager
            .poll_timers(140)
            .expect("timer polling should succeed")
            .into_iter()
            .next()
            .expect("keepalive event should be emitted");

        open_started.record_with_observability(fallback_node_id, &mut observability);
        open_succeeded.record_with_observability(fallback_node_id, &mut observability);
        keepalive.record_with_observability(fallback_node_id, &mut observability);

        assert_eq!(observability.logs().len(), 3);
        assert_eq!(observability.logs()[0].node_id, fallback_node_id);
        let latest = observability.latest_log().expect("log should be present");
        assert_eq!(latest.event, "keepalive_due");
        assert_eq!(latest.result, "ok");
    }

    #[test]
    fn established_session_gauge_can_be_synced_explicitly() {
        let idle =
            SessionManager::with_timing(27, TEST_TIMING).expect("timing config should be valid");
        let mut established =
            SessionManager::with_timing(28, TEST_TIMING).expect("timing config should be valid");
        let mut degraded =
            SessionManager::with_timing(29, TEST_TIMING).expect("timing config should be valid");
        let mut closing =
            SessionManager::with_timing(30, TEST_TIMING).expect("timing config should be valid");
        let mut observability = Observability::default();

        established
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        established
            .mark_established(120)
            .expect("opening should transition to established");

        degraded
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        degraded
            .mark_established(120)
            .expect("opening should transition to established");
        degraded
            .mark_degraded(140, "temporary path loss")
            .expect("established session should degrade");

        closing
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        closing
            .mark_established(120)
            .expect("opening should transition to established");
        closing
            .begin_close(150, Some("operator requested close".to_string()))
            .expect("established session should begin closing");

        SessionManager::sync_established_session_gauge(
            [&idle, &established, &degraded, &closing],
            &mut observability,
        );

        assert_eq!(observability.metrics().established_sessions, 2);
    }

    #[test]
    fn replay_cache_rejects_zero_limits() {
        let error = ReplayCache::new(ReplayCacheConfig {
            max_entries: 0,
            replay_window_ms: 1,
        })
        .expect_err("zero replay cache entries must be rejected");

        assert_eq!(
            error,
            ReplayCacheError::ZeroLimit {
                field: "max_entries"
            }
        );
    }

    #[test]
    fn replay_cache_rejects_replayed_transcript_within_window() {
        let outcome = handshake_outcome();
        let mut cache = ReplayCache::new(ReplayCacheConfig {
            max_entries: 2,
            replay_window_ms: 100,
        })
        .expect("replay cache config should be valid");

        cache
            .observe_outcome(&outcome, 1_000)
            .expect("first observation should be accepted");
        let error = cache
            .observe_outcome(&outcome, 1_050)
            .expect_err("second observation within window should be rejected");

        assert_eq!(
            error,
            ReplayCacheError::ReplayDetected {
                peer_node_id: outcome.peer_node_id
            }
        );
    }

    #[test]
    fn replay_cache_prunes_expired_entries_and_evicts_oldest_when_bounded() {
        let first = handshake_outcome();
        let second = HandshakeOutcome {
            peer_node_id: NodeId::from_bytes([10_u8; 32]),
            transcript_hash: [8_u8; 32],
            session_keys: first.session_keys,
        };
        let third = HandshakeOutcome {
            peer_node_id: NodeId::from_bytes([11_u8; 32]),
            transcript_hash: [9_u8; 32],
            session_keys: first.session_keys,
        };
        let mut cache = ReplayCache::new(ReplayCacheConfig {
            max_entries: 2,
            replay_window_ms: 100,
        })
        .expect("replay cache config should be valid");

        cache
            .observe_outcome(&first, 1_000)
            .expect("first outcome should fit");
        cache
            .observe_outcome(&second, 1_001)
            .expect("second outcome should fit");
        cache
            .observe_outcome(&third, 1_002)
            .expect("third outcome should evict the oldest");
        assert_eq!(cache.observed_count(), 2);

        cache
            .observe_outcome(&first, 1_200)
            .expect("expired first outcome should be accepted again");
    }

    #[test]
    fn replay_cache_wrapper_rejects_replayed_handshake_outcome() {
        let outcome = handshake_outcome();
        let mut first_manager =
            SessionManager::with_timing(25, TEST_TIMING).expect("timing config should be valid");
        let mut second_manager =
            SessionManager::with_timing(26, TEST_TIMING).expect("timing config should be valid");
        let mut replay_cache = ReplayCache::new(ReplayCacheConfig {
            max_entries: 8,
            replay_window_ms: 300_000,
        })
        .expect("replay cache config should be valid");

        first_manager
            .begin_open(100, &TcpTransport)
            .expect("open should transition from idle to opening");
        first_manager
            .mark_established_with_handshake_and_replay_cache(120, outcome, &mut replay_cache)
            .expect("first handshake outcome should be accepted");

        second_manager
            .begin_open(130, &TcpTransport)
            .expect("open should transition from idle to opening");
        let error = second_manager
            .handle_runner_input_with_replay_cache(
                140,
                SessionRunnerInput::HandshakeSucceeded { outcome },
                &mut replay_cache,
            )
            .expect_err("replayed handshake outcome should be rejected");

        assert_eq!(
            error,
            SessionError::ReplayCache(ReplayCacheError::ReplayDetected {
                peer_node_id: outcome.peer_node_id,
            })
        );
        assert_eq!(second_manager.state(), SessionState::Opening);
        assert!(second_manager.security().is_none());
    }

    fn handshake_outcome() -> HandshakeOutcome {
        HandshakeOutcome {
            peer_node_id: NodeId::from_bytes([9_u8; 32]),
            transcript_hash: [7_u8; 32],
            session_keys: SessionKeys {
                client_to_server_key: ChaCha20Poly1305Key::from_bytes([1_u8; 32]),
                server_to_client_key: ChaCha20Poly1305Key::from_bytes([2_u8; 32]),
            },
        }
    }
}
