/// Fee errors
#[derive(Debug, thiserror::Error)]
pub enum FeeError {
    /// Overflow error
    // TODO: Revisit
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// Corrupted storage fee not an item error
    #[error("corrupted storage fee not an item error: {0}")]
    CorruptedStorageFeeNotItem(&'static str),
    /// Corrupted storage fee invalid item length error
    #[error("corrupted storage fee invalid item length error: {0}")]
    CorruptedStorageFeeInvalidItemLength(&'static str),
    /// Corrupted processing fee not an item error
    #[error("corrupted processing fee not an item error: {0}")]
    CorruptedProcessingFeeNotItem(&'static str),
    /// Corrupted processing fee invalid item length error
    #[error("corrupted processing fee invalid item length error: {0}")]
    CorruptedProcessingFeeInvalidItemLength(&'static str),
    /// Corrupted start time not an item error
    #[error("corrupted start time not an item error")]
    CorruptedStartTimeNotItem(),
    /// Corrupted start time invalid item length error
    #[error("corrupted start time invalid item length error")]
    CorruptedStartTimeLength(),
    /// Corrupted start block height not an item error
    #[error("corrupted start block height not an item")]
    CorruptedStartBlockHeightNotItem(),
    /// Corrupted start block height invalid item length error
    #[error("corrupted start block height invalid item length")]
    CorruptedStartBlockHeightItemLength(),
    /// Corrupted proposer block count not an item error
    #[error("corrupted proposer block count not an item error: {0}")]
    CorruptedProposerBlockCountNotItem(&'static str),
    /// Corrupted proposer block count invalid item length error
    #[error("corrupted proposer block count invalid item length error: {0}")]
    CorruptedProposerBlockCountItemLength(&'static str),
    /// Corrupted storage fee pool not an item error
    #[error("corrupted storage fee pool not an item error: {0}")]
    CorruptedStorageFeePoolNotItem(&'static str),
    /// Corrupted storage fee pool invalid item length error
    #[error("corrupted storage fee pool invalid item length error: {0}")]
    CorruptedStorageFeePoolInvalidItemLength(&'static str),
    /// Corrupted multiplier not an item error
    #[error("corrupted multiplier not an item error: {0}")]
    CorruptedMultiplierNotItem(&'static str),
    /// Corrupted multiplier invalid item length error
    #[error("corrupted multiplier invalid item length error: {0}")]
    CorruptedMultiplierInvalidItemLength(&'static str),
    /// Corrupted unpaid epoch index invalid item length error
    #[error("corrupted unpaid epoch index invalid item length error: {0}")]
    CorruptedUnpaidEpochIndexItemLength(&'static str),
    /// Corrupted unpaid epoch index not an item error
    #[error("corrupted unpaid epoch index not an item error: {0}")]
    CorruptedUnpaidEpochIndexNotItem(&'static str),
    /// Corrupted code execution error
    #[error("corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),
    /// Decimal conversion error
    #[error("decimal conversion error: {0}")]
    DecimalConversion(&'static str),
}
