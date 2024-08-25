use crate::version::fee::hashing::FeeHashingVersion;

pub const FEE_HASHING_VERSION1: FeeHashingVersion = FeeHashingVersion {
    single_sha256_base: 100,
    blake3_base: 100,
    sha256_ripe_md160_base: 6000,
    sha256_per_block: 5000,
    blake3_per_block: 300,
};
