use serde::{Deserialize, Serialize};

use crate::ids::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceRecord {
    pub version: u8,
    pub node_id: NodeId,
    pub epoch: u64,
    pub expires_at_unix_s: u64,
    pub sequence: u64,
    pub transport_classes: Vec<String>,
    pub reachability_mode: String,
    pub locator_commitment: [u8; 32],
    pub encrypted_contact_blobs: Vec<Vec<u8>>,
    pub relay_hint_refs: Vec<[u8; 32]>,
    pub intro_policy: String,
    pub capability_requirements: Vec<String>,
    pub signature: Vec<u8>,
}
