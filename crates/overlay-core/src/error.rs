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
