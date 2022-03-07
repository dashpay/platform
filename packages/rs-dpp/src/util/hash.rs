use sha2::{Digest, Sha256};

pub fn hash(payload: impl AsRef<[u8]>) -> Vec<u8> {
    Sha256::digest(Sha256::digest(payload)).to_vec()
}
