use serde::{Deserialize, Serialize};

use crate::ids::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayHint {
    pub relay_node_id: NodeId,
    pub relay_transport_class: String,
    pub relay_score: u32,
    pub relay_policy: String,
    pub expiry_unix_s: u64,
}
