//! Relay intro and fallback reachability baseline for Milestone 6.
//! Keep direct transport attempts first and use relay only as bounded fallback.

use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::RecordValidationError,
    identity::NodeId,
    records::{FreshRecord, PresenceRecord, RelayHint, VerifiedIntroTicket},
    transport::TransportClass,
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
pub enum RelayProfile {
    Tiny,
    Standard,
    Relay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayConfig {
    pub relay_mode: bool,
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
                max_concurrent_relay_tunnels: TINY_MAX_CONCURRENT_RELAY_TUNNELS,
                max_intro_requests_per_minute: TINY_MAX_INTRO_REQUESTS_PER_MINUTE,
                max_bytes_relayed_per_peer_per_hour: TINY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
                max_total_relay_bytes_per_hour: TINY_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
            },
            RelayProfile::Standard => Self {
                relay_mode: false,
                max_concurrent_relay_tunnels: STANDARD_MAX_CONCURRENT_RELAY_TUNNELS,
                max_intro_requests_per_minute: STANDARD_MAX_INTRO_REQUESTS_PER_MINUTE,
                max_bytes_relayed_per_peer_per_hour: STANDARD_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
                max_total_relay_bytes_per_hour: STANDARD_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
            },
            RelayProfile::Relay => Self {
                relay_mode: true,
                max_concurrent_relay_tunnels: RELAY_MAX_CONCURRENT_RELAY_TUNNELS,
                max_intro_requests_per_minute: RELAY_MAX_INTRO_REQUESTS_PER_MINUTE,
                max_bytes_relayed_per_peer_per_hour: RELAY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
                max_total_relay_bytes_per_hour: RELAY_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
            },
        }
    }

    pub const fn with_relay_mode(mut self, relay_mode: bool) -> Self {
        self.relay_mode = relay_mode;
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

    pub fn note_intro_request(&mut self, now_unix_s: u64) -> Result<(), RelayError> {
        self.ensure_relay_mode()?;
        prune_recent_timestamps(&mut self.intro_requests, now_unix_s, RELAY_INTRO_WINDOW_S);
        if self.intro_requests.len() == self.config.max_intro_requests_per_minute {
            return Err(RelayError::IntroRateExceeded {
                max_intro_requests_per_minute: self.config.max_intro_requests_per_minute,
            });
        }

        self.intro_requests.push_back(now_unix_s);
        Ok(())
    }

    pub fn bind_tunnel(
        &mut self,
        tunnel_id: u64,
        relay_node_id: NodeId,
        target_node_id: NodeId,
        now_unix_s: u64,
    ) -> Result<RelayTunnel, RelayError> {
        self.ensure_relay_mode()?;
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
            Ok(RelayFallbackCandidate {
                relay_node_id: hint.relay_node_id,
                relay_transport_class: parse_transport_class(&hint.relay_transport_class)?,
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

#[cfg(test)]
mod tests {
    use super::{
        build_reachability_plan, RelayConfig, RelayError, RelayManager, RelayProfile,
        RELAY_MAX_CONCURRENT_RELAY_TUNNELS, RELAY_MAX_INTRO_REQUESTS_PER_MINUTE,
        RELAY_MAX_TOTAL_RELAY_BYTES_PER_HOUR, STANDARD_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
        STANDARD_MAX_CONCURRENT_RELAY_TUNNELS, STANDARD_MAX_INTRO_REQUESTS_PER_MINUTE,
        STANDARD_MAX_TOTAL_RELAY_BYTES_PER_HOUR, TINY_MAX_BYTES_RELAYED_PER_PEER_PER_HOUR,
        TINY_MAX_CONCURRENT_RELAY_TUNNELS, TINY_MAX_INTRO_REQUESTS_PER_MINUTE,
        TINY_MAX_TOTAL_RELAY_BYTES_PER_HOUR,
    };
    use crate::{
        crypto::sign::Ed25519SigningKey,
        error::RecordValidationError,
        identity::{derive_node_id, NodeId},
        records::{IntroTicket, PresenceRecord, RelayHint},
        transport::TransportClass,
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
            &[relay_hint.clone()],
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
    fn relay_manager_enforces_intro_tunnel_and_byte_quotas() {
        let peer_node_id = NodeId::from_bytes([9_u8; 32]);
        let relay_node_id = NodeId::from_bytes([7_u8; 32]);
        let target_node_id = NodeId::from_bytes([8_u8; 32]);
        let mut manager = RelayManager::new(RelayConfig {
            relay_mode: true,
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
}
