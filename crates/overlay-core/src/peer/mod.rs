use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    bootstrap::{BootstrapPeer, BootstrapPeerRole, BootstrapResponse, BootstrapValidationError},
    crypto::hash::Blake3Hasher,
    identity::NodeId,
    metrics::{LogComponent, LogContext, Observability},
};

pub const DEFAULT_MAX_NEIGHBORS: usize = 16;
pub const DEFAULT_MAX_RELAY_NEIGHBORS: usize = 4;
pub const DEFAULT_MAX_NEIGHBORS_PER_TRANSPORT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeighborState {
    Candidate,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeighborSource {
    Bootstrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeighborSelectionReason {
    RelayReserve,
    Diversity,
    RandomFill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighborStateEntry {
    pub node_id: NodeId,
    pub transport_classes: Vec<String>,
    pub capabilities: Vec<String>,
    pub dial_hints: Vec<String>,
    pub observed_role: BootstrapPeerRole,
    pub source: NeighborSource,
    pub state: NeighborState,
    pub selection_reason: Option<NeighborSelectionReason>,
    pub selected_transport_class: Option<String>,
    pub last_updated_unix_s: u64,
}

impl NeighborStateEntry {
    pub fn is_relay_capable(&self) -> bool {
        self.observed_role == BootstrapPeerRole::Relay
            || self
                .capabilities
                .iter()
                .any(|capability| capability == "relay-forward" || capability == "relay-intro")
            || self.transport_classes.iter().any(|class| class == "relay")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerStoreConfig {
    pub max_neighbors: usize,
    pub max_relay_neighbors: usize,
    pub max_neighbors_per_transport: usize,
}

impl Default for PeerStoreConfig {
    fn default() -> Self {
        Self {
            max_neighbors: DEFAULT_MAX_NEIGHBORS,
            max_relay_neighbors: DEFAULT_MAX_RELAY_NEIGHBORS,
            max_neighbors_per_transport: DEFAULT_MAX_NEIGHBORS_PER_TRANSPORT,
        }
    }
}

impl PeerStoreConfig {
    pub fn validate(self) -> Result<Self, PeerStoreError> {
        for (field, value) in [
            ("max_neighbors", self.max_neighbors),
            ("max_relay_neighbors", self.max_relay_neighbors),
            (
                "max_neighbors_per_transport",
                self.max_neighbors_per_transport,
            ),
        ] {
            if value == 0 {
                return Err(PeerStoreError::ZeroLimit { field });
            }
        }

        if self.max_relay_neighbors > self.max_neighbors {
            return Err(PeerStoreError::RelayLimitExceedsTotal {
                max_neighbors: self.max_neighbors,
                max_relay_neighbors: self.max_relay_neighbors,
            });
        }

        Ok(self)
    }
}

#[derive(Debug, Error)]
pub enum PeerStoreError {
    #[error("peer store limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
    #[error(
        "peer store max_relay_neighbors ({max_relay_neighbors}) exceeds max_neighbors ({max_neighbors})"
    )]
    RelayLimitExceedsTotal {
        max_neighbors: usize,
        max_relay_neighbors: usize,
    },
    #[error(transparent)]
    BootstrapValidation(#[from] BootstrapValidationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerStore {
    config: PeerStoreConfig,
    neighbors: BTreeMap<NodeId, NeighborStateEntry>,
}

impl PeerStore {
    pub fn new(config: PeerStoreConfig) -> Result<Self, PeerStoreError> {
        Ok(Self {
            config: config.validate()?,
            neighbors: BTreeMap::new(),
        })
    }

    pub const fn config(&self) -> PeerStoreConfig {
        self.config
    }

    pub fn neighbors(&self) -> impl Iterator<Item = &NeighborStateEntry> {
        self.neighbors.values()
    }

    pub fn neighbor_count(&self) -> usize {
        self.neighbors.len()
    }

    pub fn active_neighbors(&self) -> impl Iterator<Item = &NeighborStateEntry> {
        self.neighbors
            .values()
            .filter(|neighbor| neighbor.state == NeighborState::Active)
    }

    pub fn active_neighbor_entries(&self) -> Vec<NeighborStateEntry> {
        self.active_neighbors().cloned().collect()
    }

    pub fn candidate_neighbors(&self) -> impl Iterator<Item = &NeighborStateEntry> {
        self.neighbors
            .values()
            .filter(|neighbor| neighbor.state == NeighborState::Candidate)
    }

    pub fn ingest_bootstrap_response(
        &mut self,
        response: BootstrapResponse,
        now_unix_s: u64,
    ) -> Result<Vec<NodeId>, PeerStoreError> {
        let response = response.validated(now_unix_s)?;
        for peer in response.peers {
            self.upsert_bootstrap_peer(peer, now_unix_s);
        }

        Ok(self.rebalance())
    }

    pub fn ingest_bootstrap_response_with_observability(
        &mut self,
        response: BootstrapResponse,
        now_unix_s: u64,
        observability: &mut Observability,
        context: LogContext,
    ) -> Result<Vec<NodeId>, PeerStoreError> {
        match self.ingest_bootstrap_response(response, now_unix_s) {
            Ok(active) => {
                observability.set_active_peers(self.active_neighbors().count());
                observability.push_log(context, LogComponent::Peer, "bootstrap_ingest", "accepted");
                Ok(active)
            }
            Err(error) => {
                observability.push_log(context, LogComponent::Peer, "bootstrap_ingest", "rejected");
                Err(error)
            }
        }
    }

    pub fn restore_bootstrap_neighbors(
        &mut self,
        entries: impl IntoIterator<Item = NeighborStateEntry>,
        now_unix_s: u64,
    ) -> Vec<NodeId> {
        self.neighbors.clear();
        for entry in entries {
            self.neighbors.insert(
                entry.node_id,
                NeighborStateEntry {
                    node_id: entry.node_id,
                    transport_classes: entry.transport_classes,
                    capabilities: entry.capabilities,
                    dial_hints: entry.dial_hints,
                    observed_role: entry.observed_role,
                    source: NeighborSource::Bootstrap,
                    state: NeighborState::Candidate,
                    selection_reason: None,
                    selected_transport_class: None,
                    last_updated_unix_s: now_unix_s,
                },
            );
        }
        self.rebalance()
    }

    fn upsert_bootstrap_peer(&mut self, peer: BootstrapPeer, now_unix_s: u64) {
        self.neighbors.insert(
            peer.node_id,
            NeighborStateEntry {
                node_id: peer.node_id,
                transport_classes: peer.transport_classes,
                capabilities: peer.capabilities,
                dial_hints: peer.dial_hints,
                observed_role: peer.observed_role,
                source: NeighborSource::Bootstrap,
                state: NeighborState::Candidate,
                selection_reason: None,
                selected_transport_class: None,
                last_updated_unix_s: now_unix_s,
            },
        );
    }

    fn rebalance(&mut self) -> Vec<NodeId> {
        let sorted_node_ids = self.neighbors.keys().copied().collect::<Vec<_>>();
        let random_order = random_fill_order(&sorted_node_ids);
        let mut selected = BTreeSet::new();
        let mut transport_counts = BTreeMap::<String, usize>::new();
        let mut relay_selected = 0usize;

        for neighbor in self.neighbors.values_mut() {
            neighbor.state = NeighborState::Candidate;
            neighbor.selection_reason = None;
            neighbor.selected_transport_class = None;
        }

        self.select_neighbors(
            sorted_node_ids.iter().copied(),
            &mut selected,
            &mut transport_counts,
            &mut relay_selected,
            SelectionPhase {
                reason: NeighborSelectionReason::RelayReserve,
                relay_only: true,
                new_transport_only: true,
            },
        );
        self.select_neighbors(
            sorted_node_ids.iter().copied(),
            &mut selected,
            &mut transport_counts,
            &mut relay_selected,
            SelectionPhase {
                reason: NeighborSelectionReason::RelayReserve,
                relay_only: true,
                new_transport_only: false,
            },
        );
        self.select_neighbors(
            sorted_node_ids.iter().copied(),
            &mut selected,
            &mut transport_counts,
            &mut relay_selected,
            SelectionPhase {
                reason: NeighborSelectionReason::Diversity,
                relay_only: false,
                new_transport_only: true,
            },
        );
        self.select_neighbors(
            random_order,
            &mut selected,
            &mut transport_counts,
            &mut relay_selected,
            SelectionPhase {
                reason: NeighborSelectionReason::RandomFill,
                relay_only: false,
                new_transport_only: false,
            },
        );

        selected.into_iter().collect()
    }

    fn select_neighbors<I>(
        &mut self,
        candidates: I,
        selected: &mut BTreeSet<NodeId>,
        transport_counts: &mut BTreeMap<String, usize>,
        relay_selected: &mut usize,
        phase: SelectionPhase,
    ) where
        I: IntoIterator<Item = NodeId>,
    {
        for node_id in candidates {
            if selected.len() == self.config.max_neighbors || selected.contains(&node_id) {
                continue;
            }

            let Some(neighbor) = self.neighbors.get(&node_id) else {
                continue;
            };
            let is_relay_capable = neighbor.is_relay_capable();
            if phase.relay_only && !is_relay_capable {
                continue;
            }
            if phase.relay_only && *relay_selected == self.config.max_relay_neighbors {
                continue;
            }

            let Some(selected_transport_class) = choose_transport_class(
                &neighbor.transport_classes,
                transport_counts,
                self.config.max_neighbors_per_transport,
                phase.new_transport_only,
            ) else {
                continue;
            };

            let neighbor = self
                .neighbors
                .get_mut(&node_id)
                .expect("selected neighbor must exist");
            neighbor.state = NeighborState::Active;
            neighbor.selection_reason = Some(phase.reason);
            neighbor.selected_transport_class = Some(selected_transport_class.clone());
            selected.insert(node_id);
            *transport_counts
                .entry(selected_transport_class)
                .or_insert(0) += 1;
            if is_relay_capable {
                *relay_selected += 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectionPhase {
    reason: NeighborSelectionReason,
    relay_only: bool,
    new_transport_only: bool,
}

fn choose_transport_class(
    transport_classes: &[String],
    transport_counts: &BTreeMap<String, usize>,
    per_transport_limit: usize,
    new_transport_only: bool,
) -> Option<String> {
    let mut candidates = transport_classes
        .iter()
        .filter_map(|transport_class| {
            let count = transport_counts.get(transport_class).copied().unwrap_or(0);
            (count < per_transport_limit).then_some((count, transport_class))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(left_count, left_class), (right_count, right_class)| {
        left_count
            .cmp(right_count)
            .then_with(|| left_class.cmp(right_class))
    });

    if new_transport_only {
        candidates.retain(|(count, _)| *count == 0);
    }

    candidates
        .into_iter()
        .next()
        .map(|(_, transport_class)| transport_class.clone())
}

fn random_fill_order(node_ids: &[NodeId]) -> Vec<NodeId> {
    let mut randomized = node_ids
        .iter()
        .copied()
        .map(|node_id| {
            let mut hasher = Blake3Hasher::new();
            hasher.update(node_id.as_ref());
            (hasher.finalize(), node_id)
        })
        .collect::<Vec<_>>();
    randomized.sort_by(|(left_hash, left_node_id), (right_hash, right_node_id)| {
        left_hash
            .cmp(right_hash)
            .then_with(|| left_node_id.cmp(right_node_id))
    });
    randomized.into_iter().map(|(_, node_id)| node_id).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        NeighborSelectionReason, NeighborState, PeerStore, PeerStoreConfig, PeerStoreError,
    };
    use crate::bootstrap::{
        BootstrapNetworkParams, BootstrapPeer, BootstrapPeerRole, BootstrapResponse,
        BootstrapValidationError, BOOTSTRAP_SCHEMA_VERSION,
    };
    use crate::{
        identity::NodeId,
        metrics::{LogContext, Observability},
        session::HANDSHAKE_VERSION,
        wire::MAX_FRAME_BODY_LEN,
    };

    #[test]
    fn rejects_invalid_peer_store_limits() {
        let error = PeerStore::new(PeerStoreConfig {
            max_neighbors: 2,
            max_relay_neighbors: 3,
            max_neighbors_per_transport: 1,
        })
        .expect_err("relay slots must not exceed total slots");

        assert!(matches!(
            error,
            PeerStoreError::RelayLimitExceedsTotal {
                max_neighbors: 2,
                max_relay_neighbors: 3,
            }
        ));
    }

    #[test]
    fn bootstrap_ingest_enforces_bounds_and_preserves_diversity() {
        let mut store = PeerStore::new(PeerStoreConfig {
            max_neighbors: 3,
            max_relay_neighbors: 1,
            max_neighbors_per_transport: 1,
        })
        .expect("peer store config should be valid");

        let active = store
            .ingest_bootstrap_response(sample_response(), 1_700_000_100)
            .expect("bootstrap response should ingest");

        assert_eq!(active.len(), 3);
        assert_eq!(store.active_neighbors().count(), 3);
        assert_eq!(store.candidate_neighbors().count(), 2);
        assert_eq!(
            store
                .active_neighbors()
                .filter(|neighbor| neighbor.is_relay_capable())
                .count(),
            1
        );

        let active_transports = store
            .active_neighbors()
            .filter_map(|neighbor| neighbor.selected_transport_class.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(active_transports.len(), 3);
        assert!(store.active_neighbors().any(|neighbor| {
            neighbor.selection_reason == Some(NeighborSelectionReason::RelayReserve)
        }));
        assert!(store.active_neighbors().any(|neighbor| {
            neighbor.selection_reason == Some(NeighborSelectionReason::Diversity)
        }));
    }

    #[test]
    fn rebalance_marks_overflow_peers_as_candidates() {
        let mut store = PeerStore::new(PeerStoreConfig {
            max_neighbors: 2,
            max_relay_neighbors: 1,
            max_neighbors_per_transport: 2,
        })
        .expect("peer store config should be valid");

        store
            .ingest_bootstrap_response(sample_response(), 1_700_000_100)
            .expect("bootstrap response should ingest");

        assert_eq!(store.active_neighbors().count(), 2);
        assert!(store
            .candidate_neighbors()
            .all(|neighbor| neighbor.state == NeighborState::Candidate));
    }

    #[test]
    fn bootstrap_ingest_updates_observability_gauge_and_log() {
        let mut store = PeerStore::new(PeerStoreConfig {
            max_neighbors: 3,
            max_relay_neighbors: 1,
            max_neighbors_per_transport: 1,
        })
        .expect("peer store config should be valid");
        let node_id = NodeId::from_bytes([9_u8; 32]);
        let mut observability = Observability::default();

        let active = store
            .ingest_bootstrap_response_with_observability(
                sample_response(),
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 41,
                },
            )
            .expect("bootstrap response should ingest");

        assert_eq!(active.len(), 3);
        assert_eq!(observability.metrics().active_peers, 3);
        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.event, "bootstrap_ingest");
        assert_eq!(log.result, "accepted");
    }

    #[test]
    fn rejected_bootstrap_ingest_preserves_active_peer_gauge_and_logs_rejected() {
        let mut store = PeerStore::new(PeerStoreConfig {
            max_neighbors: 3,
            max_relay_neighbors: 1,
            max_neighbors_per_transport: 1,
        })
        .expect("peer store config should be valid");
        let node_id = NodeId::from_bytes([10_u8; 32]);
        let mut observability = Observability::default();

        let active = store
            .ingest_bootstrap_response_with_observability(
                sample_response(),
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 42,
                },
            )
            .expect("bootstrap response should ingest");
        assert_eq!(active.len(), 3);
        assert_eq!(observability.metrics().active_peers, 3);

        let mut invalid = sample_response();
        invalid.max_frame_body_len = 0;
        let error = store
            .ingest_bootstrap_response_with_observability(
                invalid,
                1_700_000_101,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_101_000,
                    node_id,
                    correlation_id: 43,
                },
            )
            .expect_err("invalid bootstrap response must be rejected");

        assert!(matches!(
            error,
            PeerStoreError::BootstrapValidation(BootstrapValidationError::ZeroField {
                field: "max_frame_body_len",
            })
        ));
        assert_eq!(observability.metrics().active_peers, 3);
        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.event, "bootstrap_ingest");
        assert_eq!(log.result, "rejected");
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
                peer(
                    [5_u8; 32],
                    &["tcp"],
                    &[],
                    BootstrapPeerRole::Standard,
                    &["tcp://node-e"],
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
}
