use crate::version::fee::storage::FeeStorageVersion;

/// Storage fees for protocol version 14 and above: V1 plus the TTL
/// ephemeral-bytes rate.
///
/// The rate prices a byte that provably lives at most one week (the
/// `ttl` cap) plus a bounded drainage lag. Pro-rata against the
/// perpetual-retention price (27,000 credits/byte distributed over ~50
/// years) one week is ~10 credits/byte; 270 — one percent of the
/// storage price — keeps a ~27x margin for the drainage work the
/// triggering writes perform unbilled and for disk churn, while still
/// making windowed (trending) writes two orders of magnitude cheaper
/// than permanent ones.
pub const FEE_STORAGE_VERSION2: FeeStorageVersion = FeeStorageVersion {
    storage_disk_usage_credit_per_byte: 27000,
    storage_processing_credit_per_byte: 400,
    storage_load_credit_per_byte: 20,
    non_storage_load_credit_per_byte: 10,
    storage_seek_cost: 2000,
    ttl_ephemeral_disk_usage_credit_per_byte: 270,
};
