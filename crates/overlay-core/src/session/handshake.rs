use serde::Serialize;

use crate::{
    crypto::{
        aead::{
            decrypt, encrypt, ChaCha20Poly1305Key, ChaCha20Poly1305Nonce, CHACHA20POLY1305_KEY_LEN,
            CHACHA20POLY1305_NONCE_LEN,
        },
        hash::{Blake3Digest, Blake3Hasher},
        kdf::hkdf_sha256_expand,
        kex::{X25519SharedSecret, X25519StaticSecret},
        sign::{Ed25519PublicKey, Ed25519Signature, Ed25519SigningKey},
    },
    error::HandshakeError,
    identity::{derive_node_id, NodeId},
    wire::{Message, MessageType},
};

pub const HANDSHAKE_VERSION: u8 = 1;
const CLIENT_FINISH_AAD: &[u8] = b"overlay-mvp-client-finish";
const CLIENT_TO_SERVER_KEY_INFO: &[u8] = b"overlay-mvp/client-to-server";
const SERVER_TO_CLIENT_KEY_INFO: &[u8] = b"overlay-mvp/server-to-client";
const CLIENT_FINISH_KEY_INFO: &[u8] = b"overlay-mvp/client-finish-key";
const CLIENT_FINISH_NONCE_INFO: &[u8] = b"overlay-mvp/client-finish-nonce";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum HandshakeSuite {
    X25519Ed25519HkdfSha256ChaCha20Poly1305Blake3 = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeConfig {
    pub version: u8,
    pub suite: HandshakeSuite,
}

impl Default for HandshakeConfig {
    fn default() -> Self {
        Self {
            version: HANDSHAKE_VERSION,
            suite: HandshakeSuite::X25519Ed25519HkdfSha256ChaCha20Poly1305Blake3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientHello {
    pub version: u8,
    pub suite: HandshakeSuite,
    pub client_node_id: NodeId,
    pub client_signing_public_key: Ed25519PublicKey,
    pub client_ephemeral_public_key: crate::crypto::kex::X25519PublicKey,
}

impl ClientHello {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

impl Message for ClientHello {
    const TYPE: MessageType = MessageType::ClientHello;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerHello {
    pub version: u8,
    pub suite: HandshakeSuite,
    pub server_node_id: NodeId,
    pub server_signing_public_key: Ed25519PublicKey,
    pub server_ephemeral_public_key: crate::crypto::kex::X25519PublicKey,
    pub signature: Ed25519Signature,
}

impl ServerHello {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    fn unsigned(&self) -> UnsignedServerHello {
        UnsignedServerHello {
            version: self.version,
            suite: self.suite,
            server_node_id: self.server_node_id,
            server_signing_public_key: self.server_signing_public_key,
            server_ephemeral_public_key: self.server_ephemeral_public_key,
        }
    }
}

impl Message for ServerHello {
    const TYPE: MessageType = MessageType::ServerHello;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientFinish {
    pub version: u8,
    pub suite: HandshakeSuite,
    pub confirmation: Vec<u8>,
    pub signature: Ed25519Signature,
}

impl ClientFinish {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    fn unsigned(&self) -> UnsignedClientFinish {
        UnsignedClientFinish {
            version: self.version,
            suite: self.suite,
            confirmation: self.confirmation.clone(),
        }
    }
}

impl Message for ClientFinish {
    const TYPE: MessageType = MessageType::ClientFinish;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionKeys {
    pub client_to_server_key: ChaCha20Poly1305Key,
    pub server_to_client_key: ChaCha20Poly1305Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeOutcome {
    pub peer_node_id: NodeId,
    pub transcript_hash: Blake3Digest,
    pub session_keys: SessionKeys,
}

pub struct ClientHandshake {
    config: HandshakeConfig,
    client_signing_key: Ed25519SigningKey,
    client_ephemeral_secret: X25519StaticSecret,
    client_hello: ClientHello,
}

impl ClientHandshake {
    pub fn start(
        config: HandshakeConfig,
        client_signing_key: Ed25519SigningKey,
        client_ephemeral_secret: X25519StaticSecret,
    ) -> (Self, ClientHello) {
        let client_signing_public_key = client_signing_key.public_key();
        let client_hello = ClientHello {
            version: config.version,
            suite: config.suite,
            client_node_id: derive_node_id(client_signing_public_key.as_bytes()),
            client_signing_public_key,
            client_ephemeral_public_key: client_ephemeral_secret.public_key(),
        };
        let handshake = Self {
            config,
            client_signing_key,
            client_ephemeral_secret,
            client_hello: client_hello.clone(),
        };

        (handshake, client_hello)
    }

    pub fn handle_server_hello(
        self,
        server_hello: &ServerHello,
    ) -> Result<(ClientFinish, HandshakeOutcome), HandshakeError> {
        validate_client_hello(self.config, &self.client_hello)?;
        validate_server_hello(self.config, &self.client_hello, server_hello)?;

        let hello_transcript_hash = hello_transcript_hash(&self.client_hello, server_hello)?;
        let shared_secret = self
            .client_ephemeral_secret
            .diffie_hellman(&server_hello.server_ephemeral_public_key)?;
        let derived = derive_handshake_secrets(&shared_secret, &hello_transcript_hash)?;

        let unsigned_client_finish = UnsignedClientFinish {
            version: self.config.version,
            suite: self.config.suite,
            confirmation: build_client_finish_confirmation(&derived)?,
        };
        let finish_signature_input = client_finish_signature_input(
            &self.client_hello,
            server_hello,
            &unsigned_client_finish,
        )?;
        let client_finish = ClientFinish {
            version: unsigned_client_finish.version,
            suite: unsigned_client_finish.suite,
            confirmation: unsigned_client_finish.confirmation,
            signature: self.client_signing_key.sign(&finish_signature_input),
        };
        let outcome = HandshakeOutcome {
            peer_node_id: server_hello.server_node_id,
            transcript_hash: derived.hello_transcript_hash,
            session_keys: derived.session_keys,
        };

        Ok((client_finish, outcome))
    }
}

pub struct ServerHandshake {
    config: HandshakeConfig,
    client_hello: ClientHello,
    server_hello: ServerHello,
    derived: DerivedHandshakeSecrets,
}

impl ServerHandshake {
    pub fn accept(
        config: HandshakeConfig,
        server_signing_key: Ed25519SigningKey,
        server_ephemeral_secret: X25519StaticSecret,
        client_hello: &ClientHello,
    ) -> Result<(Self, ServerHello), HandshakeError> {
        validate_client_hello(config, client_hello)?;

        let server_signing_public_key = server_signing_key.public_key();
        let unsigned_server_hello = UnsignedServerHello {
            version: config.version,
            suite: config.suite,
            server_node_id: derive_node_id(server_signing_public_key.as_bytes()),
            server_signing_public_key,
            server_ephemeral_public_key: server_ephemeral_secret.public_key(),
        };
        let signature = server_signing_key.sign(&server_signature_input(
            client_hello,
            &unsigned_server_hello,
        )?);
        let server_hello = ServerHello {
            version: unsigned_server_hello.version,
            suite: unsigned_server_hello.suite,
            server_node_id: unsigned_server_hello.server_node_id,
            server_signing_public_key: unsigned_server_hello.server_signing_public_key,
            server_ephemeral_public_key: unsigned_server_hello.server_ephemeral_public_key,
            signature,
        };
        let hello_transcript_hash = hello_transcript_hash(client_hello, &server_hello)?;
        let shared_secret =
            server_ephemeral_secret.diffie_hellman(&client_hello.client_ephemeral_public_key)?;
        let derived = derive_handshake_secrets(&shared_secret, &hello_transcript_hash)?;
        let handshake = Self {
            config,
            client_hello: client_hello.clone(),
            server_hello: server_hello.clone(),
            derived,
        };

        Ok((handshake, server_hello))
    }

    pub fn handle_client_finish(
        self,
        client_finish: &ClientFinish,
    ) -> Result<HandshakeOutcome, HandshakeError> {
        validate_message_header(self.config, client_finish.version, client_finish.suite)?;

        let finish_signature_input = client_finish_signature_input(
            &self.client_hello,
            &self.server_hello,
            &client_finish.unsigned(),
        )?;
        self.client_hello
            .client_signing_public_key
            .verify(&finish_signature_input, &client_finish.signature)
            .map_err(|_| HandshakeError::InvalidSignature { role: "client" })?;

        let plaintext = decrypt(
            &self.derived.client_finish_key,
            &self.derived.client_finish_nonce,
            CLIENT_FINISH_AAD,
            &client_finish.confirmation,
        )
        .map_err(|_| HandshakeError::InvalidClientFinish)?;
        if plaintext.as_slice() != self.derived.hello_transcript_hash {
            return Err(HandshakeError::InvalidClientFinish);
        }

        Ok(HandshakeOutcome {
            peer_node_id: self.client_hello.client_node_id,
            transcript_hash: self.derived.hello_transcript_hash,
            session_keys: self.derived.session_keys,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct UnsignedServerHello {
    version: u8,
    suite: HandshakeSuite,
    server_node_id: NodeId,
    server_signing_public_key: Ed25519PublicKey,
    server_ephemeral_public_key: crate::crypto::kex::X25519PublicKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct UnsignedClientFinish {
    version: u8,
    suite: HandshakeSuite,
    confirmation: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DerivedHandshakeSecrets {
    hello_transcript_hash: Blake3Digest,
    client_finish_key: ChaCha20Poly1305Key,
    client_finish_nonce: ChaCha20Poly1305Nonce,
    session_keys: SessionKeys,
}

fn validate_client_hello(
    config: HandshakeConfig,
    client_hello: &ClientHello,
) -> Result<(), HandshakeError> {
    validate_message_header(config, client_hello.version, client_hello.suite)?;
    validate_node_id(
        "client",
        client_hello.client_node_id,
        client_hello.client_signing_public_key.as_bytes(),
    )
}

fn validate_server_hello(
    config: HandshakeConfig,
    client_hello: &ClientHello,
    server_hello: &ServerHello,
) -> Result<(), HandshakeError> {
    validate_message_header(config, server_hello.version, server_hello.suite)?;
    validate_node_id(
        "server",
        server_hello.server_node_id,
        server_hello.server_signing_public_key.as_bytes(),
    )?;

    let signature_input = server_signature_input(client_hello, &server_hello.unsigned())?;
    server_hello
        .server_signing_public_key
        .verify(&signature_input, &server_hello.signature)
        .map_err(|_| HandshakeError::InvalidSignature { role: "server" })
}

fn validate_message_header(
    config: HandshakeConfig,
    version: u8,
    suite: HandshakeSuite,
) -> Result<(), HandshakeError> {
    if version != config.version {
        return Err(HandshakeError::UnsupportedVersion {
            expected: config.version,
            actual: version,
        });
    }
    if suite != config.suite {
        return Err(HandshakeError::UnsupportedSuite);
    }

    Ok(())
}

fn validate_node_id(
    role: &'static str,
    claimed_node_id: NodeId,
    public_key_bytes: &[u8],
) -> Result<(), HandshakeError> {
    if claimed_node_id == derive_node_id(public_key_bytes) {
        return Ok(());
    }

    Err(HandshakeError::NodeIdMismatch { role })
}

fn server_signature_input(
    client_hello: &ClientHello,
    unsigned_server_hello: &UnsignedServerHello,
) -> Result<Blake3Digest, HandshakeError> {
    let client_hello_bytes = client_hello.canonical_bytes()?;
    let unsigned_server_hello_bytes = serde_json::to_vec(unsigned_server_hello)?;
    Ok(hash_transcript(&[
        (b"client_hello", &client_hello_bytes),
        (b"server_hello_unsigned", &unsigned_server_hello_bytes),
    ]))
}

fn hello_transcript_hash(
    client_hello: &ClientHello,
    server_hello: &ServerHello,
) -> Result<Blake3Digest, HandshakeError> {
    let client_hello_bytes = client_hello.canonical_bytes()?;
    let server_hello_bytes = server_hello.canonical_bytes()?;
    Ok(hash_transcript(&[
        (b"client_hello", &client_hello_bytes),
        (b"server_hello", &server_hello_bytes),
    ]))
}

fn client_finish_signature_input(
    client_hello: &ClientHello,
    server_hello: &ServerHello,
    unsigned_client_finish: &UnsignedClientFinish,
) -> Result<Blake3Digest, HandshakeError> {
    let client_hello_bytes = client_hello.canonical_bytes()?;
    let server_hello_bytes = server_hello.canonical_bytes()?;
    let unsigned_client_finish_bytes = serde_json::to_vec(unsigned_client_finish)?;
    Ok(hash_transcript(&[
        (b"client_hello", &client_hello_bytes),
        (b"server_hello", &server_hello_bytes),
        (b"client_finish_unsigned", &unsigned_client_finish_bytes),
    ]))
}

fn hash_transcript(chunks: &[(&[u8], &[u8])]) -> Blake3Digest {
    let mut hasher = Blake3Hasher::new();
    for (label, bytes) in chunks {
        hasher.update(&(label.len() as u32).to_be_bytes());
        hasher.update(label);
        hasher.update(&(bytes.len() as u32).to_be_bytes());
        hasher.update(bytes);
    }

    hasher.finalize()
}

fn derive_handshake_secrets(
    shared_secret: &X25519SharedSecret,
    hello_transcript_hash: &Blake3Digest,
) -> Result<DerivedHandshakeSecrets, HandshakeError> {
    let mut client_to_server_key = [0_u8; CHACHA20POLY1305_KEY_LEN];
    hkdf_sha256_expand(
        hello_transcript_hash,
        shared_secret.as_bytes(),
        CLIENT_TO_SERVER_KEY_INFO,
        &mut client_to_server_key,
    )?;

    let mut server_to_client_key = [0_u8; CHACHA20POLY1305_KEY_LEN];
    hkdf_sha256_expand(
        hello_transcript_hash,
        shared_secret.as_bytes(),
        SERVER_TO_CLIENT_KEY_INFO,
        &mut server_to_client_key,
    )?;

    let mut client_finish_key = [0_u8; CHACHA20POLY1305_KEY_LEN];
    hkdf_sha256_expand(
        hello_transcript_hash,
        shared_secret.as_bytes(),
        CLIENT_FINISH_KEY_INFO,
        &mut client_finish_key,
    )?;

    let mut client_finish_nonce = [0_u8; CHACHA20POLY1305_NONCE_LEN];
    hkdf_sha256_expand(
        hello_transcript_hash,
        shared_secret.as_bytes(),
        CLIENT_FINISH_NONCE_INFO,
        &mut client_finish_nonce,
    )?;

    Ok(DerivedHandshakeSecrets {
        hello_transcript_hash: *hello_transcript_hash,
        client_finish_key: ChaCha20Poly1305Key::from_bytes(client_finish_key),
        client_finish_nonce: ChaCha20Poly1305Nonce::from_bytes(client_finish_nonce),
        session_keys: SessionKeys {
            client_to_server_key: ChaCha20Poly1305Key::from_bytes(client_to_server_key),
            server_to_client_key: ChaCha20Poly1305Key::from_bytes(server_to_client_key),
        },
    })
}

fn build_client_finish_confirmation(
    derived: &DerivedHandshakeSecrets,
) -> Result<Vec<u8>, HandshakeError> {
    Ok(encrypt(
        &derived.client_finish_key,
        &derived.client_finish_nonce,
        CLIENT_FINISH_AAD,
        &derived.hello_transcript_hash,
    )?)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::crypto::aead::encrypt;

    const CLIENT_SIGNING_SEED: [u8; 32] = [7_u8; 32];
    const SERVER_SIGNING_SEED: [u8; 32] = [9_u8; 32];
    const CLIENT_EPHEMERAL_SECRET: [u8; 32] = [1_u8; 32];
    const SERVER_EPHEMERAL_SECRET: [u8; 32] = [2_u8; 32];

    #[test]
    fn valid_handshake_establishes_matching_session_keys() {
        let config = HandshakeConfig::default();
        let client_signing_key = Ed25519SigningKey::from_seed(CLIENT_SIGNING_SEED);
        let server_signing_key = Ed25519SigningKey::from_seed(SERVER_SIGNING_SEED);
        let client_ephemeral_secret = X25519StaticSecret::from_bytes(CLIENT_EPHEMERAL_SECRET);
        let server_ephemeral_secret = X25519StaticSecret::from_bytes(SERVER_EPHEMERAL_SECRET);

        let (client_handshake, client_hello) =
            ClientHandshake::start(config, client_signing_key.clone(), client_ephemeral_secret);
        let (server_handshake, server_hello) = ServerHandshake::accept(
            config,
            server_signing_key,
            server_ephemeral_secret,
            &client_hello,
        )
        .expect("server handshake should accept valid client hello");
        let (client_finish, client_outcome) = client_handshake
            .handle_server_hello(&server_hello)
            .expect("client should accept valid server hello");
        let server_outcome = server_handshake
            .handle_client_finish(&client_finish)
            .expect("server should accept valid client finish");

        assert_eq!(
            client_outcome.transcript_hash,
            server_outcome.transcript_hash
        );
        assert_eq!(client_outcome.session_keys, server_outcome.session_keys);
        assert_eq!(client_outcome.peer_node_id, server_hello.server_node_id);
        assert_eq!(server_outcome.peer_node_id, client_hello.client_node_id);
    }

    #[test]
    fn rejects_unsupported_handshake_version() {
        let config = HandshakeConfig::default();
        let client_signing_key = Ed25519SigningKey::from_seed(CLIENT_SIGNING_SEED);
        let client_ephemeral_secret = X25519StaticSecret::from_bytes(CLIENT_EPHEMERAL_SECRET);
        let (_, mut client_hello) =
            ClientHandshake::start(config, client_signing_key, client_ephemeral_secret);
        client_hello.version = 0;

        let error = match ServerHandshake::accept(
            config,
            Ed25519SigningKey::from_seed(SERVER_SIGNING_SEED),
            X25519StaticSecret::from_bytes(SERVER_EPHEMERAL_SECRET),
            &client_hello,
        ) {
            Ok(_) => panic!("server should reject unsupported versions"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            HandshakeError::UnsupportedVersion {
                expected: HANDSHAKE_VERSION,
                actual: 0
            }
        ));
    }

    #[test]
    fn rejects_invalid_server_signature() {
        let config = HandshakeConfig::default();
        let (client_handshake, client_hello) = ClientHandshake::start(
            config,
            Ed25519SigningKey::from_seed(CLIENT_SIGNING_SEED),
            X25519StaticSecret::from_bytes(CLIENT_EPHEMERAL_SECRET),
        );
        let (_, mut server_hello) = ServerHandshake::accept(
            config,
            Ed25519SigningKey::from_seed(SERVER_SIGNING_SEED),
            X25519StaticSecret::from_bytes(SERVER_EPHEMERAL_SECRET),
            &client_hello,
        )
        .expect("server hello should build");
        server_hello.signature = Ed25519Signature::from_bytes([0_u8; 64]);

        let error = client_handshake
            .handle_server_hello(&server_hello)
            .expect_err("client should reject tampered server signatures");

        assert!(matches!(
            error,
            HandshakeError::InvalidSignature { role: "server" }
        ));
    }

    #[test]
    fn rejects_invalid_client_finish_confirmation() {
        let config = HandshakeConfig::default();
        let client_signing_key = Ed25519SigningKey::from_seed(CLIENT_SIGNING_SEED);
        let client_ephemeral_secret = X25519StaticSecret::from_bytes(CLIENT_EPHEMERAL_SECRET);
        let server_ephemeral_secret = X25519StaticSecret::from_bytes(SERVER_EPHEMERAL_SECRET);

        let (client_handshake, client_hello) =
            ClientHandshake::start(config, client_signing_key.clone(), client_ephemeral_secret);
        let (server_handshake, server_hello) = ServerHandshake::accept(
            config,
            Ed25519SigningKey::from_seed(SERVER_SIGNING_SEED),
            server_ephemeral_secret,
            &client_hello,
        )
        .expect("server hello should build");
        let _ = client_handshake
            .handle_server_hello(&server_hello)
            .expect("client should accept valid server hello");

        let wrong_hello_transcript_hash = [0xAA_u8; 32];
        let shared_secret = client_ephemeral_secret
            .diffie_hellman(&server_hello.server_ephemeral_public_key)
            .expect("shared secret should derive");
        let derived = derive_handshake_secrets(&shared_secret, &wrong_hello_transcript_hash)
            .expect("wrong transcript should still derive deterministic secrets");
        let unsigned_client_finish = UnsignedClientFinish {
            version: HANDSHAKE_VERSION,
            suite: config.suite,
            confirmation: encrypt(
                &derived.client_finish_key,
                &derived.client_finish_nonce,
                CLIENT_FINISH_AAD,
                &wrong_hello_transcript_hash,
            )
            .expect("confirmation should encrypt"),
        };
        let signature_input =
            client_finish_signature_input(&client_hello, &server_hello, &unsigned_client_finish)
                .expect("signature input should serialize");
        let invalid_client_finish = ClientFinish {
            version: unsigned_client_finish.version,
            suite: unsigned_client_finish.suite,
            confirmation: unsigned_client_finish.confirmation,
            signature: client_signing_key.sign(&signature_input),
        };

        let error = server_handshake
            .handle_client_finish(&invalid_client_finish)
            .expect_err("server should reject invalid client finish confirmation");

        assert!(matches!(error, HandshakeError::InvalidClientFinish));
    }

    #[test]
    fn handshake_vector_matches_fixture() {
        let vector = deterministic_handshake_vector();
        let fixture = read_handshake_vector();

        assert_string_eq(
            "client_signing_seed_hex",
            &vector.client_signing_seed_hex,
            &fixture.client_signing_seed_hex,
        );
        assert_string_eq(
            "server_signing_seed_hex",
            &vector.server_signing_seed_hex,
            &fixture.server_signing_seed_hex,
        );
        assert_string_eq(
            "client_ephemeral_secret_hex",
            &vector.client_ephemeral_secret_hex,
            &fixture.client_ephemeral_secret_hex,
        );
        assert_string_eq(
            "server_ephemeral_secret_hex",
            &vector.server_ephemeral_secret_hex,
            &fixture.server_ephemeral_secret_hex,
        );
        assert_string_eq(
            "client_hello_hex",
            &vector.client_hello_hex,
            &fixture.client_hello_hex,
        );
        assert_string_eq(
            "server_hello_hex",
            &vector.server_hello_hex,
            &fixture.server_hello_hex,
        );
        assert_string_eq(
            "client_finish_hex",
            &vector.client_finish_hex,
            &fixture.client_finish_hex,
        );
        assert_string_eq(
            "transcript_hash_hex",
            &vector.transcript_hash_hex,
            &fixture.transcript_hash_hex,
        );
        assert_string_eq(
            "client_to_server_key_hex",
            &vector.client_to_server_key_hex,
            &fixture.client_to_server_key_hex,
        );
        assert_string_eq(
            "server_to_client_key_hex",
            &vector.server_to_client_key_hex,
            &fixture.server_to_client_key_hex,
        );
    }

    fn deterministic_handshake_vector() -> HandshakeVector {
        let config = HandshakeConfig::default();
        let client_signing_key = Ed25519SigningKey::from_seed(CLIENT_SIGNING_SEED);
        let client_ephemeral_secret = X25519StaticSecret::from_bytes(CLIENT_EPHEMERAL_SECRET);
        let server_signing_key = Ed25519SigningKey::from_seed(SERVER_SIGNING_SEED);
        let server_ephemeral_secret = X25519StaticSecret::from_bytes(SERVER_EPHEMERAL_SECRET);

        let (client_handshake, client_hello) =
            ClientHandshake::start(config, client_signing_key, client_ephemeral_secret);
        let (server_handshake, server_hello) = ServerHandshake::accept(
            config,
            server_signing_key,
            server_ephemeral_secret,
            &client_hello,
        )
        .expect("server handshake should accept vector input");
        let (client_finish, client_outcome) = client_handshake
            .handle_server_hello(&server_hello)
            .expect("client should accept vector server hello");
        let server_outcome = server_handshake
            .handle_client_finish(&client_finish)
            .expect("server should accept vector client finish");

        assert_eq!(
            client_outcome.transcript_hash,
            server_outcome.transcript_hash
        );
        assert_eq!(client_outcome.session_keys, server_outcome.session_keys);

        HandshakeVector {
            client_signing_seed_hex: encode_hex(&CLIENT_SIGNING_SEED),
            server_signing_seed_hex: encode_hex(&SERVER_SIGNING_SEED),
            client_ephemeral_secret_hex: encode_hex(&CLIENT_EPHEMERAL_SECRET),
            server_ephemeral_secret_hex: encode_hex(&SERVER_EPHEMERAL_SECRET),
            client_hello_hex: encode_hex(
                &client_hello
                    .canonical_bytes()
                    .expect("client hello should serialize"),
            ),
            server_hello_hex: encode_hex(
                &server_hello
                    .canonical_bytes()
                    .expect("server hello should serialize"),
            ),
            client_finish_hex: encode_hex(
                &client_finish
                    .canonical_bytes()
                    .expect("client finish should serialize"),
            ),
            transcript_hash_hex: encode_hex(&client_outcome.transcript_hash),
            client_to_server_key_hex: encode_hex(
                client_outcome.session_keys.client_to_server_key.as_bytes(),
            ),
            server_to_client_key_hex: encode_hex(
                client_outcome.session_keys.server_to_client_key.as_bytes(),
            ),
        }
    }

    fn encode_hex(bytes: &[u8]) -> String {
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").expect("hex encoding should succeed");
        }

        encoded
    }

    fn assert_string_eq(name: &str, actual: &str, expected: &str) {
        if actual == expected {
            return;
        }

        let mismatch_index = actual
            .bytes()
            .zip(expected.bytes())
            .position(|(left, right)| left != right)
            .unwrap_or_else(|| actual.len().min(expected.len()));
        let actual_start = mismatch_index.saturating_sub(12);
        let actual_end = (mismatch_index + 12).min(actual.len());
        let expected_start = mismatch_index.saturating_sub(12);
        let expected_end = (mismatch_index + 12).min(expected.len());

        panic!(
            "{name} mismatch at byte {mismatch_index}: actual {:?}, expected {:?}, lengths {} vs {}",
            &actual[actual_start..actual_end],
            &expected[expected_start..expected_end],
            actual.len(),
            expected.len(),
        );
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct HandshakeVector {
        client_signing_seed_hex: String,
        server_signing_seed_hex: String,
        client_ephemeral_secret_hex: String,
        server_ephemeral_secret_hex: String,
        client_hello_hex: String,
        server_hello_hex: String,
        client_finish_hex: String,
        transcript_hash_hex: String,
        client_to_server_key_hex: String,
        server_to_client_key_hex: String,
    }

    fn handshake_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("handshake_transcript.json")
    }

    fn read_handshake_vector() -> HandshakeVector {
        let bytes = fs::read(handshake_vector_path()).expect("vector file should exist");
        serde_json::from_slice(&bytes).expect("vector file should parse")
    }
}
