// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::block_proposal::v0::BlockProposal;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;

/// Block info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStateInfoV0 {
    /// Block height
    pub height: u64,
    /// Block round
    pub round: u32,
    /// Block time in ms
    pub block_time_ms: u64,
    /// Previous block time in ms
    pub previous_block_time_ms: Option<u64>,
    /// Block proposer's proTxHash
    pub proposer_pro_tx_hash: [u8; 32],
    /// Core chain locked height
    pub core_chain_locked_height: u32,
    /// Block hash, The block hash need not be known in the case for finalize block on the proposer
    pub block_hash: Option<[u8; 32]>,
    /// Application hash
    pub app_hash: Option<[u8; 32]>,
}

impl BlockStateInfoV0 {
    /// Generate block state info based on Prepare Proposal request
    pub fn from_block_proposal(
        proposal: &BlockProposal,
        previous_block_time_ms: Option<u64>,
    ) -> BlockStateInfoV0 {
        BlockStateInfoV0 {
            height: proposal.height,
            round: proposal.round,
            block_time_ms: proposal.block_time_ms,
            previous_block_time_ms,
            proposer_pro_tx_hash: proposal.proposer_pro_tx_hash,
            core_chain_locked_height: proposal.core_chain_locked_height,
            block_hash: proposal.block_hash,
            app_hash: None,
        }
    }
}

/// Methods created on version 0 of block state info
pub trait BlockStateInfoV0Methods {
    /// Gets a block info from the block state info
    fn to_block_info(&self, epoch: Epoch) -> BlockInfo;
    /// Does this match a height and round?
    fn next_block_to(
        &self,
        previous_height: u64,
        previous_core_block_height: u32,
    ) -> Result<bool, Error>;

    /// Does this match a height and round?
    fn matches_current_block<I: TryInto<[u8; 32]>>(
        &self,
        height: u64,
        round: u32,
        block_hash: I,
    ) -> Result<bool, Error>;

    /// Does this match a height and round?
    fn matches_expected_block_info<I: TryInto<[u8; 32]>>(
        &self,
        height: u64,
        round: u32,
        core_block_height: u32,
        proposer_pro_tx_hash: [u8; 32],
        commit_hash: I,
        app_hash: I,
    ) -> Result<bool, Error>;
}

impl BlockStateInfoV0Methods for BlockStateInfoV0 {
    /// Gets a block info from the block state info
    fn to_block_info(&self, epoch: Epoch) -> BlockInfo {
        BlockInfo {
            time_ms: self.block_time_ms,
            height: self.height,
            core_height: self.core_chain_locked_height,
            epoch,
        }
    }

    /// Does this match a height and round?
    fn next_block_to(
        &self,
        previous_height: u64,
        previous_core_block_height: u32,
    ) -> Result<bool, Error> {
        Ok(self.height == previous_height + 1
            && self.core_chain_locked_height >= previous_core_block_height)
    }

    /// Does this match a height and round?
    fn matches_current_block<I: TryInto<[u8; 32]>>(
        &self,
        height: u64,
        round: u32,
        block_hash: I,
    ) -> Result<bool, Error> {
        let received_hash = block_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "can't convert hash as vec to [u8;32]".to_string(),
            ))
        })?;
        // the order is important here, don't verify commit hash before height and round
        Ok(self.height == height
            && self.round == round
            && self.block_hash.is_some()
            && self.block_hash.unwrap() == received_hash)
    }

    /// Does this match a height and round?
    fn matches_expected_block_info<I: TryInto<[u8; 32]>>(
        &self,
        height: u64,
        round: u32,
        core_block_height: u32,
        proposer_pro_tx_hash: [u8; 32],
        commit_hash: I,
        app_hash: I,
    ) -> Result<bool, Error> {
        let received_hash = commit_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "can't convert hash as vec to [u8;32]".to_string(),
            ))
        })?;
        let received_app_hash = app_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "can't convert app hash as vec to [u8;32]".to_string(),
            ))
        })?;
        // the order is important here, don't verify commit hash before height and round
        tracing::trace!(
            self=?self,
            ?height,
            ?round,
            ?core_block_height,
            proposer_pro_tx_hash = hex::encode(proposer_pro_tx_hash),
            commit_hash = hex::encode(received_hash),
            app_hash = hex::encode(received_app_hash),
            "check if block info matches request"
        );

        Ok(self.height == height
            && self.round == round
            && self.core_chain_locked_height == core_block_height
            && self.proposer_pro_tx_hash == proposer_pro_tx_hash
            && self.block_hash.is_some()
            && self.block_hash.unwrap() == received_hash
            && self.app_hash.is_some()
            && self.app_hash.unwrap_or_default() == received_app_hash)
    }
}

