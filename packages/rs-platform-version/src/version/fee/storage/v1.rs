use crate::version::fee::storage::FeeStorageVersion;

// these fees were originally calculated based on a cost of 30 $ / Dash

pub const FEE_STORAGE_VERSION1: FeeStorageVersion = FeeStorageVersion {
    storage_disk_usage_credit_per_byte: 27000,
    storage_processing_credit_per_byte: 400,
    storage_load_credit_per_byte: 20,
    non_storage_load_credit_per_byte: 10,
    storage_seek_cost: 2000,
};
