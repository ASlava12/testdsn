use async_trait::async_trait;

use crate::error::OverlayError;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn dial(&self, addr: &str) -> Result<(), OverlayError>;
    async fn send(&self, data: &[u8]) -> Result<(), OverlayError>;
    async fn recv(&self) -> Result<Vec<u8>, OverlayError>;
    async fn close(&self) -> Result<(), OverlayError>;
}
