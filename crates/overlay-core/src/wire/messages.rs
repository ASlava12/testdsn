use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    pub protocol_version: u8,
    pub node_public_key: Vec<u8>,
    pub ephemeral_key: [u8; 32],
    pub nonce: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHello {
    pub protocol_version: u8,
    pub node_public_key: Vec<u8>,
    pub ephemeral_key: [u8; 32],
    pub nonce: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientFinish {
    pub transcript_sig: Vec<u8>,
}
