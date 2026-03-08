use blake3::Hasher;

pub const BLAKE3_HASH_LEN: usize = 32;
pub type Blake3Digest = [u8; BLAKE3_HASH_LEN];

pub fn blake3_hash(input: &[u8]) -> Blake3Digest {
    *blake3::hash(input).as_bytes()
}

pub struct Blake3Hasher(Hasher);

impl Blake3Hasher {
    pub fn new() -> Self {
        Self(Hasher::new())
    }

    pub fn update(&mut self, input: &[u8]) {
        self.0.update(input);
    }

    pub fn finalize(self) -> Blake3Digest {
        *self.0.finalize().as_bytes()
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}
