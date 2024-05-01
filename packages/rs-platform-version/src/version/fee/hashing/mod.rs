pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct FeeHashingVersion {
    pub double_sha256_base: u64,
    pub double_sha256_per_block: u64,
    pub single_sha256_base: u64,
    pub single_sha256_per_block: u64,
}
