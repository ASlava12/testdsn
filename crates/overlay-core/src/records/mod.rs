use serde::{Deserialize, Serialize};

use crate::{
    error::RecordValidationError,
    identity::{derive_node_id, AppId, NodeId},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub version: u8,
    pub node_id: NodeId,
    pub node_public_key: Vec<u8>,
    pub created_at_unix_s: u64,
    pub flags: u32,
    pub supported_transports: Vec<String>,
    pub supported_kex: Vec<String>,
    pub supported_signatures: Vec<String>,
    pub anti_sybil_proof: Vec<u8>,
    pub signature: Vec<u8>,
}

impl NodeRecord {
    pub fn validate_node_id(&self) -> Result<(), RecordValidationError> {
        if self.node_id == derive_node_id(&self.node_public_key) {
            return Ok(());
        }

        Err(RecordValidationError::NodeIdMismatch)
    }

    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&NodeRecordBody {
            version: self.version,
            node_id: self.node_id,
            node_public_key: &self.node_public_key,
            created_at_unix_s: self.created_at_unix_s,
            flags: self.flags,
            supported_transports: &self.supported_transports,
            supported_kex: &self.supported_kex,
            supported_signatures: &self.supported_signatures,
            anti_sybil_proof: &self.anti_sybil_proof,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceRecord {
    pub version: u8,
    pub node_id: NodeId,
    pub epoch: u64,
    pub expires_at_unix_s: u64,
    pub sequence: u64,
    pub transport_classes: Vec<String>,
    pub reachability_mode: String,
    pub locator_commitment: Vec<u8>,
    pub encrypted_contact_blobs: Vec<Vec<u8>>,
    pub relay_hint_refs: Vec<Vec<u8>>,
    pub intro_policy: String,
    pub capability_requirements: Vec<String>,
    pub signature: Vec<u8>,
}

impl PresenceRecord {
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&PresenceRecordBody {
            version: self.version,
            node_id: self.node_id,
            epoch: self.epoch,
            expires_at_unix_s: self.expires_at_unix_s,
            sequence: self.sequence,
            transport_classes: &self.transport_classes,
            reachability_mode: &self.reachability_mode,
            locator_commitment: &self.locator_commitment,
            encrypted_contact_blobs: &self.encrypted_contact_blobs,
            relay_hint_refs: &self.relay_hint_refs,
            intro_policy: &self.intro_policy,
            capability_requirements: &self.capability_requirements,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub version: u8,
    pub node_id: NodeId,
    pub app_id: AppId,
    pub service_name: String,
    pub service_version: String,
    pub auth_mode: String,
    pub policy: Vec<u8>,
    pub reachability_ref: Vec<u8>,
    pub metadata_commitment: Vec<u8>,
    pub signature: Vec<u8>,
}

impl ServiceRecord {
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&ServiceRecordBody {
            version: self.version,
            node_id: self.node_id,
            app_id: self.app_id,
            service_name: &self.service_name,
            service_version: &self.service_version,
            auth_mode: &self.auth_mode,
            policy: &self.policy,
            reachability_ref: &self.reachability_ref,
            metadata_commitment: &self.metadata_commitment,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayHint {
    pub relay_node_id: NodeId,
    pub relay_transport_class: String,
    pub relay_score: u32,
    pub relay_policy: Vec<u8>,
    pub expiry: u64,
}

impl RelayHint {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntroTicket {
    pub ticket_id: Vec<u8>,
    pub target_node_id: NodeId,
    pub requester_binding: Vec<u8>,
    pub scope: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub nonce: Vec<u8>,
    pub signature: Vec<u8>,
}

impl IntroTicket {
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&IntroTicketBody {
            ticket_id: &self.ticket_id,
            target_node_id: self.target_node_id,
            requester_binding: &self.requester_binding,
            scope: &self.scope,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
            nonce: &self.nonce,
        })
    }
}

pub trait FreshRecord {
    fn expires_at_unix_s(&self) -> u64;

    fn validate_freshness(&self, now_unix_s: u64) -> Result<(), RecordValidationError> {
        validate_record_freshness(self.expires_at_unix_s(), now_unix_s)
    }

    fn is_fresh(&self, now_unix_s: u64) -> bool {
        self.validate_freshness(now_unix_s).is_ok()
    }
}

impl FreshRecord for PresenceRecord {
    fn expires_at_unix_s(&self) -> u64 {
        self.expires_at_unix_s
    }
}

impl FreshRecord for RelayHint {
    fn expires_at_unix_s(&self) -> u64 {
        self.expiry
    }
}

impl FreshRecord for IntroTicket {
    fn expires_at_unix_s(&self) -> u64 {
        self.expires_at_unix_s
    }
}

pub fn validate_record_freshness(
    expires_at_unix_s: u64,
    now_unix_s: u64,
) -> Result<(), RecordValidationError> {
    if expires_at_unix_s > now_unix_s {
        return Ok(());
    }

    Err(RecordValidationError::Expired {
        expires_at_unix_s,
        now_unix_s,
    })
}

// Canonical binary encoding is still open in the spec, so Milestone 1 uses
// deterministic JSON field order as a conservative placeholder for signed bodies.
#[derive(Serialize)]
struct NodeRecordBody<'a> {
    version: u8,
    node_id: NodeId,
    node_public_key: &'a [u8],
    created_at_unix_s: u64,
    flags: u32,
    supported_transports: &'a [String],
    supported_kex: &'a [String],
    supported_signatures: &'a [String],
    anti_sybil_proof: &'a [u8],
}

#[derive(Serialize)]
struct PresenceRecordBody<'a> {
    version: u8,
    node_id: NodeId,
    epoch: u64,
    expires_at_unix_s: u64,
    sequence: u64,
    transport_classes: &'a [String],
    reachability_mode: &'a str,
    locator_commitment: &'a [u8],
    encrypted_contact_blobs: &'a [Vec<u8>],
    relay_hint_refs: &'a [Vec<u8>],
    intro_policy: &'a str,
    capability_requirements: &'a [String],
}

#[derive(Serialize)]
struct ServiceRecordBody<'a> {
    version: u8,
    node_id: NodeId,
    app_id: AppId,
    service_name: &'a str,
    service_version: &'a str,
    auth_mode: &'a str,
    policy: &'a [u8],
    reachability_ref: &'a [u8],
    metadata_commitment: &'a [u8],
}

#[derive(Serialize)]
struct IntroTicketBody<'a> {
    ticket_id: &'a [u8],
    target_node_id: NodeId,
    requester_binding: &'a [u8],
    scope: &'a str,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
    nonce: &'a [u8],
}

#[cfg(test)]
mod tests {
    use super::{FreshRecord, NodeRecord, PresenceRecord};
    use crate::{error::RecordValidationError, identity::derive_node_id};

    #[test]
    fn node_record_validation_rejects_mismatched_node_id() {
        let public_key = b"node-public-key".to_vec();
        let record = NodeRecord {
            version: 1,
            node_id: derive_node_id(b"other-public-key"),
            node_public_key: public_key,
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["tcp".to_string()],
            supported_kex: vec!["x25519".to_string()],
            supported_signatures: vec!["ed25519".to_string()],
            anti_sybil_proof: vec![],
            signature: vec![1, 2, 3],
        };

        assert_eq!(
            record.validate_node_id(),
            Err(RecordValidationError::NodeIdMismatch)
        );
    }

    #[test]
    fn freshness_checks_use_strict_expiry() {
        let record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            epoch: 7,
            expires_at_unix_s: 100,
            sequence: 2,
            transport_classes: vec!["tcp".to_string()],
            reachability_mode: "direct".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["base".to_string()],
            signature: vec![1, 2, 3],
        };

        assert!(record.is_fresh(99));
        assert!(!record.is_fresh(100));
    }
}
