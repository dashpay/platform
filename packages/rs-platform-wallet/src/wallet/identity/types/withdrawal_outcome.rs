//! What a credit withdrawal leaves behind: the identity's remaining balance
//! and the block that proved it.

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;

use super::BlockTime;

/// A withdrawal's result: the identity's remaining credits as Platform
/// proved them, dated by the proving block so a host can order it against
/// other balance reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithdrawalOutcome {
    /// The identity's credit balance after the withdrawal.
    pub remaining_credits: u64,
    /// The block whose proof carried `remaining_credits`.
    pub proved_at: BlockTime,
}

impl WithdrawalOutcome {
    /// `remaining_credits` as proved by the response `metadata` describes.
    pub(crate) fn proved(remaining_credits: u64, metadata: ResponseMetadata) -> Self {
        Self {
            remaining_credits,
            proved_at: BlockTime::from(metadata),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The balance is dated by the block of the proof that carried it.
    #[test]
    fn should_date_the_remaining_balance_by_the_proving_block() {
        let metadata = ResponseMetadata {
            height: 1234,
            core_chain_locked_height: 42,
            time_ms: 1_700_000_000_000,
            ..Default::default()
        };

        assert_eq!(
            WithdrawalOutcome::proved(7, metadata),
            WithdrawalOutcome {
                remaining_credits: 7,
                proved_at: BlockTime::new(1234, 42, 1_700_000_000_000),
            }
        );
    }
}
