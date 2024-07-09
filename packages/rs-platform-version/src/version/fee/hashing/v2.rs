use crate::version::fee::hashing::FeeHashingVersion;

pub const FEE_HASHING_VERSION2: FeeHashingVersion = FeeHashingVersion {
    single_sha256_base: 10000,
    blake3_base: 10000,
    sha256_ripe_md160_base: 10000,
    sha256_per_block: 40000,
    blake3_per_block: 10000,
};
