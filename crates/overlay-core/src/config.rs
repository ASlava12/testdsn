use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub node_key_path: String,
    pub bootstrap_sources: Vec<String>,
    pub max_total_neighbors: usize,
    pub max_presence_records: usize,
    pub max_service_records: usize,
    pub presence_ttl_s: u64,
    pub epoch_duration_s: u64,
    pub path_probe_interval_ms: u64,
    pub max_transport_buffer_bytes: usize,
    pub relay_mode: bool,
}
