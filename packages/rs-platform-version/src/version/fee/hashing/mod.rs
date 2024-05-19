pub mod v1;

#[derive(Clone, Debug, Default)]
#[ferment_macro::export]
pub struct FeeHashingVersion {
    pub double_sha256_base: u64,
    pub double_sha256_per_block: u64,
    pub single_sha256_base: u64,
    pub single_sha256_per_block: u64,
}
