use serde::{Deserialize, Serialize};

use crate::ids::{AppId, NodeId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub version: u8,
    pub node_id: NodeId,
    pub app_id: AppId,
    pub service_name: String,
    pub service_version: String,
    pub auth_mode: String,
    pub policy: String,
    pub reachability_ref: [u8; 32],
    pub metadata_commitment: [u8; 32],
    pub signature: Vec<u8>,
}
