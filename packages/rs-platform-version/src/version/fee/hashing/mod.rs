pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct FeeHashingVersion {
    pub double_sha256_base: u64,
    pub double_sha256_per_block: u64,
    pub single_sha256_base: u64,
    pub single_sha256_per_block: u64,
}

impl PartialEq for FeeHashingVersion {
    fn eq(&self, other: &Self) -> bool {
        self.double_sha256_base == other.double_sha256_base
            && self.double_sha256_per_block == other.double_sha256_per_block
            && self.single_sha256_base == other.single_sha256_base
            && self.single_sha256_per_block == other.single_sha256_per_block
    }
}
