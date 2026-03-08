use serde::{Deserialize, Serialize};

use crate::ids::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntroTicket {
    pub ticket_id: [u8; 32],
    pub target_node_id: NodeId,
    pub requester_binding: [u8; 32],
    pub scope: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub nonce: [u8; 32],
    pub signature: Vec<u8>,
}
