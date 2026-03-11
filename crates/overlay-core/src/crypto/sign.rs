use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};

use crate::error::CryptoError;

pub const ED25519_SECRET_KEY_LEN: usize = 32;
pub const ED25519_PUBLIC_KEY_LEN: usize = 32;
pub const ED25519_SIGNATURE_LEN: usize = 64;

#[derive(Clone)]
pub struct Ed25519SigningKey([u8; ED25519_SECRET_KEY_LEN]);

impl Ed25519SigningKey {
    pub const fn from_seed(seed: [u8; ED25519_SECRET_KEY_LEN]) -> Self {
        Self(seed)
    }

    pub const fn as_bytes(&self) -> &[u8; ED25519_SECRET_KEY_LEN] {
        &self.0
    }

    pub fn public_key(&self) -> Ed25519PublicKey {
        let key = SigningKey::from_bytes(&self.0);
        Ed25519PublicKey::from_bytes(key.verifying_key().to_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> Ed25519Signature {
        let key = SigningKey::from_bytes(&self.0);
        Ed25519Signature::from_bytes(key.sign(message).to_bytes())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ed25519PublicKey([u8; ED25519_PUBLIC_KEY_LEN]);

impl Ed25519PublicKey {
    pub const fn from_bytes(bytes: [u8; ED25519_PUBLIC_KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_LEN] {
        &self.0
    }

    pub fn verify(&self, message: &[u8], signature: &Ed25519Signature) -> Result<(), CryptoError> {
        let verifying_key =
            VerifyingKey::from_bytes(&self.0).map_err(|_| CryptoError::InvalidEd25519PublicKey)?;
        let signature = ed25519_dalek::Signature::from_bytes(signature.as_bytes());
        verifying_key
            .verify_strict(message, &signature)
            .map_err(|_| CryptoError::SignatureVerificationFailed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ed25519Signature([u8; ED25519_SIGNATURE_LEN]);

impl Ed25519Signature {
    pub const fn from_bytes(bytes: [u8; ED25519_SIGNATURE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; ED25519_SIGNATURE_LEN] {
        &self.0
    }
}

impl Serialize for Ed25519Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Ed25519Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <Vec<u8>>::deserialize(deserializer)?;
        let actual = bytes.len();
        let array: [u8; ED25519_SIGNATURE_LEN] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(actual, &"64-byte Ed25519 signature"))?;
        Ok(Self(array))
    }
}
