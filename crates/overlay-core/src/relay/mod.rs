//! Relay intro and fallback reachability baseline for Milestone 6.
//! Keep direct transport attempts first and use relay only as bounded fallback.

use std::collections::{BTreeMap, VecDeque};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::{FrameError, IntroTicketVerificationError, RecordValidationError},
    identity::NodeId,
    metrics::{LogComponent, LogContext, Observability},
    records::{FreshRecord, IntroTicket, PresenceRecord, RelayHint, VerifiedIntroTicket},
    transport::TransportClass,
    wire::{Message, MessageType, MAX_FRAME_BODY_LEN},
};

const RELAY_INTRO_WINDOW_S: u64 = 60;
const RELAY_BYTE_WINDOW_S: u64 = 60 * 60;

pub const TINY_MAX_CONCURRENT_RELAY_TUNNELS: usize = 2;
pub const TINY_MAX_INTRO_REQUESTS_PER_MINUTE: usize = 30;
pub const TINY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR: u64 = 64 * 1024 * 1024;
pub const TINY_MAX_TOTAL_RELAY_BYTES_PER_HOUR: u64 = 256 * 1024 * 1024;

pub const STANDARD_MAX_CONCURRENT_RELAY_TUNNELS: usize = 8;
pub const STANDARD_MAX_INTRO_REQUESTS_PER_MINUTE: usize = 120;
pub const STANDARD_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR: u64 = 256 * 1024 * 1024;
pub const STANDARD_MAX_TOTAL_RELAY_BYTES_PER_HOUR: u64 = 2 * 1024 * 1024 * 1024;

pub const RELAY_MAX_CONCURRENT_RELAY_TUNNELS: usize = 128;
pub const RELAY_MAX_INTRO_REQUESTS_PER_MINUTE: usize = 2_000;
pub const RELAY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR: u64 = 1024 * 1024 * 1024;
pub const RELAY_MAX_TOTAL_RELAY_BYTES_PER_HOUR: u64 = 64 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayMode {
    Forward,
    Intro,
    Rendezvous,
    Bridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayRolePolicy {
    pub forward: bool,
    pub intro: bool,
    pub rendezvous: bool,
    pub bridge: bool,
}

impl RelayRolePolicy {
    pub const fn disabled() -> Self {
        Self {
            forward: false,
            intro: false,
            rendezvous: false,
            bridge: false,
        }
    }

    pub const fn milestone6_default() -> Self {
        Self {
            forward: true,
            intro: true,
            rendezvous: false,
            bridge: false,
        }
    }

    pub const fn is_enabled(self, mode: RelayMode) -> bool {
        match mode {
            RelayMode::Forward => self.forward,
            RelayMode::Intro => self.intro,
            RelayMode::Rendezvous => self.rendezvous,
            RelayMode::Bridge => self.bridge,
        }
    }

    pub const fn any_enabled(self) -> bool {
        self.forward || self.intro || self.rendezvous || self.bridge
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayProfile {
    Tiny,
    Standard,
    Relay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayConfig {
    pub relay_mode: bool,
    pub role_policy: RelayRolePolicy,
    pub max_concurrent_relay_tunnels: usize,
    pub max_intro_requests_per_minute: usize,
    pub max_bytes_relayed_per_peer_per_hour: u64,
    pub max_total_relay_bytes_per_hour: u64,
}

impl RelayConfig {
    pub const fn for_profile(profile: RelayProfile) -> Self {
        match profile {
            RelayProfile::Tiny => Self {
                relay_mode: false,
                role_policy: RelayRolePolicy::disabled(),
                max_concurrent_relay_tunnels: TINY_MAX_CONCURRENT_RELAY_TUNNELS,
                max_intro_requests_per_minute: TINY_MAX_INTRO_REQUESTS_PER_MINUTE,
                max_bytes_relayed_per_peer_per_hour: TINY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
                max_total_relay_bytes_per_hour: TINY_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
            },
            RelayProfile::Standard => Self {
                relay_mode: false,
                role_policy: RelayRolePolicy::disabled(),
                max_concurrent_relay_tunnels: STANDARD_MAX_CONCURRENT_RELAY_TUNNELS,
                max_intro_requests_per_minute: STANDARD_MAX_INTRO_REQUESTS_PER_MINUTE,
                max_bytes_relayed_per_peer_per_hour: STANDARD_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
                max_total_relay_bytes_per_hour: STANDARD_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
            },
            RelayProfile::Relay => Self {
                relay_mode: true,
                role_policy: RelayRolePolicy::milestone6_default(),
                max_concurrent_relay_tunnels: RELAY_MAX_CONCURRENT_RELAY_TUNNELS,
                max_intro_requests_per_minute: RELAY_MAX_INTRO_REQUESTS_PER_MINUTE,
                max_bytes_relayed_per_peer_per_hour: RELAY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
                max_total_relay_bytes_per_hour: RELAY_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
            },
        }
    }

    pub const fn with_relay_mode(mut self, relay_mode: bool) -> Self {
        self.relay_mode = relay_mode;
        self.role_policy = if relay_mode {
            RelayRolePolicy::milestone6_default()
        } else {
            RelayRolePolicy::disabled()
        };
        self
    }

    pub fn validate(self) -> Result<Self, RelayError> {
        for (field, value) in [
            (
                "max_concurrent_relay_tunnels",
                self.max_concurrent_relay_tunnels as u64,
            ),
            (
                "max_intro_requests_per_minute",
                self.max_intro_requests_per_minute as u64,
            ),
            (
                "max_bytes_relayed_per_peer_per_hour",
                self.max_bytes_relayed_per_peer_per_hour,
            ),
            (
                "max_total_relay_bytes_per_hour",
                self.max_total_relay_bytes_per_hour,
            ),
        ] {
            if value == 0 {
                return Err(RelayError::ZeroLimit { field });
            }
        }

        if self.relay_mode && !self.role_policy.any_enabled() {
            return Err(RelayError::NoRelayModesEnabled);
        }

        Ok(self)
    }
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self::for_profile(RelayProfile::Standard)
    }
}

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("relay config limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
    #[error("relay mode is disabled for this local profile")]
    RelayDisabled,
    #[error("relay mode is enabled but no relay roles are allowed locally")]
    NoRelayModesEnabled,
    #[error("relay role '{mode:?}' is disabled for this local profile")]
    RelayModeDisabled { mode: RelayMode },
    #[error(
        "relay bind would exceed max_concurrent_relay_tunnels ({max_concurrent_relay_tunnels})"
    )]
    TunnelQuotaExceeded { max_concurrent_relay_tunnels: usize },
    #[error(
        "relay intro request rate would exceed max_intro_requests_per_minute ({max_intro_requests_per_minute})"
    )]
    IntroRateExceeded {
        max_intro_requests_per_minute: usize,
    },
    #[error(
        "relay byte usage for peer {peer_node_id} would exceed max_bytes_relayed_per_peer_per_hour ({max_bytes_relayed_per_peer_per_hour})"
    )]
    PerPeerByteQuotaExceeded {
        peer_node_id: NodeId,
        max_bytes_relayed_per_peer_per_hour: u64,
    },
    #[error(
        "total relay byte usage would exceed max_total_relay_bytes_per_hour ({max_total_relay_bytes_per_hour})"
    )]
    TotalByteQuotaExceeded { max_total_relay_bytes_per_hour: u64 },
    #[error("unknown transport class in relay planning input: {value}")]
    UnknownTransportClass { value: String },
    #[error(
        "relay hint for relay {relay_node_id} uses transport class 'relay'; nested relay fallback is out of scope for Milestone 6"
    )]
    RelayChainingNotSupported { relay_node_id: NodeId },
    #[error(
        "intro ticket target_node_id {actual_target_node_id} does not match requested target_node_id {expected_target_node_id}"
    )]
    TicketTargetMismatch {
        expected_target_node_id: NodeId,
        actual_target_node_id: NodeId,
    },
    #[error("intro ticket requester_binding does not match the local requester binding")]
    RequesterBindingMismatch,
    #[error(transparent)]
    RecordValidation(#[from] RecordValidationError),
}

