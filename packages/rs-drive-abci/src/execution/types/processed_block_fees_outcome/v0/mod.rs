use crate::execution::types::{fees_in_pools, proposer_payouts};

/// Holds info relevant fees and a processed block
#[derive(Debug)]
pub struct ProcessedBlockFeesOutcome {
    /// Amount of fees in the storage and processing fee distribution pools
    pub fees_in_pools: fees_in_pools::v0::FeesInPoolsV0,
    /// A struct with the number of proposers to be paid out and the last paid epoch index
    pub payouts: Option<proposer_payouts::v0::ProposersPayouts>,
    /// A number of epochs which had refunded
    pub refunded_epochs_count: Option<u16>,
}
