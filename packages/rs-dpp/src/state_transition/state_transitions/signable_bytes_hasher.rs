use crate::util::hash::hash_double;
use platform_value::Bytes32;

/// This is a structure to hash signable bytes when we are not sure if we will need the hashing
#[derive(Debug, Clone)]
pub enum SignableBytesHasher {
    Bytes(Vec<u8>),
    PreHashed(Bytes32),
}

impl SignableBytesHasher {
    pub fn into_hashed_bytes(self) -> Bytes32 {
        match self {
            SignableBytesHasher::Bytes(signable_bytes) => hash_double(signable_bytes).into(),
            SignableBytesHasher::PreHashed(pre_hashed) => pre_hashed,
        }
    }

    pub fn to_hashed_bytes(&self) -> Bytes32 {
        match self {
            SignableBytesHasher::Bytes(signable_bytes) => hash_double(signable_bytes).into(),
            SignableBytesHasher::PreHashed(pre_hashed) => *pre_hashed,
        }
    }

    pub fn hash_bytes(&mut self) -> Bytes32 {
        match self {
            SignableBytesHasher::Bytes(signable_bytes) => {
                let bytes_32: Bytes32 = hash_double(signable_bytes).into();
                *self = SignableBytesHasher::PreHashed(bytes_32);
                bytes_32
            }
            SignableBytesHasher::PreHashed(pre_hashed) => *pre_hashed,
        }
    }

    pub fn hash_bytes_and_check_if_vec_contains(&mut self, vec: &[Bytes32]) -> bool {
        match self {
            SignableBytesHasher::Bytes(signable_bytes) => {
                let bytes_32: Bytes32 = hash_double(signable_bytes).into();
                let contains = vec.contains(&bytes_32);
                *self = SignableBytesHasher::PreHashed(bytes_32);
                contains
            }
            SignableBytesHasher::PreHashed(pre_hashed) => vec.contains(pre_hashed),
        }
    }
}
