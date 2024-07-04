use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct FeeHashingVersion {
    pub blake3_base: u64,
    pub blake3_per_block: u64,
    pub sha256_per_block: u64,
    pub sha256_ripe_md160_base: u64,
    pub single_sha256_base: u64,

}
impl PartialEq for FeeHashingVersion {
    fn eq(&self, other: &Self) -> bool {
        self.blake3_base == other.blake3_base
            && self.blake3_per_block == other.blake3_per_block
            && self.sha256_per_block == other.sha256_per_block
            && self.sha256_ripe_md160_base == other.sha256_ripe_md160_base
            && self.single_sha256_base == other.single_sha256_base
    }
}
