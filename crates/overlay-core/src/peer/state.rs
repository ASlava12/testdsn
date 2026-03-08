use serde::{Deserialize, Serialize};

use crate::ids::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborState {
    pub node_id: NodeId,
    pub session_id: u64,
    pub transport: String,
    pub conn_state: String,
    pub last_seen_unix_s: u64,
    pub trust_score: i32,
    pub latency_ms: u32,
    pub capabilities: Vec<String>,
    pub routing_role: String,
}
