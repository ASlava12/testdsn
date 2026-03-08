use blake3::Hasher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppId(pub [u8; 32]);

pub fn node_id_from_pubkey(pubkey: &[u8]) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(pubkey);
    NodeId(*hasher.finalize().as_bytes())
}

pub fn app_id_from_parts(node_id: &NodeId, namespace: &str, app_name: &str) -> AppId {
    let mut hasher = Hasher::new();
    hasher.update(&node_id.0);
    hasher.update(namespace.as_bytes());
    hasher.update(app_name.as_bytes());
    AppId(*hasher.finalize().as_bytes())
}
