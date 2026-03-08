use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdentityError {
    #[error("invalid identifier length: expected {expected} bytes, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RecordValidationError {
    #[error("node_id does not match the BLAKE3-256 hash of node_public_key")]
    NodeIdMismatch,
    #[error("record expired at {expires_at_unix_s}, now is {now_unix_s}")]
    Expired {
        expires_at_unix_s: u64,
        now_unix_s: u64,
    },
    #[error("unknown transport class in signed record: {value}")]
    UnknownTransportClass { value: String },
    #[error("unknown key exchange in signed record: {value}")]
    UnknownKeyExchange { value: String },
    #[error("unknown signature algorithm in signed record: {value}")]
    UnknownSignatureAlgorithm { value: String },
    #[error("unknown reachability mode in signed record: {value}")]
    UnknownReachabilityMode { value: String },
    #[error("unknown intro policy in signed record: {value}")]
    UnknownIntroPolicy { value: String },
    #[error("unknown service auth mode in signed record: {value}")]
    UnknownAuthMode { value: String },
    #[error("unknown intro ticket scope in signed record: {value}")]
    UnknownIntroScope { value: String },
    #[error("unknown capability in signed record: {value}")]
    UnknownCapability { value: String },
}

#[derive(Debug, Error)]
pub enum RecordEncodingError {
    #[error(transparent)]
    Validation(#[from] RecordValidationError),
    #[error("canonical record serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FrameError {
    #[error("frame header must be {expected} bytes, got {actual}")]
    InvalidHeaderLength { expected: usize, actual: usize },
    #[error("frame body length {body_len} exceeds the MVP limit of {max_body_len} bytes")]
    BodyTooLarge { body_len: u32, max_body_len: u32 },
    #[error("unknown message type {0}")]
    UnknownMessageType(u16),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CryptoError {
    #[error("invalid Ed25519 public key bytes")]
    InvalidEd25519PublicKey,
    #[error("Ed25519 signature verification failed")]
    SignatureVerificationFailed,
    #[error("derived X25519 shared secret is all-zero and replay-unsafe")]
    ReplayUnsafeSharedSecret,
    #[error("HKDF-SHA256 output length {len} is invalid")]
    InvalidKdfLength { len: usize },
    #[error("ChaCha20-Poly1305 encryption failed")]
    AeadEncrypt,
    #[error("ChaCha20-Poly1305 decryption failed")]
    AeadDecrypt,
}

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("unsupported handshake version: expected {expected}, got {actual}")]
    UnsupportedVersion { expected: u8, actual: u8 },
    #[error("unsupported handshake suite")]
    UnsupportedSuite,
    #[error("{role} node_id does not match the claimed signing public key")]
    NodeIdMismatch { role: &'static str },
    #[error("{role} handshake signature is invalid")]
    InvalidSignature { role: &'static str },
    #[error("client finish confirmation failed")]
    InvalidClientFinish,
    #[error("canonical handshake serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Identity(#[from] IdentityError),
}
