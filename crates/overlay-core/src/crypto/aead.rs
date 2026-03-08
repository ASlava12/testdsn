use chacha20poly1305::{
    aead::{Aead, Payload},
    ChaCha20Poly1305, KeyInit, Nonce,
};
use serde::{Deserialize, Serialize};

use crate::error::CryptoError;

pub const CHACHA20POLY1305_KEY_LEN: usize = 32;
pub const CHACHA20POLY1305_NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChaCha20Poly1305Key([u8; CHACHA20POLY1305_KEY_LEN]);

impl ChaCha20Poly1305Key {
    pub const fn from_bytes(bytes: [u8; CHACHA20POLY1305_KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; CHACHA20POLY1305_KEY_LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChaCha20Poly1305Nonce([u8; CHACHA20POLY1305_NONCE_LEN]);

impl ChaCha20Poly1305Nonce {
    pub const fn from_bytes(bytes: [u8; CHACHA20POLY1305_NONCE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; CHACHA20POLY1305_NONCE_LEN] {
        &self.0
    }
}

pub fn encrypt(
    key: &ChaCha20Poly1305Key,
    nonce: &ChaCha20Poly1305Nonce,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key.as_bytes()).map_err(|_| CryptoError::AeadEncrypt)?;
    cipher
        .encrypt(
            Nonce::from_slice(nonce.as_bytes()),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CryptoError::AeadEncrypt)
}

pub fn decrypt(
    key: &ChaCha20Poly1305Key,
    nonce: &ChaCha20Poly1305Nonce,
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key.as_bytes()).map_err(|_| CryptoError::AeadDecrypt)?;
    cipher
        .decrypt(
            Nonce::from_slice(nonce.as_bytes()),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| CryptoError::AeadDecrypt)
}
