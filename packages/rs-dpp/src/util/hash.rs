use dashcore::hashes::{ripemd160, sha256, Hash};
use sha2::{Digest, Sha256};

pub fn hash_to_vec(payload: impl AsRef<[u8]>) -> Vec<u8> {
    Sha256::digest(Sha256::digest(payload)).to_vec()
}

pub fn hash(payload: impl AsRef<[u8]>) -> [u8; 32] {
    Sha256::digest(Sha256::digest(payload)).into()
}

pub fn hash_to_hex_string(payload: impl AsRef<[u8]>) -> String {
    hex::encode(hash(payload))
}

pub fn ripemd160_sha256(data: &[u8]) -> [u8; 20] {
    let hash = sha256::Hash::hash(data).to_byte_array();
    ripemd160::Hash::hash(&hash).to_byte_array()
}