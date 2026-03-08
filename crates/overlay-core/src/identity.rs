use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::IdentityError;

pub const ID_LEN: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId([u8; ID_LEN]);

impl NodeId {
    pub const LEN: usize = ID_LEN;

    pub fn derive(node_public_key: &[u8]) -> Self {
        Self(*blake3::hash(node_public_key).as_bytes())
    }

    pub const fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, IdentityError> {
        let actual = bytes.len();
        let bytes = bytes.try_into().map_err(|_| IdentityError::InvalidLength {
            expected: Self::LEN,
            actual,
        })?;
        Ok(Self(bytes))
    }

    pub const fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }

    pub const fn into_bytes(self) -> [u8; ID_LEN] {
        self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self([0; ID_LEN])
    }
}

impl From<[u8; ID_LEN]> for NodeId {
    fn from(bytes: [u8; ID_LEN]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl AsRef<[u8]> for NodeId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NodeId(")?;
        write_hex(f, &self.0)?;
        f.write_str(")")
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(f, &self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppId([u8; ID_LEN]);

impl AppId {
    pub const LEN: usize = ID_LEN;

    pub fn derive(node_id: &NodeId, app_namespace: &str, app_name: &str) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(node_id.as_bytes());
        hasher.update(app_namespace.as_bytes());
        hasher.update(app_name.as_bytes());
        Self(*hasher.finalize().as_bytes())
    }

    pub const fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, IdentityError> {
        let actual = bytes.len();
        let bytes = bytes.try_into().map_err(|_| IdentityError::InvalidLength {
            expected: Self::LEN,
            actual,
        })?;
        Ok(Self(bytes))
    }

    pub const fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }

    pub const fn into_bytes(self) -> [u8; ID_LEN] {
        self.0
    }
}

impl Default for AppId {
    fn default() -> Self {
        Self([0; ID_LEN])
    }
}

impl From<[u8; ID_LEN]> for AppId {
    fn from(bytes: [u8; ID_LEN]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl AsRef<[u8]> for AppId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AppId(")?;
        write_hex(f, &self.0)?;
        f.write_str(")")
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(f, &self.0)
    }
}

pub fn derive_node_id(node_public_key: &[u8]) -> NodeId {
    NodeId::derive(node_public_key)
}

pub fn derive_app_id(node_id: &NodeId, app_namespace: &str, app_name: &str) -> AppId {
    AppId::derive(node_id, app_namespace, app_name)
}

fn write_hex(f: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{derive_app_id, derive_node_id, AppId, NodeId};
    use blake3::Hasher;

    #[test]
    fn derives_node_id_from_public_key_hash() {
        let public_key = [0x00, 0x11, 0x22, 0x33];
        let expected = *blake3::hash(&public_key).as_bytes();

        assert_eq!(derive_node_id(&public_key).into_bytes(), expected);
        assert_eq!(NodeId::derive(&public_key).into_bytes(), expected);
    }

    #[test]
    fn derives_app_id_from_node_id_namespace_and_name() {
        let node_id = NodeId::from_bytes(*blake3::hash(b"node-public-key").as_bytes());
        let mut hasher = Hasher::new();
        hasher.update(node_id.as_bytes());
        hasher.update(b"chat");
        hasher.update(b"terminal");
        let expected = *hasher.finalize().as_bytes();

        assert_eq!(
            derive_app_id(&node_id, "chat", "terminal").into_bytes(),
            expected
        );
        assert_eq!(
            AppId::derive(&node_id, "chat", "terminal").into_bytes(),
            expected
        );
    }
}
