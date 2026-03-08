use crate::ids::NodeId;

pub fn placement_keys(_node_id: &NodeId, _epoch: u64, replicas: usize) -> Vec<[u8; 32]> {
    vec![[0u8; 32]; replicas]
}