/// A trait for getting the properties of the `BlockStateInfoV0`.
pub trait BlockStateInfoV0Getters {
    /// Gets the block height.
    fn height(&self) -> u64;

    /// Gets the block round.
    fn round(&self) -> u32;

    /// Gets the block time in ms.
    fn block_time_ms(&self) -> u64;

    /// Gets the previous block time in ms.
    fn previous_block_time_ms(&self) -> Option<u64>;

    /// Gets the block proposer's proTxHash.
    fn proposer_pro_tx_hash(&self) -> [u8; 32];

    /// Gets the core chain locked height.
    fn core_chain_locked_height(&self) -> u32;

    /// Gets the block hash.
    fn block_hash(&self) -> Option<[u8; 32]>;

    /// Gets the application hash.
    fn app_hash(&self) -> Option<[u8; 32]>;
}

/// A trait for setting the properties of the `BlockStateInfoV0`.
pub trait BlockStateInfoV0Setters {
    /// Sets the block height.
    fn set_height(&mut self, height: u64);

    /// Sets the block round.
    fn set_round(&mut self, round: u32);

    /// Sets the block time in ms.
    fn set_block_time_ms(&mut self, block_time_ms: u64);

    /// Sets the previous block time in ms.
    fn set_previous_block_time_ms(&mut self, previous_block_time_ms: Option<u64>);

    /// Sets the block proposer's proTxHash.
    fn set_proposer_pro_tx_hash(&mut self, proposer_pro_tx_hash: [u8; 32]);

    /// Sets the core chain locked height.
    fn set_core_chain_locked_height(&mut self, core_chain_locked_height: u32);

    /// Sets the block hash.
    fn set_block_hash(&mut self, block_hash: Option<[u8; 32]>);

    /// Sets the application hash.
    fn set_app_hash(&mut self, app_hash: Option<[u8; 32]>);
}

impl BlockStateInfoV0Getters for BlockStateInfoV0 {
    fn height(&self) -> u64 {
        self.height
    }

    fn round(&self) -> u32 {
        self.round
    }

    fn block_time_ms(&self) -> u64 {
        self.block_time_ms
    }

    fn previous_block_time_ms(&self) -> Option<u64> {
        self.previous_block_time_ms
    }

    fn proposer_pro_tx_hash(&self) -> [u8; 32] {
        self.proposer_pro_tx_hash
    }

    fn core_chain_locked_height(&self) -> u32 {
        self.core_chain_locked_height
    }

    fn block_hash(&self) -> Option<[u8; 32]> {
        self.block_hash
    }

    fn app_hash(&self) -> Option<[u8; 32]> {
        self.app_hash
    }
}

impl BlockStateInfoV0Setters for BlockStateInfoV0 {
    fn set_height(&mut self, height: u64) {
        self.height = height;
    }

    fn set_round(&mut self, round: u32) {
        self.round = round;
    }

    fn set_block_time_ms(&mut self, block_time_ms: u64) {
        self.block_time_ms = block_time_ms;
    }

    fn set_previous_block_time_ms(&mut self, previous_block_time_ms: Option<u64>) {
        self.previous_block_time_ms = previous_block_time_ms;
    }

    fn set_proposer_pro_tx_hash(&mut self, proposer_pro_tx_hash: [u8; 32]) {
        self.proposer_pro_tx_hash = proposer_pro_tx_hash;
    }

    fn set_core_chain_locked_height(&mut self, core_chain_locked_height: u32) {
        self.core_chain_locked_height = core_chain_locked_height;
    }

    fn set_block_hash(&mut self, block_hash: Option<[u8; 32]>) {
        self.block_hash = block_hash;
    }