#[derive(Debug, Error)]
pub enum RelayMessageError {
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Frame(#[from] FrameError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveIntro {
    pub relay_node_id: NodeId,
    pub intro_ticket: IntroTicket,
}

impl ResolveIntro {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RelayMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RelayMessageError> {
        parse_message_bytes(bytes)
    }

    pub fn verify_with_public_key(
        self,
        signer_public_key: &crate::crypto::sign::Ed25519PublicKey,
    ) -> Result<VerifiedResolveIntro, IntroTicketVerificationError> {
        Ok(VerifiedResolveIntro {
            relay_node_id: self.relay_node_id,
            intro_ticket: self
                .intro_ticket
                .verify_with_public_key(signer_public_key)?,
        })
    }

    pub fn verify_with_trusted_node_record(
        self,
        node_record: &crate::records::NodeRecord,
    ) -> Result<VerifiedResolveIntro, IntroTicketVerificationError> {
        Ok(VerifiedResolveIntro {
            relay_node_id: self.relay_node_id,
            intro_ticket: self
                .intro_ticket
                .verify_with_trusted_node_record(node_record)?,
        })
    }
}

impl Message for ResolveIntro {
    const TYPE: MessageType = MessageType::ResolveIntro;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedResolveIntro {
    relay_node_id: NodeId,
    intro_ticket: VerifiedIntroTicket,
}

impl VerifiedResolveIntro {
    pub const fn relay_node_id(&self) -> NodeId {
        self.relay_node_id
    }

