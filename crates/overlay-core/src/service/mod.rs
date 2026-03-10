//! Exact `app_id` service resolution and application session setup for Milestone 8.

use std::collections::BTreeMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::FrameError,
    identity::{AppId, NodeId},
    records::{ServiceRecord, VerifiedServiceRecord},
    wire::{Message, MessageType, MAX_FRAME_BODY_LEN},
};

pub const DEFAULT_MAX_REGISTERED_SERVICES: usize = 256;
pub const DEFAULT_MAX_OPEN_SERVICE_SESSIONS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub max_registered_services: usize,
    pub max_open_service_sessions: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            max_registered_services: DEFAULT_MAX_REGISTERED_SERVICES,
            max_open_service_sessions: DEFAULT_MAX_OPEN_SERVICE_SESSIONS,
        }
    }
}

impl ServiceConfig {
    pub fn validate(self) -> Result<Self, ServiceError> {
        for (field, value) in [
            (
                "max_registered_services",
                self.max_registered_services as u64,
            ),
            (
                "max_open_service_sessions",
                self.max_open_service_sessions as u64,
            ),
        ] {
            if value == 0 {
                return Err(ServiceError::ZeroLimit { field });
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("service config limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
    #[error("service registry would exceed max_registered_services ({max_registered_services})")]
    RegisteredServiceLimitExceeded { max_registered_services: usize },
}

#[derive(Debug, Error)]
pub enum ServiceMessageError {
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("service response status {status} requires {expectation}")]
    InvalidResponseShape {
        status: &'static str,
        expectation: &'static str,
    },
    #[error("open-app-session result status {status} requires {expectation}")]
    InvalidOpenResultShape {
        status: &'static str,
        expectation: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetServiceRecord {
    pub app_id: AppId,
}

impl GetServiceRecord {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ServiceMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ServiceMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for GetServiceRecord {
    const TYPE: MessageType = MessageType::GetServiceRecord;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRecordResponseStatus {
    Found,
    NotFound,
}

impl ServiceRecordResponseStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::NotFound => "not_found",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRecordResponse {
    pub app_id: AppId,
    pub status: ServiceRecordResponseStatus,
    pub record: Option<ServiceRecord>,
}

impl ServiceRecordResponse {
    pub fn found(record: ServiceRecord) -> Self {
        Self {
            app_id: record.app_id,
            status: ServiceRecordResponseStatus::Found,
            record: Some(record),
        }
    }

    pub fn not_found(app_id: AppId) -> Self {
        Self {
            app_id,
            status: ServiceRecordResponseStatus::NotFound,
            record: None,
        }
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ServiceMessageError> {
        self.validate()?;
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ServiceMessageError> {
        let message = parse_message_bytes::<Self>(bytes)?;
        message.validate()?;
        Ok(message)
    }

    fn validate(&self) -> Result<(), ServiceMessageError> {
        match (&self.status, &self.record) {
            (ServiceRecordResponseStatus::Found, Some(record)) if record.app_id == self.app_id => {
                Ok(())
            }
            (ServiceRecordResponseStatus::Found, Some(_)) => {
                Err(ServiceMessageError::InvalidResponseShape {
                    status: self.status.as_str(),
                    expectation: "a record whose app_id matches the response app_id",
                })
            }
            (ServiceRecordResponseStatus::Found, None) => {
                Err(ServiceMessageError::InvalidResponseShape {
                    status: self.status.as_str(),
                    expectation: "a matching record payload",
                })
            }
            (ServiceRecordResponseStatus::NotFound, None) => Ok(()),
            (ServiceRecordResponseStatus::NotFound, Some(_)) => {
                Err(ServiceMessageError::InvalidResponseShape {
                    status: self.status.as_str(),
                    expectation: "no record payload",
                })
            }
        }
    }
}

impl Message for ServiceRecordResponse {
    const TYPE: MessageType = MessageType::ServiceRecordResponse;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAppSession {
    pub app_id: AppId,
    pub reachability_ref: Vec<u8>,
}

impl OpenAppSession {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ServiceMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ServiceMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for OpenAppSession {
    const TYPE: MessageType = MessageType::OpenAppSession;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAppSessionStatus {
    Opened,
    RejectedNotFound,
    RejectedPolicy,
    RejectedReachabilityMismatch,
    RejectedSessionLimit,
}

impl OpenAppSessionStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::RejectedNotFound => "rejected_not_found",
            Self::RejectedPolicy => "rejected_policy",
            Self::RejectedReachabilityMismatch => "rejected_reachability_mismatch",
            Self::RejectedSessionLimit => "rejected_session_limit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAppSessionResult {
    pub app_id: AppId,
    pub status: OpenAppSessionStatus,
    pub session_id: Option<u64>,
}

impl OpenAppSessionResult {
    pub fn opened(app_id: AppId, session_id: u64) -> Self {
        Self {
            app_id,
            status: OpenAppSessionStatus::Opened,
            session_id: Some(session_id),
        }
    }

    pub fn rejected(app_id: AppId, status: OpenAppSessionStatus) -> Self {
        debug_assert!(status != OpenAppSessionStatus::Opened);
        Self {
            app_id,
            status,
            session_id: None,
        }
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ServiceMessageError> {
        self.validate()?;
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ServiceMessageError> {
        let message = parse_message_bytes::<Self>(bytes)?;
        message.validate()?;
        Ok(message)
    }

    fn validate(&self) -> Result<(), ServiceMessageError> {
        match (&self.status, &self.session_id) {
            (OpenAppSessionStatus::Opened, Some(_)) => Ok(()),
            (OpenAppSessionStatus::Opened, None) => {
                Err(ServiceMessageError::InvalidOpenResultShape {
                    status: self.status.as_str(),
                    expectation: "a session_id",
                })
            }
            (_, None) => Ok(()),
            (_, Some(_)) => Err(ServiceMessageError::InvalidOpenResultShape {
                status: self.status.as_str(),
                expectation: "no session_id",
            }),
        }
    }
}

impl Message for OpenAppSessionResult {
    const TYPE: MessageType = MessageType::OpenAppSessionResult;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalServicePolicy {
    pub allow_open: bool,
}

impl LocalServicePolicy {
    pub const fn allow_all() -> Self {
        Self { allow_open: true }
    }

    pub const fn deny_all() -> Self {
        Self { allow_open: false }
    }
}

impl Default for LocalServicePolicy {
    fn default() -> Self {
        Self::allow_all()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenServiceSession {
    pub session_id: u64,
    pub app_id: AppId,
    pub node_id: NodeId,
    pub opened_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
struct RegisteredService {
    record: ServiceRecord,
    policy: LocalServicePolicy,
}

#[derive(Debug, Clone)]
pub struct ServiceRegistry {
    config: ServiceConfig,
    services: BTreeMap<AppId, RegisteredService>,
    open_sessions: BTreeMap<u64, OpenServiceSession>,
    next_session_id: u64,
}

impl ServiceRegistry {
    pub fn new(config: ServiceConfig) -> Result<Self, ServiceError> {
        let config = config.validate()?;
        Ok(Self {
            config,
            services: BTreeMap::new(),
            open_sessions: BTreeMap::new(),
            next_session_id: 1,
        })
    }

    pub fn register_verified(
        &mut self,
        record: VerifiedServiceRecord,
        policy: LocalServicePolicy,
    ) -> Result<(), ServiceError> {
        let record = record.into_inner();
        let app_id = record.app_id;
        if !self.services.contains_key(&app_id)
            && self.services.len() == self.config.max_registered_services
        {
            return Err(ServiceError::RegisteredServiceLimitExceeded {
                max_registered_services: self.config.max_registered_services,
            });
        }

        self.services
            .insert(app_id, RegisteredService { record, policy });
        Ok(())
    }

    pub fn registered_service_count(&self) -> usize {
        self.services.len()
    }

    pub fn open_session_count(&self) -> usize {
        self.open_sessions.len()
    }

    pub fn session(&self, session_id: u64) -> Option<&OpenServiceSession> {
        self.open_sessions.get(&session_id)
    }

    pub fn close_session(&mut self, session_id: u64) -> Option<OpenServiceSession> {
        self.open_sessions.remove(&session_id)
    }

    pub fn resolve(&self, request: GetServiceRecord) -> ServiceRecordResponse {
        match self.services.get(&request.app_id) {
            Some(registered) => ServiceRecordResponse::found(registered.record.clone()),
            None => ServiceRecordResponse::not_found(request.app_id),
        }
    }

    pub fn open_app_session(
        &mut self,
        request: OpenAppSession,
        now_unix_ms: u64,
    ) -> OpenAppSessionResult {
        let Some(registered) = self.services.get(&request.app_id) else {
            return OpenAppSessionResult::rejected(
                request.app_id,
                OpenAppSessionStatus::RejectedNotFound,
            );
        };

        if registered.record.reachability_ref != request.reachability_ref {
            return OpenAppSessionResult::rejected(
                request.app_id,
                OpenAppSessionStatus::RejectedReachabilityMismatch,
            );
        }

        if !registered.policy.allow_open {
            return OpenAppSessionResult::rejected(
                request.app_id,
                OpenAppSessionStatus::RejectedPolicy,
            );
        }

        if self.open_sessions.len() == self.config.max_open_service_sessions {
            return OpenAppSessionResult::rejected(
                request.app_id,
                OpenAppSessionStatus::RejectedSessionLimit,
            );
        }

        let session_id = self.next_session_id;
        self.next_session_id = self.next_session_id.saturating_add(1);
        let session = OpenServiceSession {
            session_id,
            app_id: request.app_id,
            node_id: registered.record.node_id,
            opened_at_unix_ms: now_unix_ms,
        };
        self.open_sessions.insert(session_id, session);
        OpenAppSessionResult::opened(request.app_id, session_id)
    }
}

fn canonical_message_bytes<T>(message: &T) -> Result<Vec<u8>, ServiceMessageError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(message)?;
    let body_len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    if body_len > MAX_FRAME_BODY_LEN {
        return Err(ServiceMessageError::Frame(FrameError::BodyTooLarge {
            body_len,
            max_body_len: MAX_FRAME_BODY_LEN,
        }));
    }

    Ok(bytes)
}

fn parse_message_bytes<T>(bytes: &[u8]) -> Result<T, ServiceMessageError>
where
    T: DeserializeOwned,
{
    let body_len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    if body_len > MAX_FRAME_BODY_LEN {
        return Err(ServiceMessageError::Frame(FrameError::BodyTooLarge {
            body_len,
            max_body_len: MAX_FRAME_BODY_LEN,
        }));
    }

    Ok(serde_json::from_slice(bytes)?)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde::Deserialize;

    use super::{
        GetServiceRecord, LocalServicePolicy, OpenAppSession, OpenAppSessionResult,
        OpenAppSessionStatus, ServiceConfig, ServiceRecordResponse, ServiceRecordResponseStatus,
        ServiceRegistry,
    };
    use crate::{
        crypto::sign::Ed25519SigningKey,
        identity::{derive_app_id, derive_node_id, AppId, NodeId},
        records::ServiceRecord,
    };

    #[test]
    fn service_config_rejects_zero_limits() {
        let error = ServiceRegistry::new(ServiceConfig {
            max_registered_services: 0,
            max_open_service_sessions: 1,
        })
        .expect_err("zero registry limit should be rejected");

        assert!(matches!(
            error,
            super::ServiceError::ZeroLimit {
                field: "max_registered_services"
            }
        ));
    }

    #[test]
    fn service_message_vectors_match_fixture() {
        let vector = read_service_message_vector();
        let app_id = AppId::from_slice(&decode_hex(&vector.record.app_id_hex))
            .expect("service message vector app_id must be 32 bytes");
        let node_id = NodeId::from_slice(&decode_hex(&vector.record.node_id_hex))
            .expect("service message vector node_id must be 32 bytes");
        let record = ServiceRecord {
            version: vector.record.version,
            node_id,
            app_id,
            service_name: vector.record.service_name,
            service_version: vector.record.service_version,
            auth_mode: vector.record.auth_mode,
            policy: decode_hex(&vector.record.policy_hex),
            reachability_ref: decode_hex(&vector.record.reachability_ref_hex),
            metadata_commitment: decode_hex(&vector.record.metadata_commitment_hex),
            signature: decode_hex(&vector.record.signature_hex),
        };

        let lookup = GetServiceRecord { app_id };
        let found = ServiceRecordResponse::found(record.clone());
        let not_found = ServiceRecordResponse::not_found(app_id);
        let open = OpenAppSession {
            app_id,
            reachability_ref: record.reachability_ref.clone(),
        };
        let opened = OpenAppSessionResult::opened(app_id, vector.opened_session_id);
        let rejected_policy =
            OpenAppSessionResult::rejected(app_id, OpenAppSessionStatus::RejectedPolicy);

        assert_eq!(
            encode_hex(
                &lookup
                    .canonical_bytes()
                    .expect("lookup vector should serialize"),
            ),
            vector.get_service_record_hex
        );
        assert_eq!(
            encode_hex(
                &found
                    .canonical_bytes()
                    .expect("found response vector should serialize"),
            ),
            vector.service_record_response_found_hex
        );
        assert_eq!(
            encode_hex(
                &not_found
                    .canonical_bytes()
                    .expect("not-found response vector should serialize"),
            ),
            vector.service_record_response_not_found_hex
        );
        assert_eq!(
            encode_hex(
                &open
                    .canonical_bytes()
                    .expect("open vector should serialize")
            ),
            vector.open_app_session_hex
        );
        assert_eq!(
            encode_hex(
                &opened
                    .canonical_bytes()
                    .expect("opened result vector should serialize"),
            ),
            vector.open_app_session_result_opened_hex
        );
        assert_eq!(
            encode_hex(
                &rejected_policy
                    .canonical_bytes()
                    .expect("rejected result vector should serialize"),
            ),
            vector.open_app_session_result_rejected_policy_hex
        );

        assert_eq!(
            ServiceRecordResponse::from_canonical_bytes(&decode_hex(
                &vector.service_record_response_found_hex
            ))
            .expect("found response should parse")
            .status,
            ServiceRecordResponseStatus::Found
        );
        assert_eq!(
            OpenAppSessionResult::from_canonical_bytes(&decode_hex(
                &vector.open_app_session_result_rejected_policy_hex
            ))
            .expect("rejected result should parse")
            .status,
            OpenAppSessionStatus::RejectedPolicy
        );
    }

    #[test]
    fn service_registry_resolves_exact_app_id_after_verified_registration() {
        let signing_key = Ed25519SigningKey::from_seed([41_u8; 32]);
        let record = sample_signed_service_record(&signing_key, "terminal");
        let app_id = record.app_id;
        let mut registry =
            ServiceRegistry::new(ServiceConfig::default()).expect("default config should work");

        registry
            .register_verified(
                record
                    .clone()
                    .verify_with_public_key(&signing_key.public_key())
                    .expect("signed service record should verify"),
                LocalServicePolicy::allow_all(),
            )
            .expect("verified service should register");

        let response = registry.resolve(GetServiceRecord { app_id });
        assert_eq!(registry.registered_service_count(), 1);
        assert_eq!(response.status, ServiceRecordResponseStatus::Found);
        assert_eq!(response.record, Some(record));
    }

    #[test]
    fn service_registry_rejects_policy_denied_open_requests() {
        let signing_key = Ed25519SigningKey::from_seed([42_u8; 32]);
        let record = sample_signed_service_record(&signing_key, "terminal");
        let mut registry =
            ServiceRegistry::new(ServiceConfig::default()).expect("default config should work");

        registry
            .register_verified(
                record
                    .clone()
                    .verify_with_public_key(&signing_key.public_key())
                    .expect("signed service record should verify"),
                LocalServicePolicy::deny_all(),
            )
            .expect("verified service should register");

        let result = registry.open_app_session(
            OpenAppSession {
                app_id: record.app_id,
                reachability_ref: record.reachability_ref.clone(),
            },
            1_700_000_000_123,
        );

        assert_eq!(result.status, OpenAppSessionStatus::RejectedPolicy);
        assert_eq!(result.session_id, None);
        assert_eq!(registry.open_session_count(), 0);
    }

    #[test]
    fn service_registry_rejects_reachability_ref_mismatch() {
        let signing_key = Ed25519SigningKey::from_seed([43_u8; 32]);
        let record = sample_signed_service_record(&signing_key, "terminal");
        let mut registry =
            ServiceRegistry::new(ServiceConfig::default()).expect("default config should work");

        registry
            .register_verified(
                record
                    .verify_with_public_key(&signing_key.public_key())
                    .expect("signed service record should verify"),
                LocalServicePolicy::allow_all(),
            )
            .expect("verified service should register");

        let result = registry.open_app_session(
            OpenAppSession {
                app_id: derive_app_id(
                    &derive_node_id(signing_key.public_key().as_bytes()),
                    "chat",
                    "terminal",
                ),
                reachability_ref: vec![0xFF, 0xEE],
            },
            1_700_000_000_123,
        );

        assert_eq!(
            result.status,
            OpenAppSessionStatus::RejectedReachabilityMismatch
        );
        assert_eq!(result.session_id, None);
    }

    #[test]
    fn service_registry_enforces_open_session_limit() {
        let signing_key = Ed25519SigningKey::from_seed([44_u8; 32]);
        let record = sample_signed_service_record(&signing_key, "terminal");
        let mut registry = ServiceRegistry::new(ServiceConfig {
            max_registered_services: 4,
            max_open_service_sessions: 1,
        })
        .expect("config should work");

        registry
            .register_verified(
                record
                    .clone()
                    .verify_with_public_key(&signing_key.public_key())
                    .expect("signed service record should verify"),
                LocalServicePolicy::allow_all(),
            )
            .expect("verified service should register");

        let opened = registry.open_app_session(
            OpenAppSession {
                app_id: record.app_id,
                reachability_ref: record.reachability_ref.clone(),
            },
            1_700_000_000_123,
        );
        let rejected = registry.open_app_session(
            OpenAppSession {
                app_id: record.app_id,
                reachability_ref: record.reachability_ref.clone(),
            },
            1_700_000_000_124,
        );

        assert_eq!(opened.status, OpenAppSessionStatus::Opened);
        assert_eq!(rejected.status, OpenAppSessionStatus::RejectedSessionLimit);
        assert_eq!(registry.open_session_count(), 1);
        assert_eq!(
            registry
                .session(
                    opened
                        .session_id
                        .expect("opened result should have session id")
                )
                .expect("opened session should be stored")
                .opened_at_unix_ms,
            1_700_000_000_123
        );
    }

    fn sample_signed_service_record(
        signing_key: &Ed25519SigningKey,
        service_name: &str,
    ) -> ServiceRecord {
        let node_id = derive_node_id(signing_key.public_key().as_bytes());
        let mut record = ServiceRecord {
            version: 1,
            node_id,
            app_id: derive_app_id(&node_id, "chat", service_name),
            service_name: service_name.to_string(),
            service_version: "1.0.0".to_string(),
            auth_mode: "none".to_string(),
            policy: vec![1, 2, 3, 4],
            reachability_ref: vec![0xAA, 0xBB],
            metadata_commitment: vec![0xCC, 0xDD],
            signature: Vec::new(),
        };
        let body = record
            .canonical_body_bytes()
            .expect("service body should serialize");
        record.signature = signing_key.sign(&body).as_bytes().to_vec();
        record
    }

    #[derive(Debug, Deserialize)]
    struct ServiceMessageVector {
        record: ServiceMessageVectorRecord,
        get_service_record_hex: String,
        service_record_response_found_hex: String,
        service_record_response_not_found_hex: String,
        open_app_session_hex: String,
        opened_session_id: u64,
        open_app_session_result_opened_hex: String,
        open_app_session_result_rejected_policy_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct ServiceMessageVectorRecord {
        version: u8,
        node_id_hex: String,
        app_id_hex: String,
        service_name: String,
        service_version: String,
        auth_mode: String,
        policy_hex: String,
        reachability_ref_hex: String,
        metadata_commitment_hex: String,
        signature_hex: String,
    }

    fn service_message_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("service_messages.json")
    }

    fn read_service_message_vector() -> ServiceMessageVector {
        let bytes = fs::read(service_message_vector_path())
            .expect("service message vector file should exist");
        serde_json::from_slice(&bytes).expect("service message vector should parse")
    }

    fn decode_hex(hex: &str) -> Vec<u8> {
        assert!(
            hex.len().is_multiple_of(2),
            "hex fixture must contain an even number of digits"
        );

        let mut bytes = Vec::with_capacity(hex.len() / 2);
        for chunk in hex.as_bytes().chunks_exact(2) {
            let high = decode_hex_nibble(chunk[0]);
            let low = decode_hex_nibble(chunk[1]);
            bytes.push((high << 4) | low);
        }
        bytes
    }

    fn decode_hex_nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!("invalid hex digit in fixture: {byte}"),
        }
    }

    fn encode_hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";

        let mut out = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }
}
