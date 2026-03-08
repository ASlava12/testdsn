use crate::error::OverlayError;

pub trait Codec<T> {
    fn encode(value: &T) -> Result<Vec<u8>, OverlayError>;
    fn decode(bytes: &[u8]) -> Result<T, OverlayError>;
}
