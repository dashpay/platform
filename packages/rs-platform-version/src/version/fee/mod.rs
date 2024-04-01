use crate::version::fee::hashing::FeeHashingVersion;
use crate::version::fee::processing::FeeProcessingVersion;
use crate::version::fee::signature::FeeSignatureVersion;
use crate::version::fee::storage::FeeStorageVersion;

mod hashing;
mod processing;
pub mod signature;
pub mod storage;
pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct FeeVersion {
    pub storage: FeeStorageVersion,
    pub signature: FeeSignatureVersion,
    pub hashing: FeeHashingVersion,
    pub processing: FeeProcessingVersion,
}
