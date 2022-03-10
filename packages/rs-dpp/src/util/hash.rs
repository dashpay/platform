use sha2::{Digest, Sha256};

pub fn sha(payload: impl AsRef<[u8]>) -> Vec<u8> {
    Sha256::digest(Sha256::digest(payload)).to_vec()
}
