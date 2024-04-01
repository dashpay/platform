use crate::version::fee::hashing::v1::FEE_HASHING_VERSION1;
use crate::version::fee::processing::v1::FEE_PROCESSING_VERSION1;
use crate::version::fee::signature::v1::FEE_SIGNATURE_VERSION1;
use crate::version::fee::storage::v1::FEE_STORAGE_VERSION1;
use crate::version::fee::FeeVersion;

pub const FEE_VERSION1: FeeVersion = FeeVersion {
    storage: FEE_STORAGE_VERSION1,
    signature: FEE_SIGNATURE_VERSION1,
    hashing: FEE_HASHING_VERSION1,
    processing: FEE_PROCESSING_VERSION1,
};
