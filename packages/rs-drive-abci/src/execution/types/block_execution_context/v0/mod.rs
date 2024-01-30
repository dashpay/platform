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

use crate::execution::types::block_state_info::BlockStateInfo;

use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform_state::PlatformState;
use dashcore_rpc::dashcore::Txid;
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ResponsePrepareProposal;

/// V0 of the Block execution context
#[derive(Debug, Clone)]
pub struct BlockExecutionContextV0 {
    /// Block info
    pub block_state_info: BlockStateInfo,
    /// Epoch info
    pub epoch_info: EpochInfo,
    /// Total hpmn count
    pub hpmn_count: u32,
    /// Current withdrawal transactions hash -> Transaction
    pub withdrawal_transactions: BTreeMap<Txid, Vec<u8>>,
    /// Block state
    pub block_platform_state: PlatformState,
    /// The response prepare proposal if proposed by us
    pub proposer_results: Option<ResponsePrepareProposal>,
}
/// A trait defining getter methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0Getters {
    /// Returns the block_state_info field.
    fn block_state_info(&self) -> &BlockStateInfo;

    /// Returns a reference of the epoch_info field.
    fn epoch_info(&self) -> &EpochInfo;

    /// Returns the hpmn_count field.
    fn hpmn_count(&self) -> u32;

    /// Returns a reference of the withdrawal_transactions field.
    fn withdrawal_transactions(&self) -> &BTreeMap<Txid, Vec<u8>>;

    /// Returns a reference of the block_platform_state field.
    fn block_platform_state(&self) -> &PlatformState;

    /// Returns a reference of the proposer_results field.
    fn proposer_results(&self) -> Option<&ResponsePrepareProposal>;
}

/// A trait defining setter methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0Setters {
    /// Sets the block_state_info field.
    fn set_block_state_info(&mut self, info: BlockStateInfo);

    /// Sets the epoch_info field.
    fn set_epoch_info(&mut self, info: EpochInfo);

    /// Sets the hpmn_count field.
    fn set_hpmn_count(&mut self, count: u32);

    /// Sets the withdrawal_transactions field.
    fn set_withdrawal_transactions(&mut self, transactions: BTreeMap<Txid, Vec<u8>>);

    /// Sets the block_platform_state field.
    fn set_block_platform_state(&mut self, state: PlatformState);

    /// Sets the proposer_results field.
    fn set_proposer_results(&mut self, results: Option<ResponsePrepareProposal>);
}

/// A trait defining methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0MutableGetters {
    /// Returns a mutable reference to the block_state_info field.
    fn block_state_info_mut(&mut self) -> &mut BlockStateInfo;

    /// Returns a mutable reference to the epoch_info field.
    fn epoch_info_mut(&mut self) -> &mut EpochInfo;

    /// Returns a mutable reference to the withdrawal_transactions field.
    fn withdrawal_transactions_mut(&mut self) -> &mut BTreeMap<Txid, Vec<u8>>;

    /// Returns a mutable reference to the block_platform_state field.
    fn block_platform_state_mut(&mut self) -> &mut PlatformState;

    /// Returns a mutable reference to the proposer_results field.
    fn proposer_results_mut(&mut self) -> Option<&mut ResponsePrepareProposal>;
}

/// A trait defining methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0OwnedGetters {
    /// Consumes the BlockExecutionContextV0 and returns the block_state_info field.
    fn block_state_info_owned(self) -> BlockStateInfo;

    /// Consumes the BlockExecutionContextV0 and returns the epoch_info field.
    fn epoch_info_owned(self) -> EpochInfo;

    /// Consumes the BlockExecutionContextV0 and returns the withdrawal_transactions field.
    fn withdrawal_transactions_owned(self) -> BTreeMap<Txid, Vec<u8>>;

    /// Consumes the BlockExecutionContextV0 and returns the block_platform_state field.
    fn block_platform_state_owned(self) -> PlatformState;

    /// Consumes the BlockExecutionContextV0 and returns the proposer_results field.
    fn proposer_results_owned(self) -> Option<ResponsePrepareProposal>;
}

