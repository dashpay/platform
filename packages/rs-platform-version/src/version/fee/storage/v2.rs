use crate::version::fee::storage::FeeStorageVersion;

pub const FEE_STORAGE_VERSION2: FeeStorageVersion = FeeStorageVersion {
    storage_disk_usage_credit_per_byte: 2700000,
    storage_processing_credit_per_byte: 40000,
    storage_load_credit_per_byte: 40000,
    non_storage_load_credit_per_byte: 3000,
    storage_seek_cost: 400000,
};
