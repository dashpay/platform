use crate::version::fee::hashing::FeeHashingVersion;

pub const FEE_HASHING_VERSION1: FeeHashingVersion = FeeHashingVersion {
    single_sha256_base: 100,
    blake3_base: 100,            // TODO: Check this value
    sha256_ripe_md160_base: 100, // TODO: Check this value
    sha256_per_block: 400,
    blake3_per_block: 100, // TODO: Check this value
};
