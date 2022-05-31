use dashcore::hashes::{ripemd160, sha256, Hash};
use sha2::{Digest, Sha256};

pub fn hash(payload: impl AsRef<[u8]>) -> Vec<u8> {
    Sha256::digest(Sha256::digest(payload)).to_vec()
}

pub fn ripemd160_sha256(data: &[u8]) -> Vec<u8> {
    ripemd160::Hash::hash(&sha256::Hash::hash(data)).to_vec()
}
