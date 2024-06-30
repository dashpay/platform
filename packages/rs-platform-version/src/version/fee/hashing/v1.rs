use crate::version::fee::hashing::FeeHashingVersion;

pub const FEE_HASHING_VERSION1: FeeHashingVersion = FeeHashingVersion {
    single_sha256_base: 100,
    sha256_per_block: 400,
};
