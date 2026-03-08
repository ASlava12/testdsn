use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameHeader {
    pub version: u8,
    pub msg_type: u16,
    pub flags: u16,
    pub body_len: u32,
    pub correlation_id: u64,
}
