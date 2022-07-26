#[derive(Debug, thiserror::Error)]
pub enum FeeError {
    // TODO: Revisit
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    #[error("corrupted storage fee not an item error: {0}")]
    CorruptedStorageFeeNotItem(&'static str),
    #[error("corrupted storage fee invalid item length error: {0}")]
    CorruptedStorageFeeInvalidItemLength(&'static str),
    #[error("corrupted processing fee not an item error: {0}")]
    CorruptedProcessingFeeNotItem(&'static str),
    #[error("corrupted processing fee invalid item length error: {0}")]
    CorruptedProcessingFeeInvalidItemLength(&'static str),
    #[error("corrupted start time not an item error")]
    CorruptedStartTimeNotItem(),
    #[error("corrupted start time invalid item length error")]
    CorruptedStartTimeLength(),
    #[error("corrupted start block height not an item")]
    CorruptedStartBlockHeightNotItem(),
    #[error("corrupted start block height invalid item length")]
    CorruptedStartBlockHeightItemLength(),
    #[error("corrupted proposer block count not an item error: {0}")]
    CorruptedProposerBlockCountNotItem(&'static str),
    #[error("corrupted proposer block count invalid item length error: {0}")]
    CorruptedProposerBlockCountItemLength(&'static str),
    #[error("corrupted storage fee pool not an item error: {0}")]
    CorruptedStorageFeePoolNotItem(&'static str),
    #[error("corrupted storage fee pool invalid item length error: {0}")]
    CorruptedStorageFeePoolInvalidItemLength(&'static str),
    #[error("corrupted multiplier not an item error: {0}")]
    CorruptedMultiplierNotItem(&'static str),
    #[error("corrupted multiplier invalid item length error: {0}")]
    CorruptedMultiplierInvalidItemLength(&'static str),

    #[error("corrupted unpaid epoch index invalid item length error: {0}")]
    CorruptedUnpaidEpochIndexItemLength(&'static str),

    #[error("corrupted unpaid epoch index not an item error: {0}")]
    CorruptedUnpaidEpochIndexNotItem(&'static str),

    #[error("corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    #[error("decimal conversion error: {0}")]
    DecimalConversion(&'static str),
}
