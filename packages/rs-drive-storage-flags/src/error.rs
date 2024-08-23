/// Storage flag errors
#[derive(Debug, thiserror::Error)]
pub enum StorageFlagsError {
    /// Error
    #[error("deserialize unknown storage flags type error: {0}")]
    DeserializeUnknownStorageFlagsType(&'static str),
    /// Error
    #[error("storage flags wrong size error: {0}")]
    StorageFlagsWrongSize(&'static str),
    /// Error
    #[error("removing at epoch with no associated storage error: {0}")]
    RemovingAtEpochWithNoAssociatedStorage(&'static str),
    /// Error
    #[error("storage flags overflow error: {0}")]
    StorageFlagsOverflow(&'static str),
    /// Error
    #[error("merging storage flags from different owners error: {0}")]
    MergingStorageFlagsFromDifferentOwners(&'static str),
    /// Error
    #[error("merging storage flags with different base epoch: {0}")]
    MergingStorageFlagsWithDifferentBaseEpoch(&'static str),
}
