use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub fn sign(sk: &SigningKey, msg: &[u8]) -> Signature {
    sk.sign(msg)
}

pub fn verify(pk: &VerifyingKey, msg: &[u8], sig: &Signature) -> bool {
    pk.verify(msg, sig).is_ok()
}
