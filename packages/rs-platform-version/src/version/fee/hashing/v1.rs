use crate::version::fee::hashing::FeeHashingVersion;

pub const FEE_HASHING_VERSION1: FeeHashingVersion = FeeHashingVersion {
    double_sha256_base: 500,
    double_sha256_per_block: 400,
    single_sha256_base: 100,
    single_sha256_per_block: 400,
};
