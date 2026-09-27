//! The unsigned withdrawal transactions of every proposal accepted at the current height

use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ExtendVoteExtension;

/// The vote extensions validators sign for the withdrawal transactions of one accepted proposal
#[derive(Debug, Clone)]
struct AcceptedProposalWithdrawals {
    block_hash: [u8; 32],
    vote_extensions: Vec<ExtendVoteExtension>,
}

/// The unsigned withdrawal transactions of every proposal this node accepted at the current
/// height, by round, kept as the vote extensions validators sign for them.
///
/// A withdrawal transaction carries the chain-locked core height of the proposal that built it
/// as its request height, so two rounds of one height whose proposers saw different chain locks
/// ask validators to sign different transactions. A vote extension is verified against the
/// block it is for, which the block execution context alone cannot give: it only holds the last
/// proposal processed. A block's withdrawal transactions do not depend on the round it is
/// proposed in, so a block re-proposed in a later round asks for the same signatures.
///
/// This is node memory, not consensus state.
#[derive(Debug, Default, Clone)]
pub struct UnsignedWithdrawalTxsByRound {
    height: u64,
    rounds: BTreeMap<u32, AcceptedProposalWithdrawals>,
}

impl UnsignedWithdrawalTxsByRound {
    /// Keeps the vote extensions of the block `block_hash`, accepted at `height` and `round`, in
    /// place of any block kept for that round. Blocks of another height are forgotten.
    pub fn insert(
        &mut self,
        height: u64,
        round: u32,
        block_hash: [u8; 32],
        vote_extensions: Vec<ExtendVoteExtension>,
    ) {
        if self.height != height {
            self.rounds.clear();
            self.height = height;
        }

        self.rounds.insert(
            round,
            AcceptedProposalWithdrawals {
                block_hash,
                vote_extensions,
            },
        );
    }

    /// The vote extensions of the block `block_hash` at `height`, as accepted at `round` or, when
    /// this node accepted that block in another round, as accepted there. `None` when this node
    /// has not accepted that block.
    pub fn get(
        &self,
        height: u64,
        round: u32,
        block_hash: &[u8],
    ) -> Option<&[ExtendVoteExtension]> {
        if self.height != height {
            return None;
        }

        let is_block =
            |proposal: &&AcceptedProposalWithdrawals| proposal.block_hash.as_slice() == block_hash;

        self.rounds
            .get(&round)
            .filter(is_block)
            .or_else(|| self.rounds.values().find(is_block))
            .map(|proposal| proposal.vote_extensions.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::withdrawals::unsigned_withdrawal_transactions;

    const ROUND_0_BLOCK: [u8; 32] = [0xA0; 32];
    const ROUND_1_BLOCK: [u8; 32] = [0xA1; 32];

    fn extensions(core_height: u32) -> Vec<ExtendVoteExtension> {
        (&unsigned_withdrawal_transactions(core_height)).into()
    }

    #[test]
    fn should_keep_the_withdrawals_of_each_round_apart() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, extensions(1000));
        by_round.insert(10, 1, ROUND_1_BLOCK, extensions(1001));

        assert_eq!(
            by_round.get(10, 0, &ROUND_0_BLOCK),
            Some(extensions(1000).as_slice()),
            "round 0 is kept after round 1"
        );
        assert_eq!(
            by_round.get(10, 1, &ROUND_1_BLOCK),
            Some(extensions(1001).as_slice())
        );
        assert_ne!(
            extensions(1000),
            extensions(1001),
            "test premise: the request height makes the two rounds' transactions differ"
        );
    }

    #[test]
    fn should_answer_for_a_block_accepted_in_another_round() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, extensions(1000));

        assert_eq!(
            by_round.get(10, 1, &ROUND_0_BLOCK),
            Some(extensions(1000).as_slice())
        );
    }

    #[test]
    fn should_prefer_the_block_accepted_in_the_same_round() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, extensions(1000));
        by_round.insert(10, 1, ROUND_0_BLOCK, extensions(1001));

        assert_eq!(
            by_round.get(10, 1, &ROUND_0_BLOCK),
            Some(extensions(1001).as_slice())
        );
    }

    #[test]
    fn should_not_answer_for_a_block_it_did_not_accept() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, extensions(1000));

        assert!(by_round.get(10, 0, &ROUND_1_BLOCK).is_none());
        assert!(by_round.get(10, 1, &ROUND_1_BLOCK).is_none());
        assert!(by_round.get(11, 0, &ROUND_0_BLOCK).is_none());
    }

    #[test]
    fn should_replace_the_block_kept_for_a_round() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, extensions(1000));
        by_round.insert(10, 0, ROUND_1_BLOCK, extensions(1001));

        assert!(by_round.get(10, 0, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(10, 0, &ROUND_1_BLOCK).is_some());
    }

    #[test]
    fn should_start_a_new_height_empty() {
        let mut by_round = UnsignedWithdrawalTxsByRound::default();
        by_round.insert(10, 0, ROUND_0_BLOCK, vec![]);
        by_round.insert(11, 1, ROUND_1_BLOCK, extensions(1001));

        assert!(by_round.get(10, 0, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(11, 0, &ROUND_0_BLOCK).is_none());
        assert!(by_round.get(11, 1, &ROUND_1_BLOCK).is_some());
    }
}
