//! Exact presence publish and lookup baseline for Milestone 5.
//! Call `publish_verified` only after signature validation has succeeded upstream.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use blake3::Hasher;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::{
    crypto::sign::Ed25519PublicKey,
    error::{FrameError, RecordEncodingError, RecordValidationError},
    identity::NodeId,
    records::{FreshRecord, NodeRecord, PresenceRecord, VerifiedPresenceRecord},
    wire::{Message, MessageType, MAX_FRAME_BODY_LEN},
};

const PLACEMENT_KEY_DOMAIN_SEPARATOR: &[u8] = b"overlay-mvp-rendezvous-placement";

pub const DEFAULT_MAX_PUBLISHED_RECORDS: usize = 1024;
pub const DEFAULT_MAX_NEGATIVE_CACHE_ENTRIES: usize = 256;
pub const DEFAULT_NEGATIVE_CACHE_TTL_S: u64 = 60;
pub const DEFAULT_MAX_LOOKUP_BUDGET: u8 = 8;
pub const DEFAULT_MAX_LOOKUP_SEEN_HELPERS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlacementKey([u8; NodeId::LEN]);

impl PlacementKey {
    pub fn derive(node_id: &NodeId) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(PLACEMENT_KEY_DOMAIN_SEPARATOR);
        hasher.update(node_id.as_bytes());
        Self(*hasher.finalize().as_bytes())
    }

    pub const fn as_bytes(&self) -> &[u8; NodeId::LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RendezvousConfig {
    pub max_published_records: usize,
    pub max_negative_cache_entries: usize,
    pub negative_cache_ttl_s: u64,
    pub max_lookup_budget: u8,
    pub max_lookup_seen_helpers: usize,
}

impl Default for RendezvousConfig {
    fn default() -> Self {
        Self {
            max_published_records: DEFAULT_MAX_PUBLISHED_RECORDS,
            max_negative_cache_entries: DEFAULT_MAX_NEGATIVE_CACHE_ENTRIES,
            negative_cache_ttl_s: DEFAULT_NEGATIVE_CACHE_TTL_S,
            max_lookup_budget: DEFAULT_MAX_LOOKUP_BUDGET,
            max_lookup_seen_helpers: DEFAULT_MAX_LOOKUP_SEEN_HELPERS,
        }
    }
}

impl RendezvousConfig {
    pub fn validate(self) -> Result<Self, RendezvousError> {
        for (field, value) in [
            ("max_published_records", self.max_published_records as u64),
            (
                "max_negative_cache_entries",
                self.max_negative_cache_entries as u64,
            ),
            ("negative_cache_ttl_s", self.negative_cache_ttl_s),
            ("max_lookup_budget", self.max_lookup_budget as u64),
            (
                "max_lookup_seen_helpers",
                self.max_lookup_seen_helpers as u64,
            ),
        ] {
            if value == 0 {
                return Err(RendezvousError::ZeroLimit { field });
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Error)]
pub enum RendezvousError {
    #[error("rendezvous config limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
    #[error(
        "conflicting presence record for {node_id}: epoch {epoch} sequence {sequence} already stored with different bytes"
    )]
    ConflictingPresenceRecord {
        node_id: NodeId,
        epoch: u64,
        sequence: u64,
    },
    #[error("lookup seen-set limit {max_lookup_seen_helpers} exceeded")]
    SeenSetLimitExceeded { max_lookup_seen_helpers: usize },
    #[error(transparent)]
    RecordValidation(#[from] RecordValidationError),
    #[error(transparent)]
    RecordEncoding(#[from] RecordEncodingError),
    #[error(transparent)]
    MessageEncoding(#[from] RendezvousMessageError),
}

#[derive(Debug, Error)]
pub enum RendezvousMessageError {
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Frame(#[from] FrameError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishPresence {
    pub record: PresenceRecord,
}

impl PublishPresence {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RendezvousMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RendezvousMessageError> {
        parse_message_bytes(bytes)
    }

    pub fn verify_with_public_key(
        self,
        signer_public_key: &Ed25519PublicKey,
    ) -> Result<VerifiedPublishPresence, crate::error::PresenceVerificationError> {
        Ok(VerifiedPublishPresence {
            record: self.record.verify_with_public_key(signer_public_key)?,
        })
    }

    pub fn verify_with_trusted_node_record(
        self,
        node_record: &NodeRecord,
    ) -> Result<VerifiedPublishPresence, crate::error::PresenceVerificationError> {
        Ok(VerifiedPublishPresence {
            record: self.record.verify_with_trusted_node_record(node_record)?,
        })
    }
}

impl Message for PublishPresence {
    const TYPE: MessageType = MessageType::PublishPresence;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPublishPresence {
    record: VerifiedPresenceRecord,
}

impl VerifiedPublishPresence {
    pub fn record(&self) -> &PresenceRecord {
        self.record.as_ref()
    }

    pub fn into_record(self) -> PresenceRecord {
        self.record.into_inner()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishDisposition {
    Stored,
    Replaced,
    Duplicate,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishAck {
    pub node_id: NodeId,
    pub placement_key: PlacementKey,
    pub disposition: PublishDisposition,
    pub accepted_epoch: u64,
    pub accepted_sequence: u64,
}

impl PublishAck {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RendezvousMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RendezvousMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for PublishAck {
    const TYPE: MessageType = MessageType::PublishAck;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookupNode {
    pub node_id: NodeId,
}

impl LookupNode {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RendezvousMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RendezvousMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for LookupNode {
    const TYPE: MessageType = MessageType::LookupNode;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookupResult {
    pub node_id: NodeId,
    pub placement_key: PlacementKey,
    pub record: PresenceRecord,
    pub remaining_budget: u8,
}

impl LookupResult {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RendezvousMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RendezvousMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for LookupResult {
    const TYPE: MessageType = MessageType::LookupResult;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookupNotFoundReason {
    Missing,
    NegativeCacheHit,
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookupNotFound {
    pub node_id: NodeId,
    pub placement_key: PlacementKey,
    pub reason: LookupNotFoundReason,
    pub remaining_budget: u8,
}

impl LookupNotFound {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RendezvousMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RendezvousMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for LookupNotFound {
    const TYPE: MessageType = MessageType::LookupNotFound;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LookupResponse {
    Result(Box<LookupResult>),
    NotFound(LookupNotFound),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupState {
    remaining_budget: u8,
    seen_helpers: BTreeSet<NodeId>,
    max_lookup_seen_helpers: usize,
}

impl LookupState {
    pub fn new(
        requested_budget: u8,
        max_lookup_budget: u8,
        max_lookup_seen_helpers: usize,
    ) -> Result<Self, RendezvousError> {
        if max_lookup_budget == 0 {
            return Err(RendezvousError::ZeroLimit {
                field: "max_lookup_budget",
            });
        }

        if max_lookup_seen_helpers == 0 {
            return Err(RendezvousError::ZeroLimit {
                field: "max_lookup_seen_helpers",
            });
        }

        Ok(Self {
            remaining_budget: requested_budget.min(max_lookup_budget),
            seen_helpers: BTreeSet::new(),
            max_lookup_seen_helpers,
        })
    }

    pub const fn remaining_budget(&self) -> u8 {
        self.remaining_budget
    }

    pub fn seen_helpers(&self) -> impl Iterator<Item = &NodeId> {
        self.seen_helpers.iter()
    }

    pub fn note_helper(&mut self, helper_node_id: NodeId) -> Result<bool, RendezvousError> {
        if self.seen_helpers.contains(&helper_node_id) {
            return Ok(false);
        }

        if self.seen_helpers.len() == self.max_lookup_seen_helpers {
            return Err(RendezvousError::SeenSetLimitExceeded {
                max_lookup_seen_helpers: self.max_lookup_seen_helpers,
            });
        }

        self.seen_helpers.insert(helper_node_id);
        Ok(true)
    }

    fn consume_attempt(&mut self) -> bool {
        if self.remaining_budget == 0 {
            return false;
        }

        self.remaining_budget -= 1;
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegativeCacheEntry {
    pub node_id: NodeId,
    pub placement_key: PlacementKey,
    pub cached_at_unix_s: u64,
    pub expires_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendezvousStore {
    config: RendezvousConfig,
    published: BTreeMap<PlacementKey, StoredPresence>,
    negative_cache: BTreeMap<PlacementKey, NegativeCacheEntry>,
}

impl RendezvousStore {
    pub fn new(config: RendezvousConfig) -> Result<Self, RendezvousError> {
        Ok(Self {
            config: config.validate()?,
            published: BTreeMap::new(),
            negative_cache: BTreeMap::new(),
        })
    }

    pub const fn config(&self) -> RendezvousConfig {
        self.config
    }

    pub fn lookup_state(&self, requested_budget: u8) -> Result<LookupState, RendezvousError> {
        LookupState::new(
            requested_budget,
            self.config.max_lookup_budget,
            self.config.max_lookup_seen_helpers,
        )
    }

    pub fn published_record(&self, node_id: &NodeId) -> Option<&PresenceRecord> {
        let placement_key = PlacementKey::derive(node_id);
        self.published
            .get(&placement_key)
            .map(|entry| &entry.record)
    }

    pub fn published_record_count(&self) -> usize {
        self.published.len()
    }

    pub fn negative_cache_len(&self) -> usize {
        self.negative_cache.len()
    }

    pub fn negative_cache_entry(&self, node_id: &NodeId) -> Option<&NegativeCacheEntry> {
        let placement_key = PlacementKey::derive(node_id);
        self.negative_cache.get(&placement_key)
    }

    pub fn publish_verified(
        &mut self,
        publish: VerifiedPublishPresence,
        now_unix_s: u64,
    ) -> Result<PublishAck, RendezvousError> {
        self.prune_expired(now_unix_s);

        let publish = PublishPresence {
            record: publish.into_record(),
        };

        publish.canonical_bytes()?;
        publish.record.canonical_body_bytes()?;
        publish.record.validate_freshness(now_unix_s)?;

        let placement_key = PlacementKey::derive(&publish.record.node_id);
        self.negative_cache.remove(&placement_key);

        if let Some(existing) = self.published.get_mut(&placement_key) {
            return match compare_presence_records(&publish.record, &existing.record) {
                PresenceOrdering::Newer => {
                    existing.record = publish.record;
                    existing.stored_at_unix_s = now_unix_s;
                    Ok(PublishAck {
                        node_id: existing.record.node_id,
                        placement_key,
                        disposition: PublishDisposition::Replaced,
                        accepted_epoch: existing.record.epoch,
                        accepted_sequence: existing.record.sequence,
                    })
                }
                PresenceOrdering::Older => Ok(PublishAck {
                    node_id: existing.record.node_id,
                    placement_key,
                    disposition: PublishDisposition::Stale,
                    accepted_epoch: existing.record.epoch,
                    accepted_sequence: existing.record.sequence,
                }),
                PresenceOrdering::Same => Ok(PublishAck {
                    node_id: existing.record.node_id,
                    placement_key,
                    disposition: PublishDisposition::Duplicate,
                    accepted_epoch: existing.record.epoch,
                    accepted_sequence: existing.record.sequence,
                }),
                PresenceOrdering::Conflict => Err(RendezvousError::ConflictingPresenceRecord {
                    node_id: publish.record.node_id,
                    epoch: publish.record.epoch,
                    sequence: publish.record.sequence,
                }),
            };
        }

        self.insert_published_record(
            placement_key,
            StoredPresence {
                record: publish.record,
                stored_at_unix_s: now_unix_s,
            },
        );

        let stored = self
            .published
            .get(&placement_key)
            .expect("published record must exist after insert");

        Ok(PublishAck {
            node_id: stored.record.node_id,
            placement_key,
            disposition: PublishDisposition::Stored,
            accepted_epoch: stored.record.epoch,
            accepted_sequence: stored.record.sequence,
        })
    }

    pub fn lookup(
        &mut self,
        lookup: LookupNode,
        now_unix_s: u64,
        state: &mut LookupState,
    ) -> LookupResponse {
        self.prune_expired(now_unix_s);

        let placement_key = PlacementKey::derive(&lookup.node_id);
        if !state.consume_attempt() {
            return LookupResponse::NotFound(LookupNotFound {
                node_id: lookup.node_id,
                placement_key,
                reason: LookupNotFoundReason::BudgetExhausted,
                remaining_budget: state.remaining_budget(),
            });
        }

        if self.negative_cache.contains_key(&placement_key) {
            return LookupResponse::NotFound(LookupNotFound {
                node_id: lookup.node_id,
                placement_key,
                reason: LookupNotFoundReason::NegativeCacheHit,
                remaining_budget: state.remaining_budget(),
            });
        }

        if let Some(stored) = self.published.get(&placement_key) {
            return LookupResponse::Result(Box::new(LookupResult {
                node_id: stored.record.node_id,
                placement_key,
                record: stored.record.clone(),
                remaining_budget: state.remaining_budget(),
            }));
        }

        self.insert_negative_cache_entry(lookup.node_id, placement_key, now_unix_s);
        LookupResponse::NotFound(LookupNotFound {
            node_id: lookup.node_id,
            placement_key,
            reason: LookupNotFoundReason::Missing,
            remaining_budget: state.remaining_budget(),
        })
    }

    fn prune_expired(&mut self, now_unix_s: u64) {
        self.published
            .retain(|_, entry| entry.record.is_fresh(now_unix_s));
        self.negative_cache
            .retain(|_, entry| entry.expires_at_unix_s > now_unix_s);
    }

    fn insert_published_record(&mut self, placement_key: PlacementKey, stored: StoredPresence) {
        if !self.published.contains_key(&placement_key)
            && self.published.len() == self.config.max_published_records
        {
            self.evict_oldest_published();
        }

        self.published.insert(placement_key, stored);
    }

    fn insert_negative_cache_entry(
        &mut self,
        node_id: NodeId,
        placement_key: PlacementKey,
        now_unix_s: u64,
    ) {
        if !self.negative_cache.contains_key(&placement_key)
            && self.negative_cache.len() == self.config.max_negative_cache_entries
        {
            self.evict_oldest_negative_cache_entry();
        }

        self.negative_cache.insert(
            placement_key,
            NegativeCacheEntry {
                node_id,
                placement_key,
                cached_at_unix_s: now_unix_s,
                expires_at_unix_s: now_unix_s.saturating_add(self.config.negative_cache_ttl_s),
            },
        );
    }

    fn evict_oldest_published(&mut self) {
        let oldest_key = self
            .published
            .iter()
            .min_by_key(|(_, entry)| entry.stored_at_unix_s)
            .map(|(placement_key, _)| *placement_key);

        if let Some(placement_key) = oldest_key {
            self.published.remove(&placement_key);
        }
    }

    fn evict_oldest_negative_cache_entry(&mut self) {
        let oldest_key = self
            .negative_cache
            .iter()
            .min_by_key(|(_, entry)| entry.cached_at_unix_s)
            .map(|(placement_key, _)| *placement_key);

        if let Some(placement_key) = oldest_key {
            self.negative_cache.remove(&placement_key);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredPresence {
    record: PresenceRecord,
    stored_at_unix_s: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresenceOrdering {
    Newer,
    Older,
    Same,
    Conflict,
}

fn compare_presence_records(
    candidate: &PresenceRecord,
    current: &PresenceRecord,
) -> PresenceOrdering {
    match candidate.epoch.cmp(&current.epoch) {
        Ordering::Greater => PresenceOrdering::Newer,
        Ordering::Less => PresenceOrdering::Older,
        Ordering::Equal => match candidate.sequence.cmp(&current.sequence) {
            Ordering::Greater => PresenceOrdering::Newer,
            Ordering::Less => PresenceOrdering::Older,
            Ordering::Equal => {
                if candidate == current {
                    PresenceOrdering::Same
                } else {
                    PresenceOrdering::Conflict
                }
            }
        },
    }
}

fn canonical_message_bytes<T>(message: &T) -> Result<Vec<u8>, RendezvousMessageError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(message)?;
    validate_message_body_len(bytes.len())?;
    Ok(bytes)
}

fn parse_message_bytes<T>(bytes: &[u8]) -> Result<T, RendezvousMessageError>
where
    T: DeserializeOwned,
{
    validate_message_body_len(bytes.len())?;
    serde_json::from_slice(bytes).map_err(Into::into)
}

fn validate_message_body_len(body_len: usize) -> Result<(), RendezvousMessageError> {
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
        LookupNode, LookupNotFound, LookupNotFoundReason, LookupResponse, LookupResult,
        PlacementKey, PublishAck, PublishDisposition, PublishPresence, RendezvousConfig,
        RendezvousError, RendezvousMessageError, RendezvousStore, VerifiedPublishPresence,
        PLACEMENT_KEY_DOMAIN_SEPARATOR,
    };
    use crate::crypto::sign::Ed25519SigningKey;
    use crate::error::FrameError;
    use crate::identity::NodeId;
    use crate::records::{NodeRecord, PresenceRecord};
    use crate::wire::{Message, MessageType, MAX_FRAME_BODY_LEN};

    #[test]
    fn placement_key_derivation_is_domain_separated() {
        let node_id = NodeId::from_bytes([7_u8; 32]);
        let actual = PlacementKey::derive(&node_id);

        let mut hasher = blake3::Hasher::new();
        hasher.update(PLACEMENT_KEY_DOMAIN_SEPARATOR);
        hasher.update(node_id.as_bytes());
        let expected = *hasher.finalize().as_bytes();

        assert_eq!(actual.as_bytes(), &expected);
    }

    #[test]
    fn publish_and_lookup_return_fresh_result() {
        let signing_key = sample_presence_signing_key(1);
        let record = sample_signed_presence_record(&signing_key, 5, 9, 1_700_000_300);
        let node_id = record.node_id;
        let mut store = RendezvousStore::new(RendezvousConfig::default())
            .expect("default rendezvous config should be valid");

        let ack = store
            .publish_verified(
                verified_publish(record.clone(), &signing_key),
                1_700_000_000,
            )
            .expect("publish should succeed");

        assert_eq!(ack.disposition, PublishDisposition::Stored);
        assert_eq!(ack.placement_key, PlacementKey::derive(&node_id));

        let mut state = store
            .lookup_state(99)
            .expect("lookup state should cap requested budget");
        let response = store.lookup(LookupNode { node_id }, 1_700_000_010, &mut state);

        match response {
            LookupResponse::Result(result) => {
                assert_eq!(result.node_id, node_id);
                assert_eq!(result.record, record);
                assert_eq!(
                    result.remaining_budget,
                    store.config().max_lookup_budget - 1
                );
            }
            LookupResponse::NotFound(not_found) => {
                panic!("expected fresh record lookup to succeed, got {not_found:?}")
            }
        }
    }

    #[test]
    fn rendezvous_messages_expose_expected_wire_types_and_round_trip() {
        let signing_key = sample_presence_signing_key(6);
        let record = sample_signed_presence_record(&signing_key, 3, 7, 1_700_000_900);
        let node_id = record.node_id;
        let placement_key = PlacementKey::derive(&node_id);

        let publish = PublishPresence {
            record: record.clone(),
        };
        assert_eq!(PublishPresence::TYPE, MessageType::PublishPresence);
        assert_eq!(
            PublishPresence::from_canonical_bytes(
                &publish.canonical_bytes().expect("publish should serialize")
            )
            .expect("publish should deserialize"),
            publish
        );

        let ack = PublishAck {
            node_id,
            placement_key,
            disposition: PublishDisposition::Stored,
            accepted_epoch: record.epoch,
            accepted_sequence: record.sequence,
        };
        assert_eq!(PublishAck::TYPE, MessageType::PublishAck);
        assert_eq!(
            PublishAck::from_canonical_bytes(&ack.canonical_bytes().expect("ack should serialize"))
                .expect("ack should deserialize"),
            ack
        );

        let lookup = LookupNode { node_id };
        assert_eq!(LookupNode::TYPE, MessageType::LookupNode);
        assert_eq!(
            LookupNode::from_canonical_bytes(
                &lookup.canonical_bytes().expect("lookup should serialize")
            )
            .expect("lookup should deserialize"),
            lookup
        );

        let result = LookupResult {
            node_id,
            placement_key,
            record,
            remaining_budget: 5,
        };
        assert_eq!(LookupResult::TYPE, MessageType::LookupResult);
        assert_eq!(
            LookupResult::from_canonical_bytes(
                &result.canonical_bytes().expect("result should serialize")
            )
            .expect("result should deserialize"),
            result
        );

        let not_found = LookupNotFound {
            node_id,
            placement_key,
            reason: LookupNotFoundReason::NegativeCacheHit,
            remaining_budget: 4,
        };
        assert_eq!(LookupNotFound::TYPE, MessageType::LookupNotFound);
        assert_eq!(
            LookupNotFound::from_canonical_bytes(
                &not_found
                    .canonical_bytes()
                    .expect("not found should serialize")
            )
            .expect("not found should deserialize"),
            not_found
        );
    }

    #[test]
    fn rendezvous_message_vectors_match_fixture() {
        let fixture = read_rendezvous_message_vector();
        let record = fixture.record.to_presence_record();
        let node_id = record.node_id;
        let derived_placement_key = PlacementKey::derive(&node_id);

        assert_eq!(
            encode_hex(derived_placement_key.as_bytes()),
            fixture.placement_key_hex
        );

        let publish = PublishPresence {
            record: record.clone(),
        };
        assert_eq!(
            encode_hex(
                &publish
                    .canonical_bytes()
                    .expect("publish vector should serialize")
            ),
            fixture.publish_presence_hex
        );
        assert_eq!(
            PublishPresence::from_canonical_bytes(&decode_hex(&fixture.publish_presence_hex))
                .expect("publish vector should deserialize"),
            publish
        );

        let ack = PublishAck {
            node_id,
            placement_key: derived_placement_key,
            disposition: parse_publish_disposition(&fixture.publish_ack.disposition),
            accepted_epoch: fixture.publish_ack.accepted_epoch,
            accepted_sequence: fixture.publish_ack.accepted_sequence,
        };
        assert_eq!(
            encode_hex(&ack.canonical_bytes().expect("ack vector should serialize")),
            fixture.publish_ack.canonical_hex
        );
        assert_eq!(
            PublishAck::from_canonical_bytes(&decode_hex(&fixture.publish_ack.canonical_hex))
                .expect("ack vector should deserialize"),
            ack
        );

        let lookup = LookupNode { node_id };
        assert_eq!(
            encode_hex(
                &lookup
                    .canonical_bytes()
                    .expect("lookup vector should serialize")
            ),
            fixture.lookup_node_hex
        );
        assert_eq!(
            LookupNode::from_canonical_bytes(&decode_hex(&fixture.lookup_node_hex))
                .expect("lookup vector should deserialize"),
            lookup
        );

        let result = LookupResult {
            node_id,
            placement_key: derived_placement_key,
            record: record.clone(),
            remaining_budget: fixture.lookup_result.remaining_budget,
        };
        assert_eq!(
            encode_hex(
                &result
                    .canonical_bytes()
                    .expect("lookup result vector should serialize")
            ),
            fixture.lookup_result.canonical_hex
        );
        assert_eq!(
            LookupResult::from_canonical_bytes(&decode_hex(&fixture.lookup_result.canonical_hex))
                .expect("lookup result vector should deserialize"),
            result
        );

        let not_found = LookupNotFound {
            node_id,
            placement_key: derived_placement_key,
            reason: parse_lookup_not_found_reason(&fixture.lookup_not_found.reason),
            remaining_budget: fixture.lookup_not_found.remaining_budget,
        };
        assert_eq!(
            encode_hex(
                &not_found
                    .canonical_bytes()
                    .expect("lookup not found vector should serialize")
            ),
            fixture.lookup_not_found.canonical_hex
        );
        assert_eq!(
            LookupNotFound::from_canonical_bytes(&decode_hex(
                &fixture.lookup_not_found.canonical_hex
            ))
            .expect("lookup not found vector should deserialize"),
            not_found
        );
    }

    #[test]
    fn publish_presence_handoff_can_use_trusted_node_record() {
        let signing_key = sample_presence_signing_key(12);
        let record = sample_signed_presence_record(&signing_key, 4, 2, 1_700_000_900);
        let node_record = trusted_node_record(&signing_key);

        let verified = PublishPresence {
            record: record.clone(),
        }
        .verify_with_trusted_node_record(&node_record)
        .expect("trusted node record should yield a verified publish");

        assert_eq!(verified.record(), &record);
    }

    #[test]
    fn publish_presence_handoff_rejects_tampered_signature_before_store() {
        let signing_key = sample_presence_signing_key(13);
        let mut record = sample_signed_presence_record(&signing_key, 4, 2, 1_700_000_900);
        record.signature[0] ^= 0xff;

        let error = (PublishPresence { record })
            .verify_with_public_key(&signing_key.public_key())
            .expect_err("tampered signature should fail during verified handoff");

        assert!(matches!(
            error,
            crate::error::PresenceVerificationError::Crypto(
                crate::error::CryptoError::SignatureVerificationFailed
            )
        ));
    }

    #[test]
    fn stale_publish_keeps_newer_record() {
        let signing_key = sample_presence_signing_key(2);
        let first_record = sample_signed_presence_record(&signing_key, 10, 4, 1_700_000_500);
        let node_id = first_record.node_id;
        let mut store = RendezvousStore::new(RendezvousConfig::default())
            .expect("default rendezvous config should be valid");

        store
            .publish_verified(verified_publish(first_record, &signing_key), 1_700_000_000)
            .expect("initial publish should succeed");

        let stale_record = sample_signed_presence_record(&signing_key, 10, 3, 1_700_000_500);
        let stale_ack = store
            .publish_verified(verified_publish(stale_record, &signing_key), 1_700_000_010)
            .expect("stale publish should return an ack");

        assert_eq!(stale_ack.disposition, PublishDisposition::Stale);
        assert_eq!(
            store
                .published_record(&node_id)
                .expect("newer record should remain stored")
                .sequence,
            4
        );
    }

    #[test]
    fn same_epoch_and_sequence_require_identical_bytes() {
        let signing_key = sample_presence_signing_key(3);
        let mut store = RendezvousStore::new(RendezvousConfig::default())
            .expect("default rendezvous config should be valid");
        let record = sample_signed_presence_record(&signing_key, 11, 8, 1_700_000_500);
        let node_id = record.node_id;

        store
            .publish_verified(
                verified_publish(record.clone(), &signing_key),
                1_700_000_000,
            )
            .expect("initial publish should succeed");

        let mut conflicting = record;
        conflicting.locator_commitment = vec![6_u8, 6, 6];
        conflicting = sign_presence_record(conflicting, &signing_key);
        let error = store
            .publish_verified(verified_publish(conflicting, &signing_key), 1_700_000_001)
            .expect_err("conflicting equal-version record should fail");

        assert!(matches!(
            error,
            RendezvousError::ConflictingPresenceRecord {
                node_id: conflict_node_id,
                epoch: 11,
                sequence: 8,
            } if conflict_node_id == node_id
        ));
    }

    #[test]
    fn lookup_negative_cache_short_circuits_repeated_misses_and_is_cleared_by_publish() {
        let signing_key = sample_presence_signing_key(4);
        let node_id = sample_signed_presence_record(&signing_key, 1, 1, 1_700_000_500).node_id;
        let mut store = RendezvousStore::new(RendezvousConfig {
            max_negative_cache_entries: 1,
            ..RendezvousConfig::default()
        })
        .expect("config should be valid");

        let mut first_state = store
            .lookup_state(2)
            .expect("lookup state should be created");
        let first = store.lookup(LookupNode { node_id }, 1_700_000_000, &mut first_state);
        assert!(matches!(
            first,
            LookupResponse::NotFound(ref not_found)
                if not_found.reason == LookupNotFoundReason::Missing
        ));
        assert_eq!(store.negative_cache_len(), 1);

        let mut second_state = store
            .lookup_state(2)
            .expect("lookup state should be created");
        let second = store.lookup(LookupNode { node_id }, 1_700_000_001, &mut second_state);
        assert!(matches!(
            second,
            LookupResponse::NotFound(ref not_found)
                if not_found.reason == LookupNotFoundReason::NegativeCacheHit
        ));

        store
            .publish_verified(
                verified_publish(
                    sample_signed_presence_record(&signing_key, 1, 1, 1_700_000_500),
                    &signing_key,
                ),
                1_700_000_002,
            )
            .expect("publish should clear negative cache");

        assert_eq!(store.negative_cache_len(), 0);
    }

    #[test]
    fn expired_records_are_not_returned_as_fresh_results() {
        let signing_key = sample_presence_signing_key(5);
        let record = sample_signed_presence_record(&signing_key, 2, 1, 1_700_000_010);
        let node_id = record.node_id;
        let mut store = RendezvousStore::new(RendezvousConfig::default())
            .expect("default rendezvous config should be valid");

        store
            .publish_verified(verified_publish(record, &signing_key), 1_700_000_000)
            .expect("publish should succeed");

        let mut state = store
            .lookup_state(1)
            .expect("lookup state should be created");
        let response = store.lookup(LookupNode { node_id }, 1_700_000_020, &mut state);

        assert!(matches!(
            response,
            LookupResponse::NotFound(ref not_found)
                if not_found.reason == LookupNotFoundReason::Missing
        ));
        assert!(store.published_record(&node_id).is_none());
        assert!(store.negative_cache_entry(&node_id).is_some());
    }

    #[test]
    fn lookup_state_caps_budget_and_bounds_seen_helpers() {
        let helper_a = NodeId::from_bytes([9_u8; 32]);
        let helper_b = NodeId::from_bytes([10_u8; 32]);
        let helper_c = NodeId::from_bytes([11_u8; 32]);

        let mut state = super::LookupState::new(9, 4, 2).expect("lookup state should be valid");
        assert_eq!(state.remaining_budget(), 4);

        assert!(state.note_helper(helper_a).expect("first helper fits"));
        assert!(!state
            .note_helper(helper_a)
            .expect("duplicate helper is ignored"));
        assert!(state.note_helper(helper_b).expect("second helper fits"));

        let error = state
            .note_helper(helper_c)
            .expect_err("third helper should exceed the seen-set limit");
        assert!(matches!(
            error,
            RendezvousError::SeenSetLimitExceeded {
                max_lookup_seen_helpers: 2
            }
        ));

        assert_eq!(
            state.seen_helpers().copied().collect::<Vec<_>>(),
            vec![helper_a, helper_b]
        );
    }

    #[test]
    fn published_store_and_negative_cache_are_bounded() {
        let mut store = RendezvousStore::new(RendezvousConfig {
            max_published_records: 1,
            max_negative_cache_entries: 1,
            ..RendezvousConfig::default()
        })
        .expect("config should be valid");

        let signing_key_a = sample_presence_signing_key(20);
        let signing_key_b = sample_presence_signing_key(21);
        let signing_key_c = sample_presence_signing_key(22);
        let record_a = sample_signed_presence_record(&signing_key_a, 1, 1, 1_700_001_000);
        let node_a = record_a.node_id;
        let record_b = sample_signed_presence_record(&signing_key_b, 1, 1, 1_700_001_000);
        let node_b = record_b.node_id;
        let node_c = sample_signed_presence_record(&signing_key_c, 1, 1, 1_700_001_000).node_id;

        store
            .publish_verified(verified_publish(record_a, &signing_key_a), 1_700_000_000)
            .expect("first publish should succeed");
        store
            .publish_verified(verified_publish(record_b, &signing_key_b), 1_700_000_010)
            .expect("second publish should evict oldest record");

        assert_eq!(store.published_record_count(), 1);
        assert!(store.published_record(&node_a).is_none());
        assert!(store.published_record(&node_b).is_some());

        let mut first_lookup_state = store
            .lookup_state(1)
            .expect("lookup state should be created");
        let _ = store.lookup(
            LookupNode { node_id: node_a },
            1_700_000_020,
            &mut first_lookup_state,
        );
        let mut second_lookup_state = store
            .lookup_state(1)
            .expect("lookup state should be created");
        let _ = store.lookup(
            LookupNode { node_id: node_c },
            1_700_000_021,
            &mut second_lookup_state,
        );

        assert_eq!(store.negative_cache_len(), 1);
        assert!(store.negative_cache_entry(&node_a).is_none());
        assert!(store.negative_cache_entry(&node_c).is_some());
    }

    #[test]
    fn publish_presence_rejects_messages_larger_than_mvp_frame_limit() {
        let signing_key = sample_presence_signing_key(23);
        let mut record = sample_signed_presence_record(&signing_key, 1, 1, 1_700_001_000);
        record.encrypted_contact_blobs = vec![vec![7_u8; MAX_FRAME_BODY_LEN as usize]];
        let publish = PublishPresence { record };

        let error = publish
            .canonical_bytes()
            .expect_err("oversized publish should be rejected");
        assert!(matches!(
            error,
            RendezvousMessageError::Frame(FrameError::BodyTooLarge {
                max_body_len: MAX_FRAME_BODY_LEN,
                ..
            })
        ));
    }

    #[test]
    fn store_rejects_publish_that_cannot_fit_in_mvp_frame_limit() {
        let signing_key = sample_presence_signing_key(24);
        let mut record = sample_signed_presence_record(&signing_key, 1, 1, 1_700_001_000);
        record.encrypted_contact_blobs = vec![vec![7_u8; MAX_FRAME_BODY_LEN as usize]];
        record = sign_presence_record(record, &signing_key);
        let publish = PublishPresence { record };
        let mut store = RendezvousStore::new(RendezvousConfig::default())
            .expect("default rendezvous config should be valid");

        let error = store
            .publish_verified(
                publish
                    .verify_with_public_key(&signing_key.public_key())
                    .expect("signature should verify before size guard"),
                1_700_000_000,
            )
            .expect_err("oversized publish should be rejected by the store");
        assert!(matches!(
            error,
            RendezvousError::MessageEncoding(RendezvousMessageError::Frame(
                FrameError::BodyTooLarge {
                    max_body_len: MAX_FRAME_BODY_LEN,
                    ..
                }
            ))
        ));
    }

    fn sample_presence_signing_key(seed_byte: u8) -> Ed25519SigningKey {
        Ed25519SigningKey::from_seed([seed_byte; 32])
    }

    fn sample_signed_presence_record(
        signing_key: &Ed25519SigningKey,
        epoch: u64,
        sequence: u64,
        expires_at_unix_s: u64,
    ) -> PresenceRecord {
        let mut record = PresenceRecord {
            version: 1,
            node_id: crate::identity::derive_node_id(signing_key.public_key().as_bytes()),
            epoch,
            expires_at_unix_s,
            sequence,
            transport_classes: vec!["quic".to_string(), "tcp".to_string()],
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
            .expect("sample presence body should serialize");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();
        record
    }

    fn sign_presence_record(
        mut record: PresenceRecord,
        signing_key: &Ed25519SigningKey,
    ) -> PresenceRecord {
        let body = record
            .canonical_body_bytes()
            .expect("presence body should serialize before signing");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();
        record
    }

    fn verified_publish(
        record: PresenceRecord,
        signing_key: &Ed25519SigningKey,
    ) -> VerifiedPublishPresence {
        PublishPresence { record }
            .verify_with_public_key(&signing_key.public_key())
            .expect("sample publish should verify")
    }

    fn trusted_node_record(signing_key: &Ed25519SigningKey) -> NodeRecord {
        let public_key = signing_key.public_key();
        NodeRecord {
            version: 1,
            node_id: crate::identity::derive_node_id(public_key.as_bytes()),
            node_public_key: public_key.as_bytes().to_vec(),
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["tcp".to_string()],
            supported_kex: vec!["x25519".to_string()],
            supported_signatures: vec!["ed25519".to_string()],
            anti_sybil_proof: Vec::new(),
            signature: vec![1, 2, 3],
        }
    }

    #[derive(Debug, Deserialize)]
    struct RendezvousMessageVector {
        record: PresenceRecordVector,
        placement_key_hex: String,
        publish_presence_hex: String,
        publish_ack: PublishAckVector,
        lookup_node_hex: String,
        lookup_result: LookupResultVector,
        lookup_not_found: LookupNotFoundVector,
    }

    #[derive(Debug, Deserialize)]
    struct PresenceRecordVector {
        version: u8,
        node_id_hex: String,
        epoch: u64,
        expires_at_unix_s: u64,
        sequence: u64,
        transport_classes: Vec<String>,
        reachability_mode: String,
        locator_commitment_hex: String,
        encrypted_contact_blobs_hex: Vec<String>,
        relay_hint_refs_hex: Vec<String>,
        intro_policy: String,
        capability_requirements: Vec<String>,
        signature_hex: String,
    }

    impl PresenceRecordVector {
        fn to_presence_record(&self) -> PresenceRecord {
            PresenceRecord {
                version: self.version,
                node_id: NodeId::from_slice(&decode_hex(&self.node_id_hex))
                    .expect("vector node_id should be 32 bytes"),
                epoch: self.epoch,
                expires_at_unix_s: self.expires_at_unix_s,
                sequence: self.sequence,
                transport_classes: self.transport_classes.clone(),
                reachability_mode: self.reachability_mode.clone(),
                locator_commitment: decode_hex(&self.locator_commitment_hex),
                encrypted_contact_blobs: self
                    .encrypted_contact_blobs_hex
                    .iter()
                    .map(|hex| decode_hex(hex))
                    .collect(),
                relay_hint_refs: self
                    .relay_hint_refs_hex
                    .iter()
                    .map(|hex| decode_hex(hex))
                    .collect(),
                intro_policy: self.intro_policy.clone(),
                capability_requirements: self.capability_requirements.clone(),
                signature: decode_hex(&self.signature_hex),
            }
        }
    }

    #[derive(Debug, Deserialize)]
    struct PublishAckVector {
        disposition: String,
        accepted_epoch: u64,
        accepted_sequence: u64,
        canonical_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct LookupResultVector {
        remaining_budget: u8,
        canonical_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct LookupNotFoundVector {
        reason: String,
        remaining_budget: u8,
        canonical_hex: String,
    }

    fn rendezvous_message_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("rendezvous_messages.json")
    }

    fn read_rendezvous_message_vector() -> RendezvousMessageVector {
        let bytes = fs::read(rendezvous_message_vector_path())
            .expect("rendezvous message vector file should exist");
        serde_json::from_slice(&bytes).expect("rendezvous message vector file should parse")
    }

    fn parse_publish_disposition(value: &str) -> PublishDisposition {
        match value {
            "stored" => PublishDisposition::Stored,
            "replaced" => PublishDisposition::Replaced,
            "duplicate" => PublishDisposition::Duplicate,
            "stale" => PublishDisposition::Stale,
            _ => panic!("unknown publish disposition in vector: {value}"),
        }
    }

    fn parse_lookup_not_found_reason(value: &str) -> LookupNotFoundReason {
        match value {
            "missing" => LookupNotFoundReason::Missing,
            "negative_cache_hit" => LookupNotFoundReason::NegativeCacheHit,
            "budget_exhausted" => LookupNotFoundReason::BudgetExhausted,
            _ => panic!("unknown lookup not found reason in vector: {value}"),
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
