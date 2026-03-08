use serde::{Deserialize, Serialize};

use crate::ids::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetric {
    pub peer_node_id: NodeId,
    pub transport_class: String,
    pub est_rtt_ms: u32,
    pub obs_rtt_ms: u32,
    pub loss_ppm: u32,
    pub jitter_ms: u32,
    pub stability_score: u32,
    pub coord_error_ppm: u32,
    pub last_success_at_unix_s: u64,
    pub score: i64,
}
