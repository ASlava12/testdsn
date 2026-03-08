use hkdf::Hkdf;
use sha2::Sha256;

use crate::error::CryptoError;

pub fn hkdf_sha256_expand(
    salt: &[u8],
    ikm: &[u8],
    info: &[u8],
    output: &mut [u8],
) -> Result<(), CryptoError> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), ikm);
    hkdf.expand(info, output)
        .map_err(|_| CryptoError::InvalidKdfLength { len: output.len() })
}
