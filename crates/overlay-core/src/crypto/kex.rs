use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::CryptoError;

pub const X25519_KEY_LEN: usize = 32;

#[derive(Clone, Copy)]
pub struct X25519StaticSecret([u8; X25519_KEY_LEN]);

impl X25519StaticSecret {
    pub const fn from_bytes(bytes: [u8; X25519_KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; X25519_KEY_LEN] {
        &self.0
    }

    pub fn public_key(&self) -> X25519PublicKey {
        let secret = StaticSecret::from(self.0);
        X25519PublicKey::from_bytes(PublicKey::from(&secret).to_bytes())
    }

    pub fn diffie_hellman(
        &self,
        peer_public_key: &X25519PublicKey,
    ) -> Result<X25519SharedSecret, CryptoError> {
        let secret = StaticSecret::from(self.0);
        let peer_public_key = PublicKey::from(*peer_public_key.as_bytes());
        let shared_secret = secret.diffie_hellman(&peer_public_key).to_bytes();
        if shared_secret == [0_u8; X25519_KEY_LEN] {
            return Err(CryptoError::ReplayUnsafeSharedSecret);
        }

        Ok(X25519SharedSecret(shared_secret))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct X25519PublicKey([u8; X25519_KEY_LEN]);

impl X25519PublicKey {
    pub const fn from_bytes(bytes: [u8; X25519_KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; X25519_KEY_LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X25519SharedSecret([u8; X25519_KEY_LEN]);

impl X25519SharedSecret {
    pub const fn as_bytes(&self) -> &[u8; X25519_KEY_LEN] {
        &self.0
    }
}
