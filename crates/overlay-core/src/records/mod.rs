use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    crypto::sign::{
        Ed25519PublicKey, Ed25519Signature, ED25519_PUBLIC_KEY_LEN, ED25519_SIGNATURE_LEN,
    },
    error::{PresenceVerificationError, RecordEncodingError, RecordValidationError},
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

    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, RecordEncodingError> {
        let supported_transports = canonicalize_transport_classes(&self.supported_transports)?;
        let supported_kex = canonicalize_supported_kex(&self.supported_kex)?;
        let supported_signatures = canonicalize_supported_signatures(&self.supported_signatures)?;

        serde_json::to_vec(&NodeRecordBody {
            version: self.version,
            node_id: self.node_id,
            node_public_key: &self.node_public_key,
            created_at_unix_s: self.created_at_unix_s,
            flags: self.flags,
            supported_transports: &supported_transports,
            supported_kex: &supported_kex,
            supported_signatures: &supported_signatures,
            anti_sybil_proof: &self.anti_sybil_proof,
        })
        .map_err(Into::into)
    }

    pub fn ed25519_public_key(&self) -> Result<Ed25519PublicKey, PresenceVerificationError> {
        let actual = self.node_public_key.len();
        let bytes = self.node_public_key.as_slice().try_into().map_err(|_| {
            PresenceVerificationError::InvalidPublicKeyLength {
                expected: ED25519_PUBLIC_KEY_LEN,
                actual,
            }
        })?;
        Ok(Ed25519PublicKey::from_bytes(bytes))
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
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, RecordEncodingError> {
        let transport_classes = canonicalize_transport_classes(&self.transport_classes)?;
        let capability_requirements = canonicalize_capabilities(&self.capability_requirements)?;
        let reachability_mode =
            canonicalize_reachability_mode(&self.reachability_mode)?.to_string();
        let intro_policy = canonicalize_intro_policy(&self.intro_policy)?.to_string();

        serde_json::to_vec(&PresenceRecordBody {
            version: self.version,
            node_id: self.node_id,
            epoch: self.epoch,
            expires_at_unix_s: self.expires_at_unix_s,
            sequence: self.sequence,
            transport_classes: &transport_classes,
            reachability_mode: &reachability_mode,
            locator_commitment: &self.locator_commitment,
            encrypted_contact_blobs: &self.encrypted_contact_blobs,
            relay_hint_refs: &self.relay_hint_refs,
            intro_policy: &intro_policy,
            capability_requirements: &capability_requirements,
        })
        .map_err(Into::into)
    }

    pub fn verify_with_public_key(
        self,
        signer_public_key: &Ed25519PublicKey,
    ) -> Result<VerifiedPresenceRecord, PresenceVerificationError> {
        let signer_node_id = derive_node_id(signer_public_key.as_bytes());
        if self.node_id != signer_node_id {
            return Err(PresenceVerificationError::SignerNodeIdMismatch {
                record_node_id: self.node_id,
                signer_node_id,
            });
        }

        let body = self.canonical_body_bytes()?;
        signer_public_key.verify(&body, &self.ed25519_signature()?)?;
        Ok(VerifiedPresenceRecord(self))
    }

    pub fn verify_with_trusted_node_record(
        self,
        node_record: &NodeRecord,
    ) -> Result<VerifiedPresenceRecord, PresenceVerificationError> {
        node_record.validate_node_id()?;
        self.verify_with_public_key(&node_record.ed25519_public_key()?)
    }

    fn ed25519_signature(&self) -> Result<Ed25519Signature, PresenceVerificationError> {
        let actual = self.signature.len();
        let bytes = self.signature.as_slice().try_into().map_err(|_| {
            PresenceVerificationError::InvalidSignatureLength {
                expected: ED25519_SIGNATURE_LEN,
                actual,
            }
        })?;
        Ok(Ed25519Signature::from_bytes(bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPresenceRecord(PresenceRecord);

impl VerifiedPresenceRecord {
    pub fn into_inner(self) -> PresenceRecord {
        self.0
    }

    pub fn as_ref(&self) -> &PresenceRecord {
        &self.0
    }
}

impl AsRef<PresenceRecord> for VerifiedPresenceRecord {
    fn as_ref(&self) -> &PresenceRecord {
        self.as_ref()
    }
}

impl From<VerifiedPresenceRecord> for PresenceRecord {
    fn from(record: VerifiedPresenceRecord) -> Self {
        record.into_inner()
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
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, RecordEncodingError> {
        let auth_mode = canonicalize_auth_mode(&self.auth_mode)?.to_string();

        serde_json::to_vec(&ServiceRecordBody {
            version: self.version,
            node_id: self.node_id,
            app_id: self.app_id,
            service_name: &self.service_name,
            service_version: &self.service_version,
            auth_mode: &auth_mode,
            policy: &self.policy,
            reachability_ref: &self.reachability_ref,
            metadata_commitment: &self.metadata_commitment,
        })
        .map_err(Into::into)
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
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RecordEncodingError> {
        let relay_transport_class =
            canonicalize_transport_class(&self.relay_transport_class)?.to_string();

        serde_json::to_vec(&RelayHintBody {
            relay_node_id: self.relay_node_id,
            relay_transport_class: &relay_transport_class,
            relay_score: self.relay_score,
            relay_policy: &self.relay_policy,
            expiry: self.expiry,
        })
        .map_err(Into::into)
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
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, RecordEncodingError> {
        let scope = canonicalize_intro_scope(&self.scope)?.to_string();

        serde_json::to_vec(&IntroTicketBody {
            ticket_id: &self.ticket_id,
            target_node_id: self.target_node_id,
            requester_binding: &self.requester_binding,
            scope: &scope,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
            nonce: &self.nonce,
        })
        .map_err(Into::into)
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

fn canonicalize_transport_classes(
    transport_classes: &[String],
) -> Result<Vec<String>, RecordValidationError> {
    canonicalize_string_set(transport_classes, canonicalize_transport_class)
}

fn canonicalize_capabilities(
    capabilities: &[String],
) -> Result<Vec<String>, RecordValidationError> {
    canonicalize_string_set(capabilities, canonicalize_capability)
}

fn canonicalize_supported_kex(
    supported_kex: &[String],
) -> Result<Vec<String>, RecordValidationError> {
    canonicalize_string_set(supported_kex, canonicalize_supported_kex_value)
}

fn canonicalize_supported_signatures(
    supported_signatures: &[String],
) -> Result<Vec<String>, RecordValidationError> {
    canonicalize_string_set(supported_signatures, canonicalize_supported_signature_value)
}

fn canonicalize_string_set(
    values: &[String],
    validate: fn(&str) -> Result<&'static str, RecordValidationError>,
) -> Result<Vec<String>, RecordValidationError> {
    let mut normalized = BTreeSet::new();
    for value in values {
        normalized.insert(validate(value)?.to_string());
    }

    Ok(normalized.into_iter().collect())
}

fn canonicalize_transport_class(value: &str) -> Result<&'static str, RecordValidationError> {
    match value {
        "tcp" => Ok("tcp"),
        "quic" => Ok("quic"),
        "ws" => Ok("ws"),
        "relay" => Ok("relay"),
        _ => Err(RecordValidationError::UnknownTransportClass {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_capability(value: &str) -> Result<&'static str, RecordValidationError> {
    match value {
        "relay-forward" => Ok("relay-forward"),
        "relay-intro" => Ok("relay-intro"),
        "rendezvous-helper" => Ok("rendezvous-helper"),
        "bridge" => Ok("bridge"),
        "service-host" => Ok("service-host"),
        _ => Err(RecordValidationError::UnknownCapability {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_reachability_mode(value: &str) -> Result<&'static str, RecordValidationError> {
    match value {
        "direct" => Ok("direct"),
        "hybrid" => Ok("hybrid"),
        _ => Err(RecordValidationError::UnknownReachabilityMode {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_intro_policy(value: &str) -> Result<&'static str, RecordValidationError> {
    match value {
        "allow" => Ok("allow"),
        _ => Err(RecordValidationError::UnknownIntroPolicy {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_supported_kex_value(value: &str) -> Result<&'static str, RecordValidationError> {
    match value {
        "x25519" => Ok("x25519"),
        _ => Err(RecordValidationError::UnknownKeyExchange {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_supported_signature_value(
    value: &str,
) -> Result<&'static str, RecordValidationError> {
    match value {
        "ed25519" => Ok("ed25519"),
        _ => Err(RecordValidationError::UnknownSignatureAlgorithm {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_auth_mode(value: &str) -> Result<&'static str, RecordValidationError> {
    match value {
        "none" => Ok("none"),
        _ => Err(RecordValidationError::UnknownAuthMode {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_intro_scope(value: &str) -> Result<&'static str, RecordValidationError> {
    match value {
        "relay-intro" => Ok("relay-intro"),
        _ => Err(RecordValidationError::UnknownIntroScope {
            value: value.to_string(),
        }),
    }
}

// MVP canonical body encoding is locked to deterministic JSON bytes in
// `docs/OPEN_QUESTIONS.md`, so signed record bodies use field-order JSON here.
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
struct RelayHintBody<'a> {
    relay_node_id: NodeId,
    relay_transport_class: &'a str,
    relay_score: u32,
    relay_policy: &'a [u8],
    expiry: u64,
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
    use std::{fs, path::PathBuf};

    use serde::Deserialize;

    use super::{FreshRecord, IntroTicket, NodeRecord, PresenceRecord, RelayHint, ServiceRecord};
    use crate::{
        crypto::sign::Ed25519SigningKey,
        error::{PresenceVerificationError, RecordEncodingError, RecordValidationError},
        identity::{derive_app_id, derive_node_id, AppId, NodeId},
    };

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
            capability_requirements: vec!["bridge".to_string()],
            signature: vec![1, 2, 3],
        };

        assert!(record.is_fresh(99));
        assert!(!record.is_fresh(100));
    }

    #[test]
    fn presence_record_verifies_with_matching_public_key() {
        let signing_key = Ed25519SigningKey::from_seed([7_u8; 32]);
        let public_key = signing_key.public_key();
        let mut record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(public_key.as_bytes()),
            epoch: 7,
            expires_at_unix_s: 100,
            sequence: 2,
            transport_classes: vec!["tcp".to_string()],
            reachability_mode: "direct".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["bridge".to_string()],
            signature: Vec::new(),
        };
        let body = record
            .canonical_body_bytes()
            .expect("presence body should serialize");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();

        let verified = record
            .clone()
            .verify_with_public_key(&public_key)
            .expect("presence signature should verify");

        assert_eq!(verified.into_inner(), record);
    }

    #[test]
    fn presence_record_rejects_mismatched_signer_public_key() {
        let signing_key = Ed25519SigningKey::from_seed([7_u8; 32]);
        let wrong_public_key = Ed25519SigningKey::from_seed([8_u8; 32]).public_key();
        let mut record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(signing_key.public_key().as_bytes()),
            epoch: 7,
            expires_at_unix_s: 100,
            sequence: 2,
            transport_classes: vec!["tcp".to_string()],
            reachability_mode: "direct".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["bridge".to_string()],
            signature: Vec::new(),
        };
        let body = record
            .canonical_body_bytes()
            .expect("presence body should serialize");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();

        let error = record
            .verify_with_public_key(&wrong_public_key)
            .expect_err("mismatched signer should be rejected");

        assert!(matches!(
            error,
            PresenceVerificationError::SignerNodeIdMismatch {
                record_node_id,
                signer_node_id,
            } if record_node_id == derive_node_id(signing_key.public_key().as_bytes())
                && signer_node_id == derive_node_id(wrong_public_key.as_bytes())
        ));
    }

    #[test]
    fn presence_record_rejects_invalid_signature_length() {
        let public_key = Ed25519SigningKey::from_seed([7_u8; 32]).public_key();
        let record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(public_key.as_bytes()),
            epoch: 7,
            expires_at_unix_s: 100,
            sequence: 2,
            transport_classes: vec!["tcp".to_string()],
            reachability_mode: "direct".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["bridge".to_string()],
            signature: vec![1, 2, 3],
        };

        let error = record
            .verify_with_public_key(&public_key)
            .expect_err("invalid signature length should be rejected");
        assert!(matches!(
            error,
            PresenceVerificationError::InvalidSignatureLength {
                expected: 64,
                actual: 3
            }
        ));
    }

    #[test]
    fn presence_record_verifies_with_trusted_node_record() {
        let signing_key = Ed25519SigningKey::from_seed([7_u8; 32]);
        let public_key = signing_key.public_key();
        let mut record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(public_key.as_bytes()),
            epoch: 7,
            expires_at_unix_s: 100,
            sequence: 2,
            transport_classes: vec!["tcp".to_string()],
            reachability_mode: "direct".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["bridge".to_string()],
            signature: Vec::new(),
        };
        let body = record
            .canonical_body_bytes()
            .expect("presence body should serialize");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();
        let trusted_node_record = NodeRecord {
            version: 1,
            node_id: derive_node_id(public_key.as_bytes()),
            node_public_key: public_key.as_bytes().to_vec(),
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["tcp".to_string()],
            supported_kex: vec!["x25519".to_string()],
            supported_signatures: vec!["ed25519".to_string()],
            anti_sybil_proof: vec![],
            signature: vec![1, 2, 3],
        };

        let verified = record
            .clone()
            .verify_with_trusted_node_record(&trusted_node_record)
            .expect("trusted node record should supply signer public key");

        assert_eq!(verified.into_inner(), record);
    }

    #[test]
    fn node_record_canonical_body_bytes_sort_and_deduplicate_supported_transports() {
        let record = NodeRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            node_public_key: b"node-public-key".to_vec(),
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["ws".to_string(), "tcp".to_string(), "ws".to_string()],
            supported_kex: vec!["x25519".to_string()],
            supported_signatures: vec!["ed25519".to_string()],
            anti_sybil_proof: vec![],
            signature: vec![1, 2, 3],
        };

        let canonical_body = String::from_utf8(
            record
                .canonical_body_bytes()
                .expect("canonical node body should serialize"),
        )
        .expect("canonical body should be utf-8 json");

        assert!(canonical_body.contains("\"supported_transports\":[\"tcp\",\"ws\"]"));
    }

    #[test]
    fn node_record_canonical_body_bytes_sort_and_deduplicate_kex_and_signatures() {
        let record = NodeRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            node_public_key: b"node-public-key".to_vec(),
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["tcp".to_string()],
            supported_kex: vec!["x25519".to_string(), "x25519".to_string()],
            supported_signatures: vec!["ed25519".to_string(), "ed25519".to_string()],
            anti_sybil_proof: vec![],
            signature: vec![1, 2, 3],
        };

        let canonical_body = String::from_utf8(
            record
                .canonical_body_bytes()
                .expect("canonical node body should serialize"),
        )
        .expect("canonical body should be utf-8 json");

        assert!(canonical_body.contains("\"supported_kex\":[\"x25519\"]"));
        assert!(canonical_body.contains("\"supported_signatures\":[\"ed25519\"]"));
    }

    #[test]
    fn node_record_canonical_body_bytes_reject_unknown_kex() {
        let record = NodeRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            node_public_key: b"node-public-key".to_vec(),
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["tcp".to_string()],
            supported_kex: vec!["kyber".to_string()],
            supported_signatures: vec!["ed25519".to_string()],
            anti_sybil_proof: vec![],
            signature: vec![1, 2, 3],
        };

        let error = record
            .canonical_body_bytes()
            .expect_err("unknown key exchange values must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(RecordValidationError::UnknownKeyExchange { value })
            if value == "kyber"
        ));
    }

    #[test]
    fn node_record_canonical_body_bytes_reject_unknown_signature_algorithm() {
        let record = NodeRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            node_public_key: b"node-public-key".to_vec(),
            created_at_unix_s: 1,
            flags: 0,
            supported_transports: vec!["tcp".to_string()],
            supported_kex: vec!["x25519".to_string()],
            supported_signatures: vec!["rsa".to_string()],
            anti_sybil_proof: vec![],
            signature: vec![1, 2, 3],
        };

        let error = record
            .canonical_body_bytes()
            .expect_err("unknown signature algorithms must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(
                RecordValidationError::UnknownSignatureAlgorithm { value }
            ) if value == "rsa"
        ));
    }

    #[test]
    fn node_record_vector_matches_fixture() {
        let vector = read_node_record_vector();
        let public_key = decode_hex(&vector.node_public_key_hex);
        let expected_node_id = NodeId::from_slice(&decode_hex(&vector.node_id_hex))
            .expect("node id vector must contain 32-byte ids");

        assert_eq!(derive_node_id(&public_key), expected_node_id);

        let record = NodeRecord {
            version: vector.version,
            node_id: expected_node_id,
            node_public_key: public_key,
            created_at_unix_s: vector.created_at_unix_s,
            flags: vector.flags,
            supported_transports: vector.supported_transports,
            supported_kex: vector.supported_kex,
            supported_signatures: vector.supported_signatures,
            anti_sybil_proof: decode_hex(&vector.anti_sybil_proof_hex),
            signature: decode_hex(&vector.signature_hex),
        };

        let canonical_body_hex = encode_hex(
            &record
                .canonical_body_bytes()
                .expect("node record vector should serialize"),
        );
        assert_eq!(canonical_body_hex, vector.canonical_body_hex);
    }

    #[test]
    fn service_record_vector_matches_fixture() {
        let vector = read_service_record_vector();
        let public_key = decode_hex(&vector.node_public_key_hex);
        let expected_node_id = NodeId::from_slice(&decode_hex(&vector.node_id_hex))
            .expect("node id vector must contain 32-byte ids");
        let expected_app_id = AppId::from_slice(&decode_hex(&vector.app_id_hex))
            .expect("app id vector must contain 32-byte ids");

        assert_eq!(derive_node_id(&public_key), expected_node_id);
        assert_eq!(
            derive_app_id(&expected_node_id, &vector.app_namespace, &vector.app_name),
            expected_app_id
        );

        let record = ServiceRecord {
            version: vector.version,
            node_id: expected_node_id,
            app_id: expected_app_id,
            service_name: vector.service_name,
            service_version: vector.service_version,
            auth_mode: vector.auth_mode,
            policy: decode_hex(&vector.policy_hex),
            reachability_ref: decode_hex(&vector.reachability_ref_hex),
            metadata_commitment: decode_hex(&vector.metadata_commitment_hex),
            signature: decode_hex(&vector.signature_hex),
        };

        let canonical_body_hex = encode_hex(
            &record
                .canonical_body_bytes()
                .expect("service record vector should serialize"),
        );
        assert_eq!(canonical_body_hex, vector.canonical_body_hex);
    }

    #[test]
    fn service_record_canonical_body_bytes_reject_unknown_auth_mode() {
        let node_id = derive_node_id(b"node-public-key");
        let record = ServiceRecord {
            version: 1,
            node_id,
            app_id: derive_app_id(&node_id, "chat", "terminal"),
            service_name: "terminal".to_string(),
            service_version: "1.0.0".to_string(),
            auth_mode: "password".to_string(),
            policy: vec![1, 2, 3, 4],
            reachability_ref: vec![0xAA, 0xBB],
            metadata_commitment: vec![0xCC, 0xDD],
            signature: vec![0x11, 0x22, 0x33],
        };

        let error = record
            .canonical_body_bytes()
            .expect_err("unknown auth modes must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(RecordValidationError::UnknownAuthMode { value })
            if value == "password"
        ));
    }

    #[test]
    fn relay_hint_vector_matches_fixture() {
        let vector = read_relay_hint_vector();
        let relay_public_key = decode_hex(&vector.relay_node_public_key_hex);
        let expected_relay_node_id = NodeId::from_slice(&decode_hex(&vector.relay_node_id_hex))
            .expect("relay node id vector must contain 32-byte ids");

        assert_eq!(derive_node_id(&relay_public_key), expected_relay_node_id);

        let relay_hint = RelayHint {
            relay_node_id: expected_relay_node_id,
            relay_transport_class: vector.relay_transport_class,
            relay_score: vector.relay_score,
            relay_policy: decode_hex(&vector.relay_policy_hex),
            expiry: vector.expiry,
        };

        let canonical_body_hex = encode_hex(
            &relay_hint
                .canonical_bytes()
                .expect("relay hint vector should serialize"),
        );
        assert_eq!(canonical_body_hex, vector.canonical_body_hex);
        assert!(relay_hint.is_fresh(700));
    }

    #[test]
    fn intro_ticket_vector_matches_fixture() {
        let vector = read_intro_ticket_vector();
        let target_public_key = decode_hex(&vector.target_node_public_key_hex);
        let expected_target_node_id = NodeId::from_slice(&decode_hex(&vector.target_node_id_hex))
            .expect("target node id vector must contain 32-byte ids");

        assert_eq!(derive_node_id(&target_public_key), expected_target_node_id);

        let intro_ticket = IntroTicket {
            ticket_id: decode_hex(&vector.ticket_id_hex),
            target_node_id: expected_target_node_id,
            requester_binding: decode_hex(&vector.requester_binding_hex),
            scope: vector.scope,
            issued_at_unix_s: vector.issued_at_unix_s,
            expires_at_unix_s: vector.expires_at_unix_s,
            nonce: decode_hex(&vector.nonce_hex),
            signature: decode_hex(&vector.signature_hex),
        };

        let canonical_body_hex = encode_hex(
            &intro_ticket
                .canonical_body_bytes()
                .expect("intro ticket vector should serialize"),
        );
        assert_eq!(canonical_body_hex, vector.canonical_body_hex);
        assert!(intro_ticket.is_fresh(1_500));
    }

    #[test]
    fn intro_ticket_canonical_body_bytes_reject_unknown_scope() {
        let intro_ticket = IntroTicket {
            ticket_id: vec![0x01, 0x23, 0x45, 0x67],
            target_node_id: derive_node_id(b"node-public-key"),
            requester_binding: vec![0x89, 0xAB, 0xCD],
            scope: "full-access".to_string(),
            issued_at_unix_s: 1_000,
            expires_at_unix_s: 2_000,
            nonce: vec![0x10, 0x20, 0x30, 0x40],
            signature: vec![0x50, 0x60, 0x70],
        };

        let error = intro_ticket
            .canonical_body_bytes()
            .expect_err("unknown intro scopes must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(RecordValidationError::UnknownIntroScope { value })
            if value == "full-access"
        ));
    }

    #[test]
    fn presence_record_vector_matches_fixture() {
        let vector = read_presence_record_vector();
        let public_key = decode_hex(&vector.node_public_key_hex);
        let expected_node_id = NodeId::from_slice(&decode_hex(&vector.node_id_hex))
            .expect("node id vector must contain 32-byte ids");

        assert_eq!(derive_node_id(&public_key), expected_node_id);

        let record = PresenceRecord {
            version: vector.version,
            node_id: expected_node_id,
            epoch: vector.epoch,
            expires_at_unix_s: vector.expires_at_unix_s,
            sequence: vector.sequence,
            transport_classes: vector.transport_classes,
            reachability_mode: vector.reachability_mode,
            locator_commitment: decode_hex(&vector.locator_commitment_hex),
            encrypted_contact_blobs: vector
                .encrypted_contact_blobs_hex
                .iter()
                .map(|value| decode_hex(value))
                .collect(),
            relay_hint_refs: vector
                .relay_hint_refs_hex
                .iter()
                .map(|value| decode_hex(value))
                .collect(),
            intro_policy: vector.intro_policy,
            capability_requirements: vector.capability_requirements,
            signature: decode_hex(&vector.signature_hex),
        };

        let canonical_body_hex = encode_hex(
            &record
                .canonical_body_bytes()
                .expect("presence record vector should serialize"),
        );
        assert_eq!(canonical_body_hex, vector.canonical_body_hex);
        assert!(record.is_fresh(1_000_000_000));
    }

    #[test]
    fn canonical_body_bytes_sort_and_deduplicate_transport_classes_and_capabilities() {
        let record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            epoch: 7,
            expires_at_unix_s: 1_000,
            sequence: 2,
            transport_classes: vec!["tcp".to_string(), "relay".to_string(), "tcp".to_string()],
            reachability_mode: "hybrid".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec![
                "service-host".to_string(),
                "bridge".to_string(),
                "bridge".to_string(),
            ],
            signature: vec![1, 2, 3],
        };

        let canonical_body = String::from_utf8(
            record
                .canonical_body_bytes()
                .expect("canonical presence body should serialize"),
        )
        .expect("canonical body should be utf-8 json");

        assert!(canonical_body.contains("\"transport_classes\":[\"relay\",\"tcp\"]"));
        assert!(
            canonical_body.contains("\"capability_requirements\":[\"bridge\",\"service-host\"]")
        );
    }

    #[test]
    fn presence_record_canonical_body_bytes_reject_unknown_reachability_mode() {
        let record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            epoch: 7,
            expires_at_unix_s: 1_000,
            sequence: 2,
            transport_classes: vec!["relay".to_string()],
            reachability_mode: "relay-only".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["bridge".to_string()],
            signature: vec![1, 2, 3],
        };

        let error = record
            .canonical_body_bytes()
            .expect_err("unknown reachability modes must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(
                RecordValidationError::UnknownReachabilityMode { value }
            ) if value == "relay-only"
        ));
    }

    #[test]
    fn presence_record_canonical_body_bytes_reject_unknown_intro_policy() {
        let record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            epoch: 7,
            expires_at_unix_s: 1_000,
            sequence: 2,
            transport_classes: vec!["relay".to_string()],
            reachability_mode: "hybrid".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "deny".to_string(),
            capability_requirements: vec!["bridge".to_string()],
            signature: vec![1, 2, 3],
        };

        let error = record
            .canonical_body_bytes()
            .expect_err("unknown intro policies must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(RecordValidationError::UnknownIntroPolicy { value })
            if value == "deny"
        ));
    }

    #[test]
    fn canonical_body_bytes_reject_unknown_transport_classes() {
        let record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            epoch: 7,
            expires_at_unix_s: 1_000,
            sequence: 2,
            transport_classes: vec!["smtp".to_string()],
            reachability_mode: "hybrid".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["bridge".to_string()],
            signature: vec![1, 2, 3],
        };

        let error = record
            .canonical_body_bytes()
            .expect_err("unknown transport classes must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(RecordValidationError::UnknownTransportClass { value })
            if value == "smtp"
        ));
    }

    #[test]
    fn canonical_body_bytes_reject_unknown_capabilities() {
        let record = PresenceRecord {
            version: 1,
            node_id: derive_node_id(b"node-public-key"),
            epoch: 7,
            expires_at_unix_s: 1_000,
            sequence: 2,
            transport_classes: vec!["relay".to_string()],
            reachability_mode: "hybrid".to_string(),
            locator_commitment: vec![1, 2, 3],
            encrypted_contact_blobs: vec![vec![4, 5, 6]],
            relay_hint_refs: vec![vec![7, 8, 9]],
            intro_policy: "allow".to_string(),
            capability_requirements: vec!["base".to_string()],
            signature: vec![1, 2, 3],
        };

        let error = record
            .canonical_body_bytes()
            .expect_err("unknown capabilities must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(RecordValidationError::UnknownCapability { value })
            if value == "base"
        ));
    }

    #[test]
    fn relay_hint_canonical_bytes_reject_unknown_transport_class() {
        let relay_hint = RelayHint {
            relay_node_id: derive_node_id(b"relay-node-public-key"),
            relay_transport_class: "smtp".to_string(),
            relay_score: 7,
            relay_policy: vec![1, 2, 3],
            expiry: 1_000,
        };

        let error = relay_hint
            .canonical_bytes()
            .expect_err("unknown relay transport classes must be rejected");
        assert!(matches!(
            error,
            RecordEncodingError::Validation(RecordValidationError::UnknownTransportClass { value })
            if value == "smtp"
        ));
    }

    #[derive(Debug, Deserialize)]
    struct NodeRecordVector {
        version: u8,
        node_public_key_hex: String,
        node_id_hex: String,
        created_at_unix_s: u64,
        flags: u32,
        supported_transports: Vec<String>,
        supported_kex: Vec<String>,
        supported_signatures: Vec<String>,
        anti_sybil_proof_hex: String,
        signature_hex: String,
        canonical_body_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct ServiceRecordVector {
        version: u8,
        node_public_key_hex: String,
        node_id_hex: String,
        app_namespace: String,
        app_name: String,
        app_id_hex: String,
        service_name: String,
        service_version: String,
        auth_mode: String,
        policy_hex: String,
        reachability_ref_hex: String,
        metadata_commitment_hex: String,
        signature_hex: String,
        canonical_body_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct RelayHintVector {
        relay_node_public_key_hex: String,
        relay_node_id_hex: String,
        relay_transport_class: String,
        relay_score: u32,
        relay_policy_hex: String,
        expiry: u64,
        canonical_body_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct IntroTicketVector {
        target_node_public_key_hex: String,
        target_node_id_hex: String,
        ticket_id_hex: String,
        requester_binding_hex: String,
        scope: String,
        issued_at_unix_s: u64,
        expires_at_unix_s: u64,
        nonce_hex: String,
        signature_hex: String,
        canonical_body_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct PresenceRecordVector {
        node_public_key_hex: String,
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
        canonical_body_hex: String,
    }

    fn presence_record_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("presence_record.json")
    }

    fn node_record_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("node_record.json")
    }

    fn service_record_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("service_record.json")
    }

    fn relay_hint_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("relay_hint.json")
    }

    fn intro_ticket_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("intro_ticket.json")
    }

    fn read_node_record_vector() -> NodeRecordVector {
        let bytes =
            fs::read(node_record_vector_path()).expect("node record vector file should exist");
        serde_json::from_slice(&bytes).expect("node record vector file should parse")
    }

    fn read_service_record_vector() -> ServiceRecordVector {
        let bytes = fs::read(service_record_vector_path())
            .expect("service record vector file should exist");
        serde_json::from_slice(&bytes).expect("service record vector file should parse")
    }

    fn read_relay_hint_vector() -> RelayHintVector {
        let bytes =
            fs::read(relay_hint_vector_path()).expect("relay hint vector file should exist");
        serde_json::from_slice(&bytes).expect("relay hint vector file should parse")
    }

    fn read_intro_ticket_vector() -> IntroTicketVector {
        let bytes =
            fs::read(intro_ticket_vector_path()).expect("intro ticket vector file should exist");
        serde_json::from_slice(&bytes).expect("intro ticket vector file should parse")
    }

    fn read_presence_record_vector() -> PresenceRecordVector {
        let bytes = fs::read(presence_record_vector_path())
            .expect("presence record vector file should exist");
        serde_json::from_slice(&bytes).expect("presence record vector file should parse")
    }

    fn decode_hex(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0, "hex input must have even length");

        hex.as_bytes()
            .chunks_exact(2)
            .map(|chunk| {
                let text = std::str::from_utf8(chunk).expect("hex input must be utf-8");
                u8::from_str_radix(text, 16).expect("hex input must be valid")
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