impl BlockExecutionContextV0Getters for BlockExecutionContextV0 {
    /// Returns a reference to the block_state_info field.
    fn block_state_info(&self) -> &BlockStateInfo {
        &self.block_state_info
    }

    /// Returns a reference to the epoch_info field.
    fn epoch_info(&self) -> &EpochInfo {
        &self.epoch_info
    }

    /// Returns the hpmn_count field.
    fn hpmn_count(&self) -> u32 {
        self.hpmn_count
    }

    /// Returns a reference to the withdrawal_transactions field.
    fn withdrawal_transactions(&self) -> &BTreeMap<Txid, Vec<u8>> {
        &self.withdrawal_transactions
    }

    /// Returns a reference to the block_platform_state field.
    fn block_platform_state(&self) -> &PlatformState {
        &self.block_platform_state
    }

    /// Returns a reference to the proposer_results field.
    fn proposer_results(&self) -> Option<&ResponsePrepareProposal> {
        self.proposer_results.as_ref()
    }
}

impl BlockExecutionContextV0Setters for BlockExecutionContextV0 {
    /// Sets the block_state_info field.
    fn set_block_state_info(&mut self, info: BlockStateInfo) {
        self.block_state_info = info;
    }
    /// Sets the epoch_info field.
    fn set_epoch_info(&mut self, info: EpochInfo) {
        self.epoch_info = info;
    }
    /// Sets the hpmn_count field.
    fn set_hpmn_count(&mut self, count: u32) {
        self.hpmn_count = count;
    }
    /// Sets the withdrawal_transactions field.
    fn set_withdrawal_transactions(&mut self, transactions: BTreeMap<Txid, Vec<u8>>) {
        self.withdrawal_transactions = transactions;
    }
    /// Sets the block_platform_state field.
    fn set_block_platform_state(&mut self, state: PlatformState) {
        self.block_platform_state = state;
    }
    /// Sets the proposer_results field.
    fn set_proposer_results(&mut self, results: Option<ResponsePrepareProposal>) {
        self.proposer_results = results;
    }
}

impl BlockExecutionContextV0MutableGetters for BlockExecutionContextV0 {
    /// Returns a mutable reference to the block_state_info field.
    fn block_state_info_mut(&mut self) -> &mut BlockStateInfo {
        &mut self.block_state_info
    }

    /// Returns a mutable reference to the epoch_info field.
    fn epoch_info_mut(&mut self) -> &mut EpochInfo {
        &mut self.epoch_info
    }

    /// Returns a mutable reference to the withdrawal_transactions field.
    fn withdrawal_transactions_mut(&mut self) -> &mut BTreeMap<Txid, Vec<u8>> {
        &mut self.withdrawal_transactions
    }

    /// Returns a mutable reference to the block_platform_state field.
    fn block_platform_state_mut(&mut self) -> &mut PlatformState {
        &mut self.block_platform_state
    }

    /// Returns a mutable reference to the proposer_results field.
    fn proposer_results_mut(&mut self) -> Option<&mut ResponsePrepareProposal> {
        self.proposer_results.as_mut()
    }
}

impl BlockExecutionContextV0OwnedGetters for BlockExecutionContextV0 {
    /// Consumes the object and returns the owned `BlockStateInfo`.
    fn block_state_info_owned(self) -> BlockStateInfo {
        self.block_state_info
    }

    /// Consumes the object and returns the owned `EpochInfo`.
    fn epoch_info_owned(self) -> EpochInfo {
        self.epoch_info
    }

    /// Consumes the object and returns the owned `BTreeMap` of withdrawal transactions.
    fn withdrawal_transactions_owned(self) -> BTreeMap<Txid, Vec<u8>> {
        self.withdrawal_transactions
    }

    /// Consumes the object and returns the owned `PlatformState`.
    fn block_platform_state_owned(self) -> PlatformState {
        self.block_platform_state
    }

    /// Consumes the object and returns the owned `ResponsePrepareProposal`.
    fn proposer_results_owned(self) -> Option<ResponsePrepareProposal> {
        self.proposer_results
    }
}
