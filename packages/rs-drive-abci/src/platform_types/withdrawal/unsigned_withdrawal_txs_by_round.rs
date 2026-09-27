//! The unsigned withdrawal transactions of every proposal accepted at the current height

use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use std::collections::BTreeMap;

/// The unsigned withdrawal transactions of one accepted proposal
#[derive(Debug, Clone)]
struct AcceptedProposalWithdrawals {
    block_hash: [u8; 32],
    transactions: UnsignedWithdrawalTxs,
}

/// The unsigned withdrawal transactions of every proposal this node accepted at the current
/// height, by round.
///
/// A withdrawal transaction carries the chain-locked core height of the proposal that built it
/// as its request height, so two rounds of one height whose proposers saw different chain locks
/// ask validators to sign different transactions. A vote extension is verified against the
/// proposal of its own round, which the block execution context alone cannot give: it only
/// holds the last proposal processed.
///
/// This is node memory, not consensus state.
#[derive(Debug, Default, Clone)]
pub struct UnsignedWithdrawalTxsByRound {
    height: u64,
    rounds: BTreeMap<u32, AcceptedProposalWithdrawals>,
}

impl UnsignedWithdrawalTxsByRound {
    /// Keeps the withdrawal transactions of the block `block_hash`, accepted at `height` and
    /// `round`, in place of any block kept for that round. Blocks of another height are
    /// forgotten.
    pub fn insert(
        &mut self,
        height: u64,
        round: u32,
        block_hash: [u8; 32],
        transactions: UnsignedWithdrawalTxs,
    ) {
        if self.height != height {
            self.rounds.clear();
            self.height = height;
        }

        self.rounds.insert(
            round,
            AcceptedProposalWithdrawals {
                block_hash,
                transactions,
            },
        );
    }

    /// The withdrawal transactions of the block `block_hash` at `height` and `round`, or `None`
    /// when this node has not accepted that block.
    pub fn get(
        &self,
        height: u64,
        round: u32,
        block_hash: &[u8],
    ) -> Option<&UnsignedWithdrawalTxs> {
        if self.height != height {
            return None;
        }

        self.rounds
            .get(&round)
            .filter(|proposal| proposal.block_hash.as_slice() == block_hash)
            .map(|proposal| &proposal.transactions)
    }

    /// Forgets every block, once their height is finalized
    pub fn clear(&mut self) {
        self.rounds.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::withdrawals::unsigned_withdrawal_transactions;
    use tenderdash_abci::proto::abci::ExtendVoteExtension;

    const ROUND_0_BLOCK: [u8; 32] = [0xA0; 32];
    const ROUND_1_BLOCK: [u8; 32] = [0xA1; 32];

    fn extensions(transactions: &UnsignedWithdrawalTxs) -> Vec<ExtendVoteExtension> {
        transactions.into()
    }

    #[test]
    fn should_keep_the_withdrawals_of_each_round_apart() {
        let round_0 = unsigned_withdrawal_transactions(1000);
        let round_1 = unsigned_withdrawal_transactions(1001);

        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, round_0.clone());
        by_round.insert(10, 1, ROUND_1_BLOCK, round_1.clone());

        let kept_round_0 = by_round
            .get(10, 0, &ROUND_0_BLOCK)
            .expect("round 0 is kept after round 1");
        let kept_round_1 = by_round
            .get(10, 1, &ROUND_1_BLOCK)
            .expect("round 1 is kept");

        assert_eq!(extensions(kept_round_0), extensions(&round_0));
        assert_eq!(extensions(kept_round_1), extensions(&round_1));
        assert_ne!(
            extensions(kept_round_0),
            extensions(kept_round_1),
            "test premise: the request height makes the two rounds' transactions differ"
        );
    }

    #[test]
    fn should_not_answer_for_a_block_or_round_it_did_not_accept() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, unsigned_withdrawal_transactions(1000));

        assert!(by_round.get(10, 0, &ROUND_1_BLOCK).is_none());
        assert!(by_round.get(10, 1, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(11, 0, &ROUND_0_BLOCK).is_none());
    }

    #[test]
    fn should_replace_the_block_kept_for_a_round() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, unsigned_withdrawal_transactions(1000));
        by_round.insert(10, 0, ROUND_1_BLOCK, unsigned_withdrawal_transactions(1001));

        assert!(by_round.get(10, 0, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(10, 0, &ROUND_1_BLOCK).is_some());
    }

    #[test]
    fn should_start_a_new_height_empty() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, unsigned_withdrawal_transactions(1000));
        by_round.insert(11, 1, ROUND_1_BLOCK, unsigned_withdrawal_transactions(1001));

        assert!(by_round.get(10, 0, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(11, 0, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(11, 1, &ROUND_1_BLOCK).is_some());
    }

    #[test]
    fn should_forget_every_block_when_cleared() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, unsigned_withdrawal_transactions(1000));
        by_round.insert(10, 1, ROUND_1_BLOCK, unsigned_withdrawal_transactions(1001));

        by_round.clear();

        assert!(by_round.get(10, 0, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(10, 1, &ROUND_1_BLOCK).is_none());
    }
}
