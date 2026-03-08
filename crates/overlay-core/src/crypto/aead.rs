pub struct SessionKeys {
    pub tx_key: [u8; 32],
    pub rx_key: [u8; 32],
}

// TODO: HKDF + ChaCha20-Poly1305 wrappers.
