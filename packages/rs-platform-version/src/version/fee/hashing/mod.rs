use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeHashingVersion {
    pub blake3_base: u64,
    pub blake3_per_block: u64,
    pub sha256_per_block: u64,
    pub sha256_ripe_md160_base: u64,
    pub single_sha256_base: u64,
    pub ripemd160_per_block: u64,
}

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeHashingVersionBeforeVersion11 {
    pub blake3_base: u64,
    pub blake3_per_block: u64,
    pub sha256_per_block: u64,
    pub sha256_ripe_md160_base: u64,
    pub single_sha256_base: u64,
}
