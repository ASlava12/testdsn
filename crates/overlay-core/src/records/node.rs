use serde::{Deserialize, Serialize};

use crate::ids::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub version: u8,
    pub node_id: NodeId,
    pub node_public_key: Vec<u8>,
    pub created_at_unix_s: u64,
    pub flags: u64,
    pub supported_transports: Vec<String>,
    pub supported_kex: Vec<String>,
    pub supported_signatures: Vec<String>,
    pub anti_sybil_proof: Option<Vec<u8>>,
    pub signature: Vec<u8>,
}
