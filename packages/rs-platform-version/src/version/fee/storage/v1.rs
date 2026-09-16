use crate::version::fee::storage::FeeStorageVersion;

// these fees were originally calculated based on a cost of 30 $ / Dash

pub const FEE_STORAGE_VERSION1: FeeStorageVersion = FeeStorageVersion {
    storage_disk_usage_credit_per_byte: 27000,
    storage_processing_credit_per_byte: 400,
    storage_load_credit_per_byte: 20,
    non_storage_load_credit_per_byte: 10,
    storage_seek_cost: 2000,
    // Dead below protocol v14: the `ttl` grammar does not parse there, so
    // no ephemeral-classified operation can exist and nothing reads this
    // rate — carrying the live value here changes no released behavior.
    //
    // The rate prices a byte that provably lives at most one week (the
    // `ttl` cap) plus a bounded drainage lag. Pro-rata against the
    // perpetual-retention price (27,000 credits/byte distributed over ~50
    // years) one week is ~10 credits/byte; 270 — one percent of the
    // storage price — keeps a ~27x margin for the drainage work the
    // triggering writes perform unbilled and for disk churn, while still
    // making windowed (trending) writes two orders of magnitude cheaper
    // than permanent ones.
    ttl_ephemeral_disk_usage_credit_per_byte: 270,
};
