use thiserror::Error;

#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("invalid message")]
    InvalidMessage,
    #[error("unsupported version")]
    UnsupportedVersion,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("expired record")]
    ExpiredRecord,
    #[error("not found")]
    NotFound,
    #[error("rate limited")]
    RateLimited,
    #[error("policy denied")]
    PolicyDenied,
    #[error("invalid ticket")]
    InvalidTicket,
    #[error("internal error: {0}")]
    Internal(String),
}
