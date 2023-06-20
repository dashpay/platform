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
use crate::platform::block_proposal::v0::BlockProposal;
use dashcore_rpc::dashcore::hashes::hex::ToHex;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;

/// Block info
#[derive(Debug)]
pub struct BlockStateInfo {
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

impl BlockStateInfo {
    /// Gets a block info from the block state info
    pub fn to_block_info(&self, epoch: Epoch) -> BlockInfo {
        BlockInfo {
            time_ms: self.block_time_ms,
            height: self.height,
            core_height: self.core_chain_locked_height,
            epoch,
        }
    }
    /// Generate block state info based on Prepare Proposal request
    pub fn from_block_proposal(
        proposal: &BlockProposal,
        previous_block_time_ms: Option<u64>,
    ) -> BlockStateInfo {
        BlockStateInfo {
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

    /// Does this match a height and round?
    pub fn next_block_to(
        &self,
        previous_height: u64,
        previous_core_block_height: u32,
    ) -> Result<bool, Error> {
        Ok(self.height == previous_height + 1
            && self.core_chain_locked_height >= previous_core_block_height)
    }

    /// Does this match a height and round?
    pub fn matches_current_block<I: TryInto<[u8; 32]>>(
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
    pub fn matches_expected_block_info<I: TryInto<[u8; 32]>>(
        &self,
        height: u64,
        round: u32,
        core_block_height: u32,
        proposer_pro_tx_hash: [u8; 32],
        commit_hash: I,
    ) -> Result<bool, Error> {
        let received_hash = commit_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "can't convert hash as vec to [u8;32]".to_string(),
            ))
        })?;
        // the order is important here, don't verify commit hash before height and round
        tracing::trace!(
            self=?self,
            ?height,
            ?round,
            ?core_block_height,
            proposer_pro_tx_hash = proposer_pro_tx_hash.to_hex(),
            commit_hash = received_hash.to_hex(),
            "check if block info matches request"
        );
        Ok(self.height == height
            && self.round == round
            && self.core_chain_locked_height == core_block_height
            && self.proposer_pro_tx_hash == proposer_pro_tx_hash
            && self.block_hash.is_some()
            && self.block_hash.unwrap() == received_hash)
    }
}