    pub fn intro_ticket(&self) -> &IntroTicket {
        self.intro_ticket.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntroResponseStatus {
    Forwarded,
    RejectedRelayDisabled,
    RejectedRelayMismatch,
    RejectedRoleDisabled,
    RejectedTicketExpired,
    RejectedRequesterBinding,
    RejectedRateLimited,
}

impl IntroResponseStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Forwarded => "forwarded",
            Self::RejectedRelayDisabled => "rejected_relay_disabled",
            Self::RejectedRelayMismatch => "rejected_relay_mismatch",
            Self::RejectedRoleDisabled => "rejected_role_disabled",
            Self::RejectedTicketExpired => "rejected_ticket_expired",
            Self::RejectedRequesterBinding => "rejected_requester_binding",
            Self::RejectedRateLimited => "rejected_rate_limited",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntroResponse {
    pub relay_node_id: NodeId,
    pub target_node_id: NodeId,
    pub ticket_id: Vec<u8>,
    pub status: IntroResponseStatus,
}

impl IntroResponse {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RelayMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RelayMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for IntroResponse {
    const TYPE: MessageType = MessageType::IntroResponse;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayFallbackCandidate {
    pub relay_node_id: NodeId,
    pub relay_transport_class: TransportClass,
    pub relay_score: u32,
    pub ticket_id: Vec<u8>,
    pub target_node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachabilityPlan {
    pub target_node_id: NodeId,
    pub direct_attempts: Vec<TransportClass>,
    pub relay_fallbacks: Vec<RelayFallbackCandidate>,
}

impl ReachabilityPlan {
    pub fn relay_fallback_count(&self) -> usize {
        self.relay_fallbacks.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayTunnel {
    pub tunnel_id: u64,
    pub relay_node_id: NodeId,
    pub target_node_id: NodeId,
    pub opened_at_unix_s: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RelayUsageSnapshot {
    pub active_tunnels: usize,
    pub recent_intro_requests: usize,
    pub tracked_relay_peers: usize,
    pub total_relayed_bytes_last_hour: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RelayCleanupSummary {
    pub stale_tunnels_pruned: usize,
    pub expired_intro_requests_pruned: usize,
    pub stale_peer_byte_windows_pruned: usize,
    pub stale_total_byte_samples_pruned: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayManager {
    config: RelayConfig,
    intro_requests: VecDeque<u64>,
    active_tunnels: BTreeMap<u64, RelayTunnel>,
    peer_byte_windows: BTreeMap<NodeId, VecDeque<RelayByteSample>>,
    total_byte_window: VecDeque<RelayByteSample>,
}

impl RelayManager {
    pub fn new(config: RelayConfig) -> Result<Self, RelayError> {
        Ok(Self {
            config: config.validate()?,
            intro_requests: VecDeque::new(),
            active_tunnels: BTreeMap::new(),
            peer_byte_windows: BTreeMap::new(),
            total_byte_window: VecDeque::new(),
        })
    }

    pub const fn config(&self) -> RelayConfig {
        self.config
    }

    pub fn active_tunnel_count(&self) -> usize {
        self.active_tunnels.len()
    }

    pub fn usage_snapshot(&self) -> RelayUsageSnapshot {
        RelayUsageSnapshot {
            active_tunnels: self.active_tunnels.len(),
            recent_intro_requests: self.intro_requests.len(),
            tracked_relay_peers: self.peer_byte_windows.len(),
            total_relayed_bytes_last_hour: self.used_total_bytes(),
        }
    }

    pub fn prune_stale_state(
        &mut self,
        now_unix_s: u64,
        max_tunnel_age_s: u64,
    ) -> RelayCleanupSummary {
        let intro_before = self.intro_requests.len();
        prune_recent_timestamps(&mut self.intro_requests, now_unix_s, RELAY_INTRO_WINDOW_S);

        let peer_windows_before = self.peer_byte_windows.len();
        let total_samples_before = self.total_byte_window.len();
        self.prune_byte_windows(now_unix_s);

        let tunnels_before = self.active_tunnels.len();
        self.active_tunnels.retain(|_, tunnel| {
            now_unix_s.saturating_sub(tunnel.opened_at_unix_s) < max_tunnel_age_s
        });

        RelayCleanupSummary {
            stale_tunnels_pruned: tunnels_before.saturating_sub(self.active_tunnels.len()),
            expired_intro_requests_pruned: intro_before.saturating_sub(self.intro_requests.len()),
            stale_peer_byte_windows_pruned: peer_windows_before
                .saturating_sub(self.peer_byte_windows.len()),
            stale_total_byte_samples_pruned: total_samples_before
                .saturating_sub(self.total_byte_window.len()),
        }
    }

    pub fn note_intro_request(&mut self, now_unix_s: u64) -> Result<(), RelayError> {
        self.ensure_relay_mode()?;
        self.ensure_role(RelayMode::Intro)?;
        prune_recent_timestamps(&mut self.intro_requests, now_unix_s, RELAY_INTRO_WINDOW_S);
        if self.intro_requests.len() == self.config.max_intro_requests_per_minute {
            return Err(RelayError::IntroRateExceeded {
                max_intro_requests_per_minute: self.config.max_intro_requests_per_minute,
            });
        }

        self.intro_requests.push_back(now_unix_s);
        Ok(())
    }

    pub fn process_resolve_intro(
        &mut self,
        local_relay_node_id: NodeId,
        request: VerifiedResolveIntro,
        expected_requester_binding: &[u8],
        now_unix_s: u64,
    ) -> IntroResponse {
        let ticket = request.intro_ticket();
        let mut response = IntroResponse {
            relay_node_id: request.relay_node_id(),
            target_node_id: ticket.target_node_id,
            ticket_id: ticket.ticket_id.clone(),
            status: IntroResponseStatus::Forwarded,
        };

        if request.relay_node_id() != local_relay_node_id {
            response.status = IntroResponseStatus::RejectedRelayMismatch;
            return response;
        }
        if !self.config.relay_mode {
            response.status = IntroResponseStatus::RejectedRelayDisabled;
            return response;
        }
        if !self.config.role_policy.is_enabled(RelayMode::Intro) {
            response.status = IntroResponseStatus::RejectedRoleDisabled;
            return response;
        }
        if ticket.validate_freshness(now_unix_s).is_err() {
            response.status = IntroResponseStatus::RejectedTicketExpired;
            return response;
        }
        if ticket.requester_binding != expected_requester_binding {
            response.status = IntroResponseStatus::RejectedRequesterBinding;
            return response;
        }

        prune_recent_timestamps(&mut self.intro_requests, now_unix_s, RELAY_INTRO_WINDOW_S);
        if self.intro_requests.len() == self.config.max_intro_requests_per_minute {
            response.status = IntroResponseStatus::RejectedRateLimited;
            return response;
        }

        self.intro_requests.push_back(now_unix_s);
        response
    }

    pub fn process_resolve_intro_with_observability(
        &mut self,
        local_relay_node_id: NodeId,
        request: VerifiedResolveIntro,
        expected_requester_binding: &[u8],
        now_unix_s: u64,
        observability: &mut Observability,
        context: LogContext,
    ) -> IntroResponse {
        let response = self.process_resolve_intro(
            local_relay_node_id,
            request,
            expected_requester_binding,
            now_unix_s,
        );
        if response.status == IntroResponseStatus::RejectedRateLimited {
            observability.note_rate_limited_drop();
        }
        observability.push_log(
            context,
            LogComponent::Relay,
            "resolve_intro",
            response.status.as_str(),
        );
        response
    }

    pub fn bind_tunnel(
        &mut self,
        tunnel_id: u64,
        relay_node_id: NodeId,
        target_node_id: NodeId,
        now_unix_s: u64,
    ) -> Result<RelayTunnel, RelayError> {
        self.ensure_relay_mode()?;
        self.ensure_role(RelayMode::Forward)?;
        if !self.active_tunnels.contains_key(&tunnel_id)
            && self.active_tunnels.len() == self.config.max_concurrent_relay_tunnels
        {
            return Err(RelayError::TunnelQuotaExceeded {
                max_concurrent_relay_tunnels: self.config.max_concurrent_relay_tunnels,
            });
        }

        let tunnel = RelayTunnel {
            tunnel_id,
            relay_node_id,
            target_node_id,
            opened_at_unix_s: now_unix_s,
        };
        self.active_tunnels.insert(tunnel_id, tunnel);
        Ok(tunnel)
    }

    pub fn bind_tunnel_with_observability(
        &mut self,
        tunnel_id: u64,
        relay_node_id: NodeId,
        target_node_id: NodeId,
        now_unix_s: u64,
        observability: &mut Observability,
        context: LogContext,
    ) -> Result<RelayTunnel, RelayError> {
        match self.bind_tunnel(tunnel_id, relay_node_id, target_node_id, now_unix_s) {
            Ok(tunnel) => {
                observability.note_relay_bind();
                observability.push_log(context, LogComponent::Relay, "bind_tunnel", "opened");
                Ok(tunnel)
            }
            Err(error) => {
                observability.push_log(context, LogComponent::Relay, "bind_tunnel", "rejected");
                Err(error)
            }
        }
    }

    pub fn release_tunnel(&mut self, tunnel_id: u64) -> Option<RelayTunnel> {
        self.active_tunnels.remove(&tunnel_id)
    }

    pub fn note_relayed_bytes(
        &mut self,
        peer_node_id: NodeId,
        byte_len: u64,
        now_unix_s: u64,
    ) -> Result<(), RelayError> {
        self.ensure_relay_mode()?;
        self.ensure_role(RelayMode::Forward)?;
        self.prune_byte_windows(now_unix_s);

        let peer_bytes = self.used_bytes_for_peer(&peer_node_id);
        if peer_bytes.saturating_add(byte_len) > self.config.max_bytes_relayed_per_peer_per_hour {
            return Err(RelayError::PerPeerByteQuotaExceeded {
                peer_node_id,
                max_bytes_relayed_per_peer_per_hour: self
                    .config
                    .max_bytes_relayed_per_peer_per_hour,
            });
        }

        let total_bytes = self.used_total_bytes();
        if total_bytes.saturating_add(byte_len) > self.config.max_total_relay_bytes_per_hour {
            return Err(RelayError::TotalByteQuotaExceeded {
                max_total_relay_bytes_per_hour: self.config.max_total_relay_bytes_per_hour,
            });
        }

        let sample = RelayByteSample {
            timestamp_unix_s: now_unix_s,
            byte_len,
        };
        self.peer_byte_windows
            .entry(peer_node_id)
            .or_default()
            .push_back(sample);
        self.total_byte_window.push_back(sample);
        Ok(())
    }

    fn ensure_relay_mode(&self) -> Result<(), RelayError> {
        if self.config.relay_mode {
            return Ok(());
        }

        Err(RelayError::RelayDisabled)
    }

    fn ensure_role(&self, mode: RelayMode) -> Result<(), RelayError> {
        if self.config.role_policy.is_enabled(mode) {
            return Ok(());
        }

        Err(RelayError::RelayModeDisabled { mode })
    }

    fn prune_byte_windows(&mut self, now_unix_s: u64) {
        for window in self.peer_byte_windows.values_mut() {
            prune_recent_samples(window, now_unix_s, RELAY_BYTE_WINDOW_S);
        }
        self.peer_byte_windows
            .retain(|_, window| !window.is_empty());
        prune_recent_samples(&mut self.total_byte_window, now_unix_s, RELAY_BYTE_WINDOW_S);
    }

    fn used_bytes_for_peer(&self, peer_node_id: &NodeId) -> u64 {
        self.peer_byte_windows
            .get(peer_node_id)
            .map(|window| window.iter().map(|sample| sample.byte_len).sum())
            .unwrap_or(0)
    }

    fn used_total_bytes(&self) -> u64 {
        self.total_byte_window
            .iter()
            .map(|sample| sample.byte_len)
            .sum()
    }
}

pub fn build_reachability_plan(
    target_presence: &PresenceRecord,
    relay_hints: &[RelayHint],
    intro_ticket: &VerifiedIntroTicket,
    expected_requester_binding: &[u8],
    now_unix_s: u64,
) -> Result<ReachabilityPlan, RelayError> {
    let ticket = intro_ticket.as_ref();
    ticket.validate_freshness(now_unix_s)?;
    if ticket.target_node_id != target_presence.node_id {
        return Err(RelayError::TicketTargetMismatch {
            expected_target_node_id: target_presence.node_id,
            actual_target_node_id: ticket.target_node_id,
        });
    }
    if ticket.requester_binding != expected_requester_binding {
        return Err(RelayError::RequesterBindingMismatch);
    }

    let mut direct_attempts = Vec::new();
    for transport_class in &target_presence.transport_classes {
        let parsed = parse_transport_class(transport_class)?;
        if parsed != TransportClass::Relay && !direct_attempts.contains(&parsed) {
            direct_attempts.push(parsed);
        }
    }

    let mut relay_fallbacks = relay_hints
        .iter()
        .filter(|hint| hint.is_fresh(now_unix_s))
        .map(|hint| {
            let relay_transport_class = parse_transport_class(&hint.relay_transport_class)?;
            if relay_transport_class == TransportClass::Relay {
                return Err(RelayError::RelayChainingNotSupported {
                    relay_node_id: hint.relay_node_id,
                });
            }

            Ok(RelayFallbackCandidate {
                relay_node_id: hint.relay_node_id,
                relay_transport_class,
                relay_score: hint.relay_score,
                ticket_id: ticket.ticket_id.clone(),
                target_node_id: ticket.target_node_id,
            })
        })
        .collect::<Result<Vec<_>, RelayError>>()?;

    relay_fallbacks.sort_by(|left, right| {
        right
            .relay_score
            .cmp(&left.relay_score)
            .then_with(|| left.relay_node_id.cmp(&right.relay_node_id))
            .then_with(|| {
                left.relay_transport_class
                    .as_str()
                    .cmp(right.relay_transport_class.as_str())
            })
    });
    relay_fallbacks.dedup_by(|left, right| {
        left.relay_node_id == right.relay_node_id
            && left.relay_transport_class == right.relay_transport_class
    });

    Ok(ReachabilityPlan {
        target_node_id: target_presence.node_id,
        direct_attempts,
        relay_fallbacks,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RelayByteSample {
    timestamp_unix_s: u64,
    byte_len: u64,
}

fn parse_transport_class(value: &str) -> Result<TransportClass, RelayError> {
    match value {
        "tcp" => Ok(TransportClass::Tcp),
        "quic" => Ok(TransportClass::Quic),
        "ws" => Ok(TransportClass::Ws),
        "relay" => Ok(TransportClass::Relay),
        _ => Err(RelayError::UnknownTransportClass {
            value: value.to_string(),
        }),
    }
}

fn prune_recent_timestamps(window: &mut VecDeque<u64>, now_unix_s: u64, horizon_s: u64) {
    while let Some(timestamp_unix_s) = window.front().copied() {
        if now_unix_s.saturating_sub(timestamp_unix_s) < horizon_s {
            break;
        }
        window.pop_front();
    }
}

fn prune_recent_samples(window: &mut VecDeque<RelayByteSample>, now_unix_s: u64, horizon_s: u64) {
    while let Some(sample) = window.front().copied() {
        if now_unix_s.saturating_sub(sample.timestamp_unix_s) < horizon_s {
            break;
        }
        window.pop_front();
    }
}

fn canonical_message_bytes<T>(message: &T) -> Result<Vec<u8>, RelayMessageError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(message)?;
    validate_message_body_len(bytes.len())?;
    Ok(bytes)
}

fn parse_message_bytes<T>(bytes: &[u8]) -> Result<T, RelayMessageError>
where
    T: DeserializeOwned,
{
    validate_message_body_len(bytes.len())?;
    serde_json::from_slice(bytes).map_err(Into::into)
}

fn validate_message_body_len(body_len: usize) -> Result<(), RelayMessageError> {
    let body_len = u32::try_from(body_len).unwrap_or(u32::MAX);
    if body_len > MAX_FRAME_BODY_LEN {
        return Err(FrameError::BodyTooLarge {
            body_len,
            max_body_len: MAX_FRAME_BODY_LEN,
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde::Deserialize;

    use super::{
        build_reachability_plan, IntroResponse, IntroResponseStatus, RelayConfig, RelayError,
        RelayManager, RelayMessageError, RelayMode, RelayProfile, RelayRolePolicy, ResolveIntro,
        RELAY_MAX_CONCURRENT_RELAY_TUNNELS, RELAY_MAX_INTRO_REQUESTS_PER_MINUTE,
        RELAY_MAX_TOTAL_RELAY_BYTES_PER_HOUR, STANDARD_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
        STANDARD_MAX_CONCURRENT_RELAY_TUNNELS, STANDARD_MAX_INTRO_REQUESTS_PER_MINUTE,
        STANDARD_MAX_TOTAL_RELAY_BYTES_PER_HOUR, TINY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
        TINY_MAX_CONCURRENT_RELAY_TUNNELS, TINY_MAX_INTRO_REQUESTS_PER_MINUTE,
        TINY_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
    };
    use crate::{
        crypto::sign::Ed25519SigningKey,
        error::{FrameError, RecordValidationError},
        identity::{derive_node_id, NodeId},
        metrics::{LogContext, Observability},
        records::{IntroTicket, NodeRecord, PresenceRecord, RelayHint},
        transport::TransportClass,
        wire::{Message, MessageType, MAX_FRAME_BODY_LEN},
    };

    #[test]
    fn relay_profile_defaults_match_open_questions() {
        let tiny = RelayConfig::for_profile(RelayProfile::Tiny);
        assert_eq!(
            tiny.max_concurrent_relay_tunnels,
            TINY_MAX_CONCURRENT_RELAY_TUNNELS
        );
        assert_eq!(
            tiny.max_intro_requests_per_minute,
            TINY_MAX_INTRO_REQUESTS_PER_MINUTE
        );
        assert_eq!(
            tiny.max_bytes_relayed_per_peer_per_hour,
            TINY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR
        );
        assert_eq!(
            tiny.max_total_relay_bytes_per_hour,
            TINY_MAX_TOTAL_RELAY_BYTES_PER_HOUR
        );
        assert!(!tiny.relay_mode);
        assert_eq!(tiny.role_policy, RelayRolePolicy::disabled());

        let standard = RelayConfig::for_profile(RelayProfile::Standard);
        assert_eq!(
            standard.max_concurrent_relay_tunnels,
            STANDARD_MAX_CONCURRENT_RELAY_TUNNELS
        );
        assert_eq!(
            standard.max_intro_requests_per_minute,
            STANDARD_MAX_INTRO_REQUESTS_PER_MINUTE
        );
        assert_eq!(
            standard.max_bytes_relayed_per_peer_per_hour,
            STANDARD_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR
        );
        assert_eq!(
            standard.max_total_relay_bytes_per_hour,
            STANDARD_MAX_TOTAL_RELAY_BYTES_PER_HOUR
        );
        assert!(!standard.relay_mode);
        assert_eq!(standard.role_policy, RelayRolePolicy::disabled());

        let relay = RelayConfig::for_profile(RelayProfile::Relay);
        assert_eq!(
            relay.max_concurrent_relay_tunnels,
            RELAY_MAX_CONCURRENT_RELAY_TUNNELS
        );
        assert_eq!(
            relay.max_intro_requests_per_minute,
            RELAY_MAX_INTRO_REQUESTS_PER_MINUTE
        );
        assert_eq!(
            relay.max_total_relay_bytes_per_hour,
            RELAY_MAX_TOTAL_RELAY_BYTES_PER_HOUR
        );
        assert!(relay.relay_mode);
        assert_eq!(relay.role_policy, RelayRolePolicy::milestone6_default());
    }

    #[test]
    fn relay_intro_messages_expose_expected_wire_types_and_round_trip() {
        let target_signing_key = Ed25519SigningKey::from_seed([31_u8; 32]);
        let relay_node_id = NodeId::from_bytes([7_u8; 32]);
        let request = ResolveIntro {
            relay_node_id,
            intro_ticket: verified_intro_ticket(
                &target_signing_key,
                b"requester-binding",
                1_700_000_600,
            )
            .into_inner(),
        };
        assert_eq!(ResolveIntro::TYPE, MessageType::ResolveIntro);
        assert_eq!(
            ResolveIntro::from_canonical_bytes(
                &request
                    .canonical_bytes()
                    .expect("resolve intro should serialize")
            )
            .expect("resolve intro should deserialize"),
            request
        );

        let response = IntroResponse {
            relay_node_id,
            target_node_id: derive_node_id(target_signing_key.public_key().as_bytes()),
            ticket_id: vec![1_u8, 2, 3, 4],
            status: IntroResponseStatus::Forwarded,
        };
        assert_eq!(IntroResponse::TYPE, MessageType::IntroResponse);
        assert_eq!(
            IntroResponse::from_canonical_bytes(
                &response
                    .canonical_bytes()
                    .expect("intro response should serialize")
            )
            .expect("intro response should deserialize"),
            response
        );
    }

    #[test]
    fn resolve_intro_rejects_messages_larger_than_mvp_frame_limit() {
        let target_signing_key = Ed25519SigningKey::from_seed([32_u8; 32]);
        let relay_node_id = NodeId::from_bytes([8_u8; 32]);
        let mut intro_ticket =
            verified_intro_ticket(&target_signing_key, b"requester-binding", 1_700_000_600)
                .into_inner();
        intro_ticket.requester_binding = vec![0x55; MAX_FRAME_BODY_LEN as usize];

        let error = ResolveIntro {
            relay_node_id,
            intro_ticket,
        }
        .canonical_bytes()
        .expect_err("oversized resolve-intro request should be rejected");

        assert!(matches!(
            error,
            RelayMessageError::Frame(FrameError::BodyTooLarge {
                max_body_len: MAX_FRAME_BODY_LEN,
                ..
            })
        ));
    }

    #[test]
    fn resolve_intro_handoff_can_use_trusted_node_record() {
        let signing_key = Ed25519SigningKey::from_seed([32_u8; 32]);
        let public_key = signing_key.public_key();
        let request = ResolveIntro {
            relay_node_id: NodeId::from_bytes([8_u8; 32]),
            intro_ticket: verified_intro_ticket(&signing_key, b"requester-binding", 1_700_000_600)
                .into_inner(),
        };
        let trusted_node_record = NodeRecord {
            version: 1,
            node_id: derive_node_id(public_key.as_bytes()),
            node_public_key: public_key.as_bytes().to_vec(),
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["tcp".to_string()],
            supported_kex: vec!["x25519".to_string()],
            supported_signatures: vec!["ed25519".to_string()],
            anti_sybil_proof: Vec::new(),
            signature: vec![1_u8, 2, 3],
        };

        let verified = request
            .clone()
            .verify_with_trusted_node_record(&trusted_node_record)
            .expect("trusted node record should verify resolve intro request");

        assert_eq!(verified.relay_node_id(), request.relay_node_id);
        assert_eq!(verified.intro_ticket(), &request.intro_ticket);
    }

    #[test]
    fn process_resolve_intro_returns_forwarded_status_for_verified_request() {
        let target_signing_key = Ed25519SigningKey::from_seed([33_u8; 32]);
        let relay_node_id = NodeId::from_bytes([7_u8; 32]);
        let requester_binding = b"requester-binding";
        let request = ResolveIntro {
            relay_node_id,
            intro_ticket: verified_intro_ticket(
                &target_signing_key,
                requester_binding,
                1_700_000_600,
            )
            .into_inner(),
        }
        .verify_with_public_key(&target_signing_key.public_key())
        .expect("resolve intro request should verify");
        let mut manager = RelayManager::new(RelayConfig {
            relay_mode: true,
            role_policy: RelayRolePolicy::milestone6_default(),
            max_concurrent_relay_tunnels: 1,
            max_intro_requests_per_minute: 1,
            max_bytes_relayed_per_peer_per_hour: 32,
            max_total_relay_bytes_per_hour: 40,
        })
        .expect("relay config should be valid");

        let response =
            manager.process_resolve_intro(relay_node_id, request, requester_binding, 1_700_000_000);

        assert_eq!(response.relay_node_id, relay_node_id);
        assert_eq!(response.status, IntroResponseStatus::Forwarded);
    }

    #[test]
    fn process_resolve_intro_rejects_mismatched_relay_and_rate_limit() {
        let target_signing_key = Ed25519SigningKey::from_seed([34_u8; 32]);
        let relay_node_id = NodeId::from_bytes([7_u8; 32]);
        let other_relay_node_id = NodeId::from_bytes([8_u8; 32]);
        let requester_binding = b"requester-binding";
        let verified_request = ResolveIntro {
            relay_node_id: other_relay_node_id,
            intro_ticket: verified_intro_ticket(
                &target_signing_key,
                requester_binding,
                1_700_000_600,
            )
            .into_inner(),
        }
        .verify_with_public_key(&target_signing_key.public_key())
        .expect("resolve intro request should verify");
        let mut manager = RelayManager::new(RelayConfig {
            relay_mode: true,
            role_policy: RelayRolePolicy::milestone6_default(),
            max_concurrent_relay_tunnels: 1,
            max_intro_requests_per_minute: 1,
            max_bytes_relayed_per_peer_per_hour: 32,
            max_total_relay_bytes_per_hour: 40,
        })
        .expect("relay config should be valid");

        let mismatched = manager.process_resolve_intro(
            relay_node_id,
            verified_request,
            requester_binding,
            1_700_000_000,
        );
        assert_eq!(
            mismatched.status,
            IntroResponseStatus::RejectedRelayMismatch
        );

        let request = ResolveIntro {
            relay_node_id,
            intro_ticket: verified_intro_ticket(
                &target_signing_key,
                requester_binding,
                1_700_000_600,
            )
            .into_inner(),
        }
        .verify_with_public_key(&target_signing_key.public_key())
        .expect("resolve intro request should verify");
        let forwarded =
            manager.process_resolve_intro(relay_node_id, request, requester_binding, 1_700_000_000);
        assert_eq!(forwarded.status, IntroResponseStatus::Forwarded);

        let request = ResolveIntro {
            relay_node_id,
            intro_ticket: verified_intro_ticket(
                &target_signing_key,
                requester_binding,
                1_700_000_600,
            )
            .into_inner(),
        }
        .verify_with_public_key(&target_signing_key.public_key())
        .expect("resolve intro request should verify");
        let rate_limited =
            manager.process_resolve_intro(relay_node_id, request, requester_binding, 1_700_000_001);
        assert_eq!(
            rate_limited.status,
            IntroResponseStatus::RejectedRateLimited
        );
    }

    #[test]
    fn relay_intro_message_vectors_match_fixture() {
        let fixture = read_relay_intro_message_vector();
        let request = ResolveIntro {
            relay_node_id: NodeId::from_slice(&decode_hex(&fixture.relay_node_id_hex))
                .expect("relay node id should be 32 bytes"),
            intro_ticket: IntroTicket {
                ticket_id: decode_hex(&fixture.ticket_id_hex),
                target_node_id: NodeId::from_slice(&decode_hex(&fixture.target_node_id_hex))
                    .expect("target node id should be 32 bytes"),
                requester_binding: decode_hex(&fixture.requester_binding_hex),
                scope: "relay-intro".to_string(),
                issued_at_unix_s: fixture.issued_at_unix_s,
                expires_at_unix_s: fixture.expires_at_unix_s,
                nonce: decode_hex(&fixture.nonce_hex),
                signature: decode_hex(&fixture.signature_hex),
            },
        };
        let response = IntroResponse {
            relay_node_id: request.relay_node_id,
            target_node_id: request.intro_ticket.target_node_id,
            ticket_id: request.intro_ticket.ticket_id.clone(),
            status: parse_intro_response_status(&fixture.response_status),
        };

        assert_eq!(
            encode_hex(
                &request
                    .canonical_bytes()
                    .expect("resolve intro should serialize")
            ),
            fixture.resolve_intro_hex
        );
        assert_eq!(
            ResolveIntro::from_canonical_bytes(&decode_hex(&fixture.resolve_intro_hex))
                .expect("resolve intro should deserialize"),
            request
        );
        assert_eq!(
            encode_hex(
                &response
                    .canonical_bytes()
                    .expect("intro response should serialize")
            ),
            fixture.intro_response_hex
        );
        assert_eq!(
            IntroResponse::from_canonical_bytes(&decode_hex(&fixture.intro_response_hex))
                .expect("intro response should deserialize"),
            response
        );
    }

    #[test]
    fn reachability_plan_prefers_direct_first_and_keeps_secondary_relays() {
        let target_signing_key = Ed25519SigningKey::from_seed([21_u8; 32]);
        let target_node_id = derive_node_id(target_signing_key.public_key().as_bytes());
        let requester_binding = b"requester-binding";
        let verified_ticket =
            verified_intro_ticket(&target_signing_key, requester_binding, 1_700_000_600);
        let target_presence = sample_presence_record(target_node_id);
        let relay_a = RelayHint {
            relay_node_id: NodeId::from_bytes([2_u8; 32]),
            relay_transport_class: "quic".to_string(),
            relay_score: 40,
            relay_policy: vec![1_u8],
            expiry: 1_700_000_800,
        };
        let relay_b = RelayHint {
            relay_node_id: NodeId::from_bytes([1_u8; 32]),
            relay_transport_class: "tcp".to_string(),
            relay_score: 90,
            relay_policy: vec![2_u8],
            expiry: 1_700_000_800,
        };

        let plan = build_reachability_plan(
            &target_presence,
            &[relay_a, relay_b],
            &verified_ticket,
            requester_binding,
            1_700_000_100,
        )
        .expect("reachability plan should succeed");

        assert_eq!(
            plan.direct_attempts,
            vec![TransportClass::Quic, TransportClass::Tcp]
        );
        assert_eq!(plan.relay_fallback_count(), 2);
        assert_eq!(
            plan.relay_fallbacks[0].relay_node_id,
            NodeId::from_bytes([1_u8; 32])
        );
        assert_eq!(
            plan.relay_fallbacks[1].relay_node_id,
            NodeId::from_bytes([2_u8; 32])
        );
    }

    #[test]
    fn reachability_plan_rejects_expired_ticket_and_binding_mismatch() {
        let target_signing_key = Ed25519SigningKey::from_seed([22_u8; 32]);
        let target_node_id = derive_node_id(target_signing_key.public_key().as_bytes());
        let target_presence = sample_presence_record(target_node_id);
        let relay_hint = RelayHint {
            relay_node_id: NodeId::from_bytes([3_u8; 32]),
            relay_transport_class: "tcp".to_string(),
            relay_score: 50,
            relay_policy: vec![1_u8],
            expiry: 1_700_000_800,
        };

        let expired_ticket =
            verified_intro_ticket(&target_signing_key, b"requester-binding", 1_700_000_000);
        let error = build_reachability_plan(
            &target_presence,
            std::slice::from_ref(&relay_hint),
            &expired_ticket,
            b"requester-binding",
            1_700_000_001,
        )
        .expect_err("expired intro ticket must be rejected");
        assert!(matches!(
            error,
            RelayError::RecordValidation(RecordValidationError::Expired { .. })
        ));

        let fresh_ticket =
            verified_intro_ticket(&target_signing_key, b"requester-binding", 1_700_000_600);
        let error = build_reachability_plan(
            &target_presence,
            &[relay_hint],
            &fresh_ticket,
            b"wrong-binding",
            1_700_000_001,
        )
        .expect_err("requester binding mismatch must be rejected");
        assert!(matches!(error, RelayError::RequesterBindingMismatch));
    }

    #[test]
    fn reachability_plan_rejects_recursive_relay_hint_transport() {
        let target_signing_key = Ed25519SigningKey::from_seed([23_u8; 32]);
        let target_node_id = derive_node_id(target_signing_key.public_key().as_bytes());
        let target_presence = sample_presence_record(target_node_id);
        let relay_node_id = NodeId::from_bytes([4_u8; 32]);
        let relay_hint = RelayHint {
            relay_node_id,
            relay_transport_class: "relay".to_string(),
            relay_score: 75,
            relay_policy: vec![1_u8],
            expiry: 1_700_000_800,
        };
        let verified_ticket =
            verified_intro_ticket(&target_signing_key, b"requester-binding", 1_700_000_600);

        let error = build_reachability_plan(
            &target_presence,
            &[relay_hint],
            &verified_ticket,
            b"requester-binding",
            1_700_000_100,
        )
        .expect_err("relay-on-relay fallback should stay out of scope");
        assert!(matches!(
            error,
            RelayError::RelayChainingNotSupported { relay_node_id: actual }
            if actual == relay_node_id
        ));
    }

    #[test]
    fn relay_manager_enforces_intro_tunnel_and_byte_quotas() {
        let peer_node_id = NodeId::from_bytes([9_u8; 32]);
        let relay_node_id = NodeId::from_bytes([7_u8; 32]);
        let target_node_id = NodeId::from_bytes([8_u8; 32]);
        let mut manager = RelayManager::new(RelayConfig {
            relay_mode: true,
            role_policy: RelayRolePolicy::milestone6_default(),
            max_concurrent_relay_tunnels: 1,
            max_intro_requests_per_minute: 1,
            max_bytes_relayed_per_peer_per_hour: 32,
            max_total_relay_bytes_per_hour: 40,
        })
        .expect("relay config should be valid");

        manager
            .note_intro_request(1_700_000_000)
            .expect("first intro request should fit");
        let error = manager
            .note_intro_request(1_700_000_001)
            .expect_err("second intro request should exceed the per-minute limit");
        assert!(matches!(error, RelayError::IntroRateExceeded { .. }));

        manager
            .bind_tunnel(1, relay_node_id, target_node_id, 1_700_000_000)
            .expect("first tunnel should fit");
        let error = manager
            .bind_tunnel(2, relay_node_id, target_node_id, 1_700_000_001)
            .expect_err("second tunnel should exceed the concurrent limit");
        assert!(matches!(error, RelayError::TunnelQuotaExceeded { .. }));
        assert_eq!(manager.active_tunnel_count(), 1);

        manager
            .note_relayed_bytes(peer_node_id, 20, 1_700_000_000)
            .expect("first byte sample should fit");
        let error = manager
            .note_relayed_bytes(peer_node_id, 13, 1_700_000_001)
            .expect_err("per-peer byte cap should be enforced");
        assert!(matches!(error, RelayError::PerPeerByteQuotaExceeded { .. }));

        manager
            .note_relayed_bytes(NodeId::from_bytes([10_u8; 32]), 20, 1_700_000_002)
            .expect("total quota should still fit exactly");
        let error = manager
            .note_relayed_bytes(NodeId::from_bytes([11_u8; 32]), 1, 1_700_000_003)
            .expect_err("total relay byte cap should be enforced");
        assert!(matches!(error, RelayError::TotalByteQuotaExceeded { .. }));
    }

    #[test]
    fn relay_manager_rejects_usage_when_relay_mode_is_disabled() {
        let mut manager = RelayManager::new(RelayConfig::default())
            .expect("default relay config should be valid");

        let error = manager
            .note_intro_request(1_700_000_000)
            .expect_err("standard profile defaults to relay mode disabled");
        assert!(matches!(error, RelayError::RelayDisabled));
    }

    #[test]
    fn relay_cleanup_prunes_stale_tunnels_and_usage_windows() {
        let relay_node_id = NodeId::from_bytes([20_u8; 32]);
        let target_node_id = NodeId::from_bytes([21_u8; 32]);
        let peer_node_id = NodeId::from_bytes([22_u8; 32]);
        let mut manager = RelayManager::new(RelayConfig {
            relay_mode: true,
            role_policy: RelayRolePolicy::milestone6_default(),
            max_concurrent_relay_tunnels: 2,
            max_intro_requests_per_minute: 2,
            max_bytes_relayed_per_peer_per_hour: 128,
            max_total_relay_bytes_per_hour: 256,
        })
        .expect("relay config should be valid");

        manager
            .note_intro_request(1_700_000_000)
            .expect("intro request should fit");
        manager
            .bind_tunnel(1, relay_node_id, target_node_id, 1_700_000_000)
            .expect("tunnel should bind");
        manager
            .note_relayed_bytes(peer_node_id, 32, 1_700_000_000)
            .expect("byte sample should fit");

        let cleanup = manager.prune_stale_state(1_700_003_700, 120);
        assert_eq!(cleanup.stale_tunnels_pruned, 1);
        assert_eq!(cleanup.expired_intro_requests_pruned, 1);
        assert_eq!(cleanup.stale_peer_byte_windows_pruned, 1);
        assert_eq!(cleanup.stale_total_byte_samples_pruned, 1);

        let usage = manager.usage_snapshot();
        assert_eq!(usage.active_tunnels, 0);
        assert_eq!(usage.recent_intro_requests, 0);
        assert_eq!(usage.tracked_relay_peers, 0);
        assert_eq!(usage.total_relayed_bytes_last_hour, 0);
    }

    #[test]
    fn relay_observability_tracks_bind_and_rate_limited_intro() {
        let target_signing_key = Ed25519SigningKey::from_seed([61_u8; 32]);
        let relay_node_id = NodeId::from_bytes([70_u8; 32]);
        let target_node_id = derive_node_id(target_signing_key.public_key().as_bytes());
        let requester_binding = b"requester-binding";
        let mut manager = RelayManager::new(RelayConfig {
            relay_mode: true,
            role_policy: RelayRolePolicy::milestone6_default(),
            max_concurrent_relay_tunnels: 2,
            max_intro_requests_per_minute: 1,
            max_bytes_relayed_per_peer_per_hour: 32,
            max_total_relay_bytes_per_hour: 40,
        })
        .expect("relay config should be valid");
        let mut observability = Observability::default();

        let tunnel = manager
            .bind_tunnel_with_observability(
                1,
                relay_node_id,
                target_node_id,
                1_700_000_000,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_000_000,
                    node_id: relay_node_id,
                    correlation_id: 61,
                },
            )
            .expect("bind should succeed");
        assert_eq!(tunnel.tunnel_id, 1);

        let first_request = ResolveIntro {
            relay_node_id,
            intro_ticket: verified_intro_ticket(
                &target_signing_key,
                requester_binding,
                1_700_000_600,
            )
            .into_inner(),
        }
        .verify_with_public_key(&target_signing_key.public_key())
        .expect("resolve intro should verify");
        let first = manager.process_resolve_intro_with_observability(
            relay_node_id,
            first_request,
            requester_binding,
            1_700_000_000,
            &mut observability,
            LogContext {
                timestamp_unix_ms: 1_700_000_000_100,
                node_id: relay_node_id,
                correlation_id: 62,
            },
        );
        assert_eq!(first.status, IntroResponseStatus::Forwarded);

        let second_request = ResolveIntro {
            relay_node_id,
            intro_ticket: verified_intro_ticket(
                &target_signing_key,
                requester_binding,
                1_700_000_600,
            )
            .into_inner(),
        }
        .verify_with_public_key(&target_signing_key.public_key())
        .expect("resolve intro should verify");
        let second = manager.process_resolve_intro_with_observability(
            relay_node_id,
            second_request,
            requester_binding,
            1_700_000_001,
            &mut observability,
            LogContext {
                timestamp_unix_ms: 1_700_000_001_000,
                node_id: relay_node_id,
                correlation_id: 63,
            },
        );
        assert_eq!(second.status, IntroResponseStatus::RejectedRateLimited);
        assert_eq!(observability.metrics().relay_bind_total, 1);
        assert_eq!(observability.metrics().dropped_rate_limited_total, 1);
        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.event, "resolve_intro");
        assert_eq!(log.result, "rejected_rate_limited");
    }

    #[test]
    fn relay_manager_enforces_role_policy_for_intro_and_forward_paths() {
        let relay_node_id = NodeId::from_bytes([7_u8; 32]);
        let target_node_id = NodeId::from_bytes([8_u8; 32]);
        let peer_node_id = NodeId::from_bytes([9_u8; 32]);
        let mut manager = RelayManager::new(RelayConfig {
            relay_mode: true,
            role_policy: RelayRolePolicy {
                forward: false,
                intro: true,
                rendezvous: false,
                bridge: false,
            },
            max_concurrent_relay_tunnels: 1,
            max_intro_requests_per_minute: 1,
            max_bytes_relayed_per_peer_per_hour: 32,
            max_total_relay_bytes_per_hour: 40,
        })
        .expect("relay config should be valid");

        manager
            .note_intro_request(1_700_000_000)
            .expect("intro role should be enabled");

        let error = manager
            .bind_tunnel(1, relay_node_id, target_node_id, 1_700_000_000)
            .expect_err("forward role should be required for tunnel binds");
        assert!(matches!(
            error,
            RelayError::RelayModeDisabled {
                mode: RelayMode::Forward
            }
        ));

        let error = manager
            .note_relayed_bytes(peer_node_id, 1, 1_700_000_000)
            .expect_err("forward role should be required for relayed bytes");
        assert!(matches!(
            error,
            RelayError::RelayModeDisabled {
                mode: RelayMode::Forward
            }
        ));
    }

    fn sample_presence_record(node_id: NodeId) -> PresenceRecord {
        PresenceRecord {
            version: 1,
            node_id,
            epoch: 5,
            expires_at_unix_s: 1_700_000_600,
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

    fn verified_intro_ticket(
        target_signing_key: &Ed25519SigningKey,
        requester_binding: &[u8],
        expires_at_unix_s: u64,
    ) -> crate::records::VerifiedIntroTicket {
        let mut ticket = IntroTicket {
            ticket_id: vec![1_u8, 2, 3, 4],
            target_node_id: derive_node_id(target_signing_key.public_key().as_bytes()),
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
        ticket.signature = target_signing_key.sign(&body).as_bytes().to_vec();
        ticket
            .verify_with_public_key(&target_signing_key.public_key())
            .expect("intro ticket should verify")
    }

    #[derive(Debug, Deserialize)]
    struct RelayIntroMessageVector {
        target_node_id_hex: String,
        relay_node_id_hex: String,
        ticket_id_hex: String,
        requester_binding_hex: String,
        issued_at_unix_s: u64,
        expires_at_unix_s: u64,
        nonce_hex: String,
        signature_hex: String,
        resolve_intro_hex: String,
        response_status: String,
        intro_response_hex: String,
    }

    fn relay_intro_message_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("relay_intro_messages.json")
    }

    fn read_relay_intro_message_vector() -> RelayIntroMessageVector {
        let bytes = fs::read(relay_intro_message_vector_path())
            .expect("relay intro message vector file should exist");
        serde_json::from_slice(&bytes).expect("relay intro message vector file should parse")
    }

    fn parse_intro_response_status(value: &str) -> IntroResponseStatus {
        match value {
            "forwarded" => IntroResponseStatus::Forwarded,
            "rejected_relay_disabled" => IntroResponseStatus::RejectedRelayDisabled,
            "rejected_relay_mismatch" => IntroResponseStatus::RejectedRelayMismatch,
            "rejected_role_disabled" => IntroResponseStatus::RejectedRoleDisabled,
            "rejected_ticket_expired" => IntroResponseStatus::RejectedTicketExpired,
            "rejected_requester_binding" => IntroResponseStatus::RejectedRequesterBinding,
            "rejected_rate_limited" => IntroResponseStatus::RejectedRateLimited,
            _ => panic!("unknown intro response status in vector: {value}"),
        }
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0, "hex input should have even length");

        value
            .as_bytes()
            .chunks_exact(2)
            .map(|chunk| {
                let hex = std::str::from_utf8(chunk).expect("hex bytes should be utf-8");
                u8::from_str_radix(hex, 16).expect("hex digits should parse")
            })
            .collect()
    }

    fn encode_hex(bytes: &[u8]) -> String {
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").expect("hex encoding should succeed");
        }

        encoded
    }
}