    fn set_app_hash(&mut self, app_hash: Option<[u8; 32]>) {
        self.app_hash = app_hash;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::block_proposal::v0::BlockProposal;
    use dpp::block::epoch::Epoch;
    use tenderdash_abci::proto::version::Consensus;

    fn make_block_state_info() -> BlockStateInfoV0 {
        BlockStateInfoV0 {
            height: 10,
            round: 2,
            block_time_ms: 1_700_000_000_000,
            previous_block_time_ms: Some(1_699_999_990_000),
            proposer_pro_tx_hash: [1u8; 32],
            core_chain_locked_height: 500,
            block_hash: Some([0xAA; 32]),
            app_hash: Some([0xBB; 32]),
        }
    }

    #[test]
    fn getters_return_correct_values() {
        let info = make_block_state_info();
        assert_eq!(info.height(), 10);
        assert_eq!(info.round(), 2);
        assert_eq!(info.block_time_ms(), 1_700_000_000_000);
        assert_eq!(info.previous_block_time_ms(), Some(1_699_999_990_000));
        assert_eq!(info.proposer_pro_tx_hash(), [1u8; 32]);
        assert_eq!(info.core_chain_locked_height(), 500);
        assert_eq!(info.block_hash(), Some([0xAA; 32]));
        assert_eq!(info.app_hash(), Some([0xBB; 32]));
    }

    #[test]
    fn setters_update_values() {
        let mut info = make_block_state_info();
        info.set_height(20);
        info.set_round(5);
        info.set_block_time_ms(2_000_000_000_000);
        info.set_previous_block_time_ms(None);
        info.set_proposer_pro_tx_hash([2u8; 32]);
        info.set_core_chain_locked_height(600);
        info.set_block_hash(None);
        info.set_app_hash(None);

        assert_eq!(info.height(), 20);
        assert_eq!(info.round(), 5);
        assert_eq!(info.block_time_ms(), 2_000_000_000_000);
        assert_eq!(info.previous_block_time_ms(), None);
        assert_eq!(info.proposer_pro_tx_hash(), [2u8; 32]);
        assert_eq!(info.core_chain_locked_height(), 600);
        assert_eq!(info.block_hash(), None);
        assert_eq!(info.app_hash(), None);
    }

    #[test]
    fn to_block_info_produces_correct_block_info() {
        let info = make_block_state_info();
        let epoch = Epoch::new(3).unwrap();
        let block_info = info.to_block_info(epoch);
        assert_eq!(block_info.time_ms, 1_700_000_000_000);
        assert_eq!(block_info.height, 10);
        assert_eq!(block_info.core_height, 500);
        assert_eq!(block_info.epoch.index, 3);
    }

    #[test]
    fn next_block_to_returns_true_for_sequential_block() {
        let info = make_block_state_info(); // height=10, core_chain_locked_height=500
        assert!(info.next_block_to(9, 500).unwrap());
        assert!(info.next_block_to(9, 400).unwrap());
    }

    #[test]
    fn next_block_to_returns_false_for_non_sequential_height() {
        let info = make_block_state_info(); // height=10
        assert!(!info.next_block_to(10, 500).unwrap());
        assert!(!info.next_block_to(8, 500).unwrap());
    }

    #[test]
    fn next_block_to_returns_false_for_core_height_regression() {
        let info = make_block_state_info(); // core_chain_locked_height=500
        assert!(!info.next_block_to(9, 501).unwrap());
    }

    #[test]
    fn matches_current_block_returns_true_for_matching() {
        let info = make_block_state_info(); // height=10, round=2, block_hash=Some([0xAA; 32])
        assert!(info.matches_current_block(10, 2, [0xAA; 32]).unwrap());
    }

    #[test]
    fn matches_current_block_returns_false_for_wrong_height() {
        let info = make_block_state_info();
        assert!(!info.matches_current_block(11, 2, [0xAA; 32]).unwrap());
    }

    #[test]
    fn matches_current_block_returns_false_for_wrong_round() {
        let info = make_block_state_info();
        assert!(!info.matches_current_block(10, 3, [0xAA; 32]).unwrap());
    }

    #[test]
    fn matches_current_block_returns_false_for_wrong_hash() {
        let info = make_block_state_info();
        assert!(!info.matches_current_block(10, 2, [0xBB; 32]).unwrap());
    }

    #[test]
    fn matches_current_block_returns_false_when_no_block_hash() {
        let mut info = make_block_state_info();
        info.block_hash = None;
        assert!(!info.matches_current_block(10, 2, [0xAA; 32]).unwrap());
    }

    #[test]
    fn matches_current_block_error_on_bad_hash_size() {
        let info = make_block_state_info();
        // Provide a Vec that can't convert to [u8; 32]
        let bad_hash: Vec<u8> = vec![0u8; 16];
        let result = info.matches_current_block(10, 2, bad_hash);
        assert!(result.is_err());
    }

    #[test]
    fn matches_expected_block_info_returns_true_for_exact_match() {
        let info = make_block_state_info();
        assert!(info
            .matches_expected_block_info(10, 2, 500, [1u8; 32], [0xAA; 32], [0xBB; 32])
            .unwrap());
    }

    #[test]
    fn matches_expected_block_info_returns_false_for_wrong_core_height() {
        let info = make_block_state_info();
        assert!(!info
            .matches_expected_block_info(10, 2, 501, [1u8; 32], [0xAA; 32], [0xBB; 32])
            .unwrap());
    }

    #[test]
    fn matches_expected_block_info_returns_false_for_wrong_proposer() {
        let info = make_block_state_info();
        assert!(!info
            .matches_expected_block_info(10, 2, 500, [9u8; 32], [0xAA; 32], [0xBB; 32])
            .unwrap());
    }

    #[test]
    fn matches_expected_block_info_returns_false_for_wrong_app_hash() {
        let info = make_block_state_info();
        assert!(!info
            .matches_expected_block_info(10, 2, 500, [1u8; 32], [0xAA; 32], [0xCC; 32])
            .unwrap());
    }

    #[test]
    fn matches_expected_block_info_returns_false_when_no_block_hash() {
        let mut info = make_block_state_info();
        info.block_hash = None;
        assert!(!info
            .matches_expected_block_info(10, 2, 500, [1u8; 32], [0xAA; 32], [0xBB; 32])
            .unwrap());
    }

    #[test]
    fn matches_expected_block_info_returns_false_when_no_app_hash() {
        let mut info = make_block_state_info();
        info.app_hash = None;
        assert!(!info
            .matches_expected_block_info(10, 2, 500, [1u8; 32], [0xAA; 32], [0xBB; 32])
            .unwrap());
    }

    #[test]
    fn matches_expected_block_info_error_on_bad_commit_hash() {
        let info = make_block_state_info();
        let bad_hash: Vec<u8> = vec![0u8; 5];
        let good_app_hash: Vec<u8> = vec![0xBB; 32];
        let result =
            info.matches_expected_block_info(10, 2, 500, [1u8; 32], bad_hash, good_app_hash);
        assert!(result.is_err());
    }

    #[test]
    fn matches_expected_block_info_error_on_bad_app_hash() {
        let info = make_block_state_info();
        let good_commit_hash: Vec<u8> = vec![0xAA; 32];
        let bad_hash: Vec<u8> = vec![0u8; 5];
        let result =
            info.matches_expected_block_info(10, 2, 500, [1u8; 32], good_commit_hash, bad_hash);
        assert!(result.is_err());
    }

    #[test]
    fn from_block_proposal_creates_correct_state_info() {
        let empty_txs = vec![];
        let proposal = BlockProposal {
            consensus_versions: Consensus { block: 1, app: 1 },
            block_hash: Some([0xDD; 32]),
            height: 42,
            round: 3,
            block_time_ms: 1_700_000_000_000,
            core_chain_locked_height: 800,
            core_chain_lock_update: None,
            proposed_app_version: 1,
            proposer_pro_tx_hash: [5u8; 32],
            validator_set_quorum_hash: [0u8; 32],
            raw_state_transitions: &empty_txs,
        };
        let info = BlockStateInfoV0::from_block_proposal(&proposal, Some(1_699_999_000_000));
        assert_eq!(info.height(), 42);
        assert_eq!(info.round(), 3);
        assert_eq!(info.block_time_ms(), 1_700_000_000_000);
        assert_eq!(info.previous_block_time_ms(), Some(1_699_999_000_000));
        assert_eq!(info.proposer_pro_tx_hash(), [5u8; 32]);
        assert_eq!(info.core_chain_locked_height(), 800);
        assert_eq!(info.block_hash(), Some([0xDD; 32]));
        assert_eq!(info.app_hash(), None);
    }

    #[test]
    fn from_block_proposal_with_no_previous_time() {
        let empty_txs = vec![];
        let proposal = BlockProposal {
            consensus_versions: Consensus { block: 1, app: 1 },
            block_hash: None,
            height: 1,
            round: 0,
            block_time_ms: 1_000_000,
            core_chain_locked_height: 1,
            core_chain_lock_update: None,
            proposed_app_version: 1,
            proposer_pro_tx_hash: [0u8; 32],
            validator_set_quorum_hash: [0u8; 32],
            raw_state_transitions: &empty_txs,
        };
        let info = BlockStateInfoV0::from_block_proposal(&proposal, None);
        assert_eq!(info.previous_block_time_ms(), None);
        assert_eq!(info.block_hash(), None);
    }
}
