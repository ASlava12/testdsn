use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    crypto::sign::{Ed25519PublicKey, Ed25519Signature, Ed25519SigningKey},
    error::CryptoError,
    identity::NodeId,
    metrics::{LogComponent, LogContext, Observability},
    session::HANDSHAKE_VERSION,
    wire::MAX_FRAME_BODY_LEN,
};

pub const BOOTSTRAP_SCHEMA_VERSION: u8 = 1;
pub const SIGNED_BOOTSTRAP_ARTIFACT_VERSION: u8 = 1;
const SIGNED_BOOTSTRAP_ARTIFACT_CONTEXT: &[u8] = b"overlay-bootstrap-artifact-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapNetworkParams {
    pub network_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapPeerRole {
    Standard,
    Relay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapPeer {
    pub node_id: NodeId,
    pub transport_classes: Vec<String>,
    pub capabilities: Vec<String>,
    pub dial_hints: Vec<String>,
    pub observed_role: BootstrapPeerRole,
}

impl BootstrapPeer {
    pub fn is_relay_capable(&self) -> bool {
        self.observed_role == BootstrapPeerRole::Relay
            || self
                .capabilities
                .iter()
                .any(|capability| capability == "relay-forward" || capability == "relay-intro")
            || self.transport_classes.iter().any(|class| class == "relay")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeHint {
    pub transport_class: String,
    pub dial_hint: String,
    pub capabilities: Vec<String>,
    pub expires_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapResponse {
    pub version: u8,
    pub generated_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub network_params: BootstrapNetworkParams,
    pub epoch_duration_s: u64,
    pub presence_ttl_s: u64,
    pub max_frame_body_len: u32,
    pub handshake_version: u8,
    pub peers: Vec<BootstrapPeer>,
    pub bridge_hints: Vec<BridgeHint>,
}

impl BootstrapResponse {
    pub fn validated(mut self, now_unix_s: u64) -> Result<Self, BootstrapValidationError> {
        if self.version != BOOTSTRAP_SCHEMA_VERSION {
            return Err(BootstrapValidationError::UnsupportedSchemaVersion {
                expected: BOOTSTRAP_SCHEMA_VERSION,
                actual: self.version,
            });
        }
        if self.generated_at_unix_s > self.expires_at_unix_s {
            return Err(BootstrapValidationError::GeneratedAfterExpiry {
                generated_at_unix_s: self.generated_at_unix_s,
                expires_at_unix_s: self.expires_at_unix_s,
            });
        }
        if self.expires_at_unix_s <= now_unix_s {
            return Err(BootstrapValidationError::Expired {
                expires_at_unix_s: self.expires_at_unix_s,
                now_unix_s,
            });
        }
        if self.network_params.network_id.trim().is_empty() {
            return Err(BootstrapValidationError::EmptyNetworkId);
        }
        if self.epoch_duration_s == 0 {
            return Err(BootstrapValidationError::ZeroField {
                field: "epoch_duration_s",
            });
        }
        if self.presence_ttl_s == 0 {
            return Err(BootstrapValidationError::ZeroField {
                field: "presence_ttl_s",
            });
        }
        if self.max_frame_body_len == 0 {
            return Err(BootstrapValidationError::ZeroField {
                field: "max_frame_body_len",
            });
        }
        if self.max_frame_body_len > MAX_FRAME_BODY_LEN {
            return Err(BootstrapValidationError::FrameBodyTooLarge {
                max_frame_body_len: self.max_frame_body_len,
                allowed_max_frame_body_len: MAX_FRAME_BODY_LEN,
            });
        }
        if self.handshake_version != HANDSHAKE_VERSION {
            return Err(BootstrapValidationError::UnsupportedHandshakeVersion {
                expected: HANDSHAKE_VERSION,
                actual: self.handshake_version,
            });
        }

        let mut seen_peer_node_ids = BTreeSet::new();
        for peer in &mut self.peers {
            if !seen_peer_node_ids.insert(peer.node_id) {
                return Err(BootstrapValidationError::DuplicatePeerNodeId {
                    node_id: peer.node_id,
                });
            }
            peer.transport_classes = canonicalize_transport_classes(&peer.transport_classes)?;
            peer.capabilities = canonicalize_capabilities(&peer.capabilities)?;
            canonicalize_dial_hints(&mut peer.dial_hints)?;
            if peer.transport_classes.is_empty() {
                return Err(BootstrapValidationError::PeerWithoutTransportClasses {
                    node_id: peer.node_id,
                });
            }
            if peer.dial_hints.is_empty() {
                return Err(BootstrapValidationError::PeerWithoutDialHints {
                    node_id: peer.node_id,
                });
            }
        }

        let mut seen_bridge_hints = BTreeSet::new();
        for hint in &mut self.bridge_hints {
            hint.transport_class = canonicalize_transport_class(&hint.transport_class)?.to_string();
            hint.capabilities = canonicalize_capabilities(&hint.capabilities)?;
            canonicalize_single_dial_hint(&mut hint.dial_hint, "bridge_hints[].dial_hint")?;
            if !seen_bridge_hints.insert((hint.transport_class.clone(), hint.dial_hint.clone())) {
                return Err(BootstrapValidationError::DuplicateBridgeHint {
                    transport_class: hint.transport_class.clone(),
                    dial_hint: hint.dial_hint.clone(),
                });
            }
            if hint.expires_at_unix_s <= now_unix_s {
                return Err(BootstrapValidationError::ExpiredBridgeHint {
                    expires_at_unix_s: hint.expires_at_unix_s,
                    now_unix_s,
                });
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedBootstrapArtifact {
    pub version: u8,
    pub signer_public_key: Ed25519PublicKey,
    pub bootstrap_response: BootstrapResponse,
    pub signature: Ed25519Signature,
}

impl SignedBootstrapArtifact {
    pub fn sign(
        bootstrap_response: BootstrapResponse,
        signing_key: &Ed25519SigningKey,
    ) -> Result<Self, SignedBootstrapArtifactError> {
        let signer_public_key = signing_key.public_key();
        let signature_input = signed_bootstrap_artifact_signature_input(
            SIGNED_BOOTSTRAP_ARTIFACT_VERSION,
            signer_public_key,
            &bootstrap_response,
        )?;
        Ok(Self {
            version: SIGNED_BOOTSTRAP_ARTIFACT_VERSION,
            signer_public_key,
            bootstrap_response,
            signature: signing_key.sign(&signature_input),
        })
    }

    pub fn verify_with_trusted_signer(
        &self,
        trusted_signer: &Ed25519PublicKey,
    ) -> Result<BootstrapResponse, SignedBootstrapArtifactError> {
        if self.version != SIGNED_BOOTSTRAP_ARTIFACT_VERSION {
            return Err(SignedBootstrapArtifactError::UnsupportedVersion {
                expected: SIGNED_BOOTSTRAP_ARTIFACT_VERSION,
                actual: self.version,
            });
        }
        if &self.signer_public_key != trusted_signer {
            return Err(SignedBootstrapArtifactError::TrustedSignerMismatch {
                expected: *trusted_signer,
                actual: self.signer_public_key,
            });
        }

        let signature_input = signed_bootstrap_artifact_signature_input(
            self.version,
            self.signer_public_key,
            &self.bootstrap_response,
        )?;
        self.signer_public_key
            .verify(&signature_input, &self.signature)?;
        Ok(self.bootstrap_response.clone())
    }
}

#[derive(Debug, Error)]
pub enum SignedBootstrapArtifactError {
    #[error("unsupported signed bootstrap artifact version: expected {expected}, got {actual}")]
    UnsupportedVersion { expected: u8, actual: u8 },
    #[error("signed bootstrap artifact signer mismatch between trusted and artifact key")]
    TrustedSignerMismatch {
        expected: Ed25519PublicKey,
        actual: Ed25519PublicKey,
    },
    #[error("failed to encode signed bootstrap artifact signature input: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BootstrapValidationError {
    #[error("unsupported bootstrap schema version: expected {expected}, got {actual}")]
    UnsupportedSchemaVersion { expected: u8, actual: u8 },
    #[error(
        "bootstrap response generated_at_unix_s ({generated_at_unix_s}) exceeds expires_at_unix_s ({expires_at_unix_s})"
    )]
    GeneratedAfterExpiry {
        generated_at_unix_s: u64,
        expires_at_unix_s: u64,
    },
    #[error("bootstrap response expired at {expires_at_unix_s}, now is {now_unix_s}")]
    Expired {
        expires_at_unix_s: u64,
        now_unix_s: u64,
    },
    #[error("bootstrap network_id must not be empty")]
    EmptyNetworkId,
    #[error("bootstrap field {field} must be non-zero")]
    ZeroField { field: &'static str },
    #[error(
        "bootstrap max_frame_body_len {max_frame_body_len} exceeds allowed MVP limit {allowed_max_frame_body_len}"
    )]
    FrameBodyTooLarge {
        max_frame_body_len: u32,
        allowed_max_frame_body_len: u32,
    },
    #[error("unsupported bootstrap handshake version: expected {expected}, got {actual}")]
    UnsupportedHandshakeVersion { expected: u8, actual: u8 },
    #[error("unknown transport class in bootstrap response: {value}")]
    UnknownTransportClass { value: String },
    #[error("unknown capability in bootstrap response: {value}")]
    UnknownCapability { value: String },
    #[error("bootstrap response contains duplicate peer entry for node {node_id}")]
    DuplicatePeerNodeId { node_id: NodeId },
    #[error("bootstrap peer {node_id} has no transport classes")]
    PeerWithoutTransportClasses { node_id: NodeId },
    #[error("bootstrap peer {node_id} has no dial hints")]
    PeerWithoutDialHints { node_id: NodeId },
    #[error("{field} must not be empty")]
    EmptyDialHint { field: &'static str },
    #[error(
        "bootstrap response contains duplicate bridge hint for transport {transport_class} and dial hint {dial_hint}"
    )]
    DuplicateBridgeHint {
        transport_class: String,
        dial_hint: String,
    },
    #[error("bridge hint expired at {expires_at_unix_s}, now is {now_unix_s}")]
    ExpiredBridgeHint {
        expires_at_unix_s: u64,
        now_unix_s: u64,
    },
}

#[derive(Debug, Error)]
pub enum BootstrapProviderError {
    #[error("bootstrap provider is unavailable: {0}")]
    Unavailable(String),
    #[error("bootstrap provider artifact integrity check failed: {0}")]
    Integrity(String),
    #[error("bootstrap provider trust verification failed: {0}")]
    Trust(String),
    #[error(transparent)]
    Validation(#[from] BootstrapValidationError),
}

pub trait BootstrapProvider {
    fn provider_name(&self) -> &'static str;

    fn fetch_response(&self) -> Result<BootstrapResponse, BootstrapProviderError>;

    fn fetch_validated_response(
        &self,
        now_unix_s: u64,
    ) -> Result<BootstrapResponse, BootstrapProviderError> {
        self.fetch_response()?
            .validated(now_unix_s)
            .map_err(Into::into)
    }

    fn fetch_validated_response_with_observability(
        &self,
        now_unix_s: u64,
        observability: &mut Observability,
        context: LogContext,
    ) -> Result<BootstrapResponse, BootstrapProviderError> {
        match self.fetch_validated_response(now_unix_s) {
            Ok(response) => {
                observability.push_log(
                    context,
                    LogComponent::Bootstrap,
                    "bootstrap_fetch",
                    "accepted",
                );
                Ok(response)
            }
            Err(error) => {
                observability.push_log(
                    context,
                    LogComponent::Bootstrap,
                    "bootstrap_fetch",
                    bootstrap_provider_error_log_result(&error),
                );
                Err(error)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct StaticBootstrapProvider {
    response: BootstrapResponse,
}

impl StaticBootstrapProvider {
    pub fn new(response: BootstrapResponse) -> Self {
        Self { response }
    }
}

impl BootstrapProvider for StaticBootstrapProvider {
    fn provider_name(&self) -> &'static str {
        "static"
    }

    fn fetch_response(&self) -> Result<BootstrapResponse, BootstrapProviderError> {
        Ok(self.response.clone())
    }
}

fn canonicalize_transport_classes(
    transport_classes: &[String],
) -> Result<Vec<String>, BootstrapValidationError> {
    canonicalize_string_set(transport_classes, canonicalize_transport_class)
}

fn canonicalize_capabilities(
    capabilities: &[String],
) -> Result<Vec<String>, BootstrapValidationError> {
    canonicalize_string_set(capabilities, canonicalize_capability)
}

fn canonicalize_string_set(
    values: &[String],
    validate: fn(&str) -> Result<&'static str, BootstrapValidationError>,
) -> Result<Vec<String>, BootstrapValidationError> {
    let mut normalized = BTreeSet::new();
    for value in values {
        normalized.insert(validate(value)?.to_string());
    }

    Ok(normalized.into_iter().collect())
}

fn canonicalize_transport_class(value: &str) -> Result<&'static str, BootstrapValidationError> {
    match value {
        "tcp" => Ok("tcp"),
        "quic" => Ok("quic"),
        "ws" => Ok("ws"),
        "relay" => Ok("relay"),
        _ => Err(BootstrapValidationError::UnknownTransportClass {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_capability(value: &str) -> Result<&'static str, BootstrapValidationError> {
    match value {
        "relay-forward" => Ok("relay-forward"),
        "relay-intro" => Ok("relay-intro"),
        "rendezvous-helper" => Ok("rendezvous-helper"),
        "bridge" => Ok("bridge"),
        "service-host" => Ok("service-host"),
        _ => Err(BootstrapValidationError::UnknownCapability {
            value: value.to_string(),
        }),
    }
}

fn canonicalize_dial_hints(dial_hints: &mut Vec<String>) -> Result<(), BootstrapValidationError> {
    for dial_hint in dial_hints.iter_mut() {
        canonicalize_single_dial_hint(dial_hint, "peers[].dial_hints[]")?;
    }
    dial_hints.retain(|dial_hint| !dial_hint.is_empty());
    dial_hints.sort();
    dial_hints.dedup();
    Ok(())
}

fn canonicalize_single_dial_hint(
    dial_hint: &mut String,
    field: &'static str,
) -> Result<(), BootstrapValidationError> {
    *dial_hint = dial_hint.trim().to_string();
    if dial_hint.is_empty() {
        return Err(BootstrapValidationError::EmptyDialHint { field });
    }
    Ok(())
}

fn bootstrap_provider_error_log_result(error: &BootstrapProviderError) -> &'static str {
    match error {
        BootstrapProviderError::Unavailable(_) => "unavailable",
        BootstrapProviderError::Integrity(_) => "integrity_mismatch",
        BootstrapProviderError::Trust(_) => "trust_verification_failed",
        BootstrapProviderError::Validation(validation_error) => {
            if bootstrap_validation_error_is_stale(validation_error) {
                "stale"
            } else {
                "rejected"
            }
        }
    }
}

fn bootstrap_validation_error_is_stale(error: &BootstrapValidationError) -> bool {
    matches!(
        error,
        BootstrapValidationError::GeneratedAfterExpiry { .. }
            | BootstrapValidationError::Expired { .. }
            | BootstrapValidationError::ExpiredBridgeHint { .. }
    )
}

fn signed_bootstrap_artifact_signature_input(
    version: u8,
    signer_public_key: Ed25519PublicKey,
    bootstrap_response: &BootstrapResponse,
) -> Result<Vec<u8>, serde_json::Error> {
    #[derive(Serialize)]
    struct UnsignedSignedBootstrapArtifact<'a> {
        version: u8,
        signer_public_key: Ed25519PublicKey,
        bootstrap_response: &'a BootstrapResponse,
    }

    let unsigned = UnsignedSignedBootstrapArtifact {
        version,
        signer_public_key,
        bootstrap_response,
    };
    let unsigned_bytes = serde_json::to_vec(&unsigned)?;
    let mut signature_input =
        Vec::with_capacity(SIGNED_BOOTSTRAP_ARTIFACT_CONTEXT.len() + unsigned_bytes.len());
    signature_input.extend_from_slice(SIGNED_BOOTSTRAP_ARTIFACT_CONTEXT);
    signature_input.extend_from_slice(&unsigned_bytes);
    Ok(signature_input)
}

#[cfg(test)]
mod tests {
    use super::{
        BootstrapNetworkParams, BootstrapPeer, BootstrapPeerRole, BootstrapProvider,
        BootstrapProviderError, BootstrapResponse, BootstrapValidationError, BridgeHint,
        SignedBootstrapArtifact, SignedBootstrapArtifactError, StaticBootstrapProvider,
        BOOTSTRAP_SCHEMA_VERSION, SIGNED_BOOTSTRAP_ARTIFACT_VERSION,
    };
    use crate::{
        crypto::sign::{Ed25519PublicKey, Ed25519SigningKey},
        identity::NodeId,
        metrics::{LogComponent, LogContext, Observability},
        session::HANDSHAKE_VERSION,
        wire::MAX_FRAME_BODY_LEN,
    };

    #[test]
    fn validates_and_canonicalizes_bootstrap_response() {
        let response = sample_response()
            .validated(1_700_000_100)
            .expect("bootstrap response should validate");

        assert_eq!(response.peers[0].transport_classes, vec!["quic", "tcp"]);
        assert_eq!(
            response.peers[0].capabilities,
            vec!["relay-forward", "service-host"]
        );
        assert_eq!(
            response.peers[0].dial_hints,
            vec!["quic://node-a", "tcp://node-a"]
        );
        assert_eq!(response.bridge_hints[0].transport_class, "ws");
        assert_eq!(response.bridge_hints[0].capabilities, vec!["bridge"]);
    }

    #[test]
    fn rejects_expired_bootstrap_response() {
        let error = sample_response()
            .validated(1_700_000_901)
            .expect_err("expired bootstrap responses must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::Expired {
                expires_at_unix_s: 1_700_000_900,
                now_unix_s: 1_700_000_901,
            }
        );
    }

    #[test]
    fn rejects_bootstrap_response_expiring_at_current_time() {
        let mut response = sample_response();
        response.expires_at_unix_s = 1_700_000_100;

        let error = response
            .validated(1_700_000_100)
            .expect_err("bootstrap responses expiring at now must be rejected as stale");

        assert_eq!(
            error,
            BootstrapValidationError::Expired {
                expires_at_unix_s: 1_700_000_100,
                now_unix_s: 1_700_000_100,
            }
        );
    }

    #[test]
    fn rejects_unsupported_bootstrap_schema_version() {
        let mut response = sample_response();
        response.version = BOOTSTRAP_SCHEMA_VERSION + 1;

        let error = response
            .validated(1_700_000_100)
            .expect_err("unsupported bootstrap schema versions must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::UnsupportedSchemaVersion {
                expected: BOOTSTRAP_SCHEMA_VERSION,
                actual: BOOTSTRAP_SCHEMA_VERSION + 1,
            }
        );
    }

    #[test]
    fn rejects_bootstrap_generated_after_expiry() {
        let mut response = sample_response();
        response.generated_at_unix_s = 1_700_000_901;

        let error = response
            .validated(1_700_000_100)
            .expect_err("generated_at after expiry must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::GeneratedAfterExpiry {
                generated_at_unix_s: 1_700_000_901,
                expires_at_unix_s: 1_700_000_900,
            }
        );
    }

    #[test]
    fn rejects_empty_bootstrap_network_id() {
        let mut response = sample_response();
        response.network_params.network_id = "   ".to_string();

        let error = response
            .validated(1_700_000_100)
            .expect_err("blank network_id must be rejected");

        assert_eq!(error, BootstrapValidationError::EmptyNetworkId);
    }

    #[test]
    fn rejects_zero_bootstrap_epoch_duration_and_presence_ttl() {
        for field in ["epoch_duration_s", "presence_ttl_s"] {
            let mut response = sample_response();
            match field {
                "epoch_duration_s" => response.epoch_duration_s = 0,
                "presence_ttl_s" => response.presence_ttl_s = 0,
                _ => unreachable!(),
            }

            let error = response
                .validated(1_700_000_100)
                .expect_err("zero bootstrap timing fields must be rejected");

            assert_eq!(error, BootstrapValidationError::ZeroField { field });
        }
    }

    #[test]
    fn rejects_bootstrap_frame_body_len_above_mvp_limit() {
        let mut response = sample_response();
        response.max_frame_body_len = MAX_FRAME_BODY_LEN + 1;

        let error = response
            .validated(1_700_000_100)
            .expect_err("bootstrap frame body len above MVP limit must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::FrameBodyTooLarge {
                max_frame_body_len: MAX_FRAME_BODY_LEN + 1,
                allowed_max_frame_body_len: MAX_FRAME_BODY_LEN,
            }
        );
    }

    #[test]
    fn rejects_unknown_bootstrap_peer_transport_class() {
        let mut response = sample_response();
        response.peers[0].transport_classes.push("smtp".to_string());

        let error = response
            .validated(1_700_000_100)
            .expect_err("unknown peer transport classes must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::UnknownTransportClass {
                value: "smtp".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unknown_bootstrap_capability() {
        let mut response = sample_response();
        response.peers[0].capabilities.push("smtp".to_string());

        let error = response
            .validated(1_700_000_100)
            .expect_err("unknown bootstrap capabilities must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::UnknownCapability {
                value: "smtp".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unknown_bootstrap_bridge_hint_transport_class() {
        let mut response = sample_response();
        response.bridge_hints[0].transport_class = "smtp".to_string();

        let error = response
            .validated(1_700_000_100)
            .expect_err("unknown bridge-hint transport classes must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::UnknownTransportClass {
                value: "smtp".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unknown_bootstrap_bridge_hint_capability() {
        let mut response = sample_response();
        response.bridge_hints[0]
            .capabilities
            .push("smtp".to_string());

        let error = response
            .validated(1_700_000_100)
            .expect_err("unknown bridge-hint capabilities must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::UnknownCapability {
                value: "smtp".to_string(),
            }
        );
    }

    #[test]
    fn rejects_duplicate_bootstrap_peer_node_ids() {
        let mut response = sample_response();
        response.peers.push(BootstrapPeer {
            node_id: response.peers[0].node_id,
            transport_classes: vec!["tcp".to_string()],
            capabilities: vec![],
            dial_hints: vec!["tcp://node-a-shadow".to_string()],
            observed_role: BootstrapPeerRole::Standard,
        });

        let error = response
            .validated(1_700_000_100)
            .expect_err("duplicate bootstrap peers must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::DuplicatePeerNodeId {
                node_id: NodeId::from_bytes([1_u8; 32]),
            }
        );
    }

    #[test]
    fn rejects_zero_bootstrap_max_frame_body_len() {
        let mut response = sample_response();
        response.max_frame_body_len = 0;

        let error = response
            .validated(1_700_000_100)
            .expect_err("zero max_frame_body_len must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::ZeroField {
                field: "max_frame_body_len",
            }
        );
    }

    #[test]
    fn rejects_bootstrap_peer_without_transport_classes() {
        let mut response = sample_response();
        response.peers[0].transport_classes.clear();

        let error = response
            .validated(1_700_000_100)
            .expect_err("bootstrap peers without transport classes must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::PeerWithoutTransportClasses {
                node_id: NodeId::from_bytes([1_u8; 32]),
            }
        );
    }

    #[test]
    fn rejects_bootstrap_peer_without_dial_hints() {
        let mut response = sample_response();
        response.peers[0].dial_hints.clear();

        let error = response
            .validated(1_700_000_100)
            .expect_err("bootstrap peers without dial hints must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::PeerWithoutDialHints {
                node_id: NodeId::from_bytes([1_u8; 32]),
            }
        );
    }

    #[test]
    fn rejects_blank_peer_dial_hint_after_trimming() {
        let mut response = sample_response();
        response.peers[0].dial_hints = vec!["   ".to_string()];

        let error = response
            .validated(1_700_000_100)
            .expect_err("blank dial hints must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::EmptyDialHint {
                field: "peers[].dial_hints[]",
            }
        );
    }

    #[test]
    fn rejects_blank_bridge_hint_dial_hint_after_trimming() {
        let mut response = sample_response();
        response.bridge_hints[0].dial_hint = "   ".to_string();

        let error = response
            .validated(1_700_000_100)
            .expect_err("blank bridge dial hints must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::EmptyDialHint {
                field: "bridge_hints[].dial_hint",
            }
        );
    }

    #[test]
    fn rejects_expired_bridge_hint() {
        let mut response = sample_response();
        response.bridge_hints[0].expires_at_unix_s = 1_700_000_100;

        let error = response
            .validated(1_700_000_100)
            .expect_err("expired bridge hints must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::ExpiredBridgeHint {
                expires_at_unix_s: 1_700_000_100,
                now_unix_s: 1_700_000_100,
            }
        );
    }

    #[test]
    fn rejects_duplicate_bridge_hints_after_canonicalization() {
        let mut response = sample_response();
        response.bridge_hints.push(BridgeHint {
            transport_class: "ws".to_string(),
            dial_hint: "https://bridge-a".to_string(),
            capabilities: vec!["bridge".to_string()],
            expires_at_unix_s: 1_700_000_850,
        });

        let error = response
            .validated(1_700_000_100)
            .expect_err("duplicate bridge hints must be rejected");

        assert_eq!(
            error,
            BootstrapValidationError::DuplicateBridgeHint {
                transport_class: "ws".to_string(),
                dial_hint: "https://bridge-a".to_string(),
            }
        );
    }

    #[test]
    fn static_provider_returns_validated_response() {
        let provider = StaticBootstrapProvider::new(sample_response());
        let response = provider
            .fetch_validated_response(1_700_000_100)
            .expect("static provider should validate its response");

        assert_eq!(provider.provider_name(), "static");
        assert_eq!(response.version, BOOTSTRAP_SCHEMA_VERSION);
    }

    #[test]
    fn static_provider_surfaces_validation_errors() {
        let mut response = sample_response();
        response.handshake_version = 99;
        let provider = StaticBootstrapProvider::new(response);

        let error = provider
            .fetch_validated_response(1_700_000_100)
            .expect_err("invalid static responses must fail validation");

        assert!(matches!(
            error,
            BootstrapProviderError::Validation(
                BootstrapValidationError::UnsupportedHandshakeVersion { .. }
            )
        ));
    }

    #[test]
    fn signed_bootstrap_artifact_verifies_with_trusted_signer() {
        let signing_key = Ed25519SigningKey::from_seed([17_u8; 32]);
        let artifact = SignedBootstrapArtifact::sign(sample_response(), &signing_key)
            .expect("artifact should sign");

        let response = artifact
            .verify_with_trusted_signer(&signing_key.public_key())
            .expect("artifact should verify");

        assert_eq!(artifact.version, SIGNED_BOOTSTRAP_ARTIFACT_VERSION);
        assert_eq!(artifact.signer_public_key, signing_key.public_key());
        assert_eq!(response, sample_response());
    }

    #[test]
    fn signed_bootstrap_artifact_rejects_trusted_signer_mismatch() {
        let signing_key = Ed25519SigningKey::from_seed([18_u8; 32]);
        let artifact = SignedBootstrapArtifact::sign(sample_response(), &signing_key)
            .expect("artifact should sign");
        let wrong_signer = Ed25519PublicKey::from_bytes([19_u8; 32]);

        let error = artifact
            .verify_with_trusted_signer(&wrong_signer)
            .expect_err("wrong signer should be rejected");

        assert!(matches!(
            error,
            SignedBootstrapArtifactError::TrustedSignerMismatch { .. }
        ));
    }

    #[test]
    fn signed_bootstrap_artifact_rejects_tampered_signature() {
        let signing_key = Ed25519SigningKey::from_seed([20_u8; 32]);
        let mut artifact = SignedBootstrapArtifact::sign(sample_response(), &signing_key)
            .expect("artifact should sign");
        artifact.signature = crate::crypto::sign::Ed25519Signature::from_bytes([0_u8; 64]);

        let error = artifact
            .verify_with_trusted_signer(&signing_key.public_key())
            .expect_err("tampered signature should be rejected");

        assert!(matches!(error, SignedBootstrapArtifactError::Crypto(_)));
    }

    #[test]
    fn bootstrap_provider_observability_logs_accepted_fetch() {
        let provider = StaticBootstrapProvider::new(sample_response());
        let node_id = NodeId::from_bytes([9_u8; 32]);
        let mut observability = Observability::default();

        let response = provider
            .fetch_validated_response_with_observability(
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 51,
                },
            )
            .expect("bootstrap response should validate");

        assert_eq!(response.version, BOOTSTRAP_SCHEMA_VERSION);
        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.component, LogComponent::Bootstrap);
        assert_eq!(log.event, "bootstrap_fetch");
        assert_eq!(log.result, "accepted");
    }

    #[test]
    fn bootstrap_provider_observability_logs_rejected_validation_error() {
        let mut response = sample_response();
        response.handshake_version = 99;
        let provider = StaticBootstrapProvider::new(response);
        let node_id = NodeId::from_bytes([10_u8; 32]);
        let mut observability = Observability::default();

        let error = provider
            .fetch_validated_response_with_observability(
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 52,
                },
            )
            .expect_err("invalid bootstrap response must be rejected");

        assert!(matches!(
            error,
            BootstrapProviderError::Validation(
                BootstrapValidationError::UnsupportedHandshakeVersion { .. }
            )
        ));
        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.component, LogComponent::Bootstrap);
        assert_eq!(log.event, "bootstrap_fetch");
        assert_eq!(log.result, "rejected");
    }

    #[test]
    fn bootstrap_provider_observability_logs_stale_validation_error() {
        let mut response = sample_response();
        response.expires_at_unix_s = 1_700_000_000;
        let provider = StaticBootstrapProvider::new(response);
        let node_id = NodeId::from_bytes([12_u8; 32]);
        let mut observability = Observability::default();

        let error = provider
            .fetch_validated_response_with_observability(
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 54,
                },
            )
            .expect_err("stale bootstrap response must be rejected");

        assert!(matches!(
            error,
            BootstrapProviderError::Validation(BootstrapValidationError::Expired { .. })
        ));
        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.result, "stale");
    }

    #[test]
    fn bootstrap_provider_observability_logs_integrity_failure() {
        struct IntegrityBootstrapProvider;

        impl BootstrapProvider for IntegrityBootstrapProvider {
            fn provider_name(&self) -> &'static str {
                "integrity"
            }

            fn fetch_response(&self) -> Result<BootstrapResponse, BootstrapProviderError> {
                Err(BootstrapProviderError::Integrity(
                    "pin mismatch".to_string(),
                ))
            }
        }

        let provider = IntegrityBootstrapProvider;
        let node_id = NodeId::from_bytes([13_u8; 32]);
        let mut observability = Observability::default();

        provider
            .fetch_validated_response_with_observability(
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 55,
                },
            )
            .expect_err("integrity failure should surface");

        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.result, "integrity_mismatch");
    }

    #[test]
    fn bootstrap_provider_observability_logs_trust_failure() {
        struct TrustBootstrapProvider;

        impl BootstrapProvider for TrustBootstrapProvider {
            fn provider_name(&self) -> &'static str {
                "trust"
            }

            fn fetch_response(&self) -> Result<BootstrapResponse, BootstrapProviderError> {
                Err(BootstrapProviderError::Trust("signer mismatch".to_string()))
            }
        }

        let provider = TrustBootstrapProvider;
        let node_id = NodeId::from_bytes([14_u8; 32]);
        let mut observability = Observability::default();

        provider
            .fetch_validated_response_with_observability(
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 56,
                },
            )
            .expect_err("trust failure should surface");

        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.result, "trust_verification_failed");
    }

    #[test]
    fn bootstrap_provider_observability_logs_unavailable_fetch() {
        struct UnavailableBootstrapProvider;

        impl BootstrapProvider for UnavailableBootstrapProvider {
            fn provider_name(&self) -> &'static str {
                "unavailable"
            }

            fn fetch_response(&self) -> Result<BootstrapResponse, BootstrapProviderError> {
                Err(BootstrapProviderError::Unavailable(
                    "provider offline".to_string(),
                ))
            }
        }

        let provider = UnavailableBootstrapProvider;
        let node_id = NodeId::from_bytes([11_u8; 32]);
        let mut observability = Observability::default();

        let error = provider
            .fetch_validated_response_with_observability(
                1_700_000_100,
                &mut observability,
                LogContext {
                    timestamp_unix_ms: 1_700_000_100_000,
                    node_id,
                    correlation_id: 53,
                },
            )
            .expect_err("unavailable provider must surface an error");

        assert!(matches!(error, BootstrapProviderError::Unavailable(_)));
        let log = observability.latest_log().expect("log should be present");
        assert_eq!(log.component, LogComponent::Bootstrap);
        assert_eq!(log.event, "bootstrap_fetch");
        assert_eq!(log.result, "unavailable");
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
                BootstrapPeer {
                    node_id: NodeId::from_bytes([1_u8; 32]),
                    transport_classes: vec![
                        "tcp".to_string(),
                        "quic".to_string(),
                        "tcp".to_string(),
                    ],
                    capabilities: vec![
                        "service-host".to_string(),
                        "relay-forward".to_string(),
                        "service-host".to_string(),
                    ],
                    dial_hints: vec![
                        " tcp://node-a ".to_string(),
                        "quic://node-a".to_string(),
                        "tcp://node-a".to_string(),
                    ],
                    observed_role: BootstrapPeerRole::Relay,
                },
                BootstrapPeer {
                    node_id: NodeId::from_bytes([2_u8; 32]),
                    transport_classes: vec!["ws".to_string()],
                    capabilities: vec!["bridge".to_string()],
                    dial_hints: vec!["https://node-b".to_string()],
                    observed_role: BootstrapPeerRole::Standard,
                },
            ],
            bridge_hints: vec![BridgeHint {
                transport_class: "ws".to_string(),
                dial_hint: " https://bridge-a ".to_string(),
                capabilities: vec!["bridge".to_string(), "bridge".to_string()],
                expires_at_unix_s: 1_700_000_800,
            }],
        }
    }
}
