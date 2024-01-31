/// Version 0
pub mod v0;

use crate::error::Error;
use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0Getters, BlockExecutionContextV0MutableGetters,
    BlockExecutionContextV0OwnedGetters, BlockExecutionContextV0Setters,
};
use crate::execution::types::block_state_info::BlockStateInfo;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform_state::PlatformState;
use derive_more::From;
use dpp::dashcore::Txid;
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ResponsePrepareProposal;

/// The versioned block execution context
#[derive(Debug, From, Clone)]
pub enum BlockExecutionContext {
    /// Version 0
    V0(v0::BlockExecutionContextV0),
}

impl BlockExecutionContext {
    /// version v0 as a reference or an error if not holding v0
    pub fn v0(&self) -> Result<&v0::BlockExecutionContextV0, Error> {
        match self {
            BlockExecutionContext::V0(v) => Ok(v),
            //_ => Err(Error::Execution(ExecutionError::CorruptedCodeVersionMismatch("block execution context mismatch"))),
        }
    }
}
impl BlockExecutionContextV0Getters for BlockExecutionContext {
    fn block_state_info(&self) -> &BlockStateInfo {
        match self {
            BlockExecutionContext::V0(v0) => &v0.block_state_info,
        }
    }

    fn epoch_info(&self) -> &EpochInfo {
        match self {
            BlockExecutionContext::V0(v0) => &v0.epoch_info,
        }
    }

    fn hpmn_count(&self) -> u32 {
        match self {
            BlockExecutionContext::V0(v0) => v0.hpmn_count,
        }
    }

    fn withdrawal_transactions(&self) -> &BTreeMap<Txid, Vec<u8>> {
        match self {
            BlockExecutionContext::V0(v0) => &v0.withdrawal_transactions,
        }
    }

    fn block_platform_state(&self) -> &PlatformState {
        match self {
            BlockExecutionContext::V0(v0) => &v0.block_platform_state,
        }
    }

    fn proposer_results(&self) -> Option<&ResponsePrepareProposal> {
        match self {
            BlockExecutionContext::V0(v0) => v0.proposer_results.as_ref(),
        }
    }
}

impl BlockExecutionContextV0Setters for BlockExecutionContext {
    fn set_block_state_info(&mut self, info: BlockStateInfo) {
        match self {
            BlockExecutionContext::V0(v0) => v0.block_state_info = info,
        }
    }

    fn set_epoch_info(&mut self, info: EpochInfo) {
        match self {
            BlockExecutionContext::V0(v0) => v0.epoch_info = info,
        }
    }

    fn set_hpmn_count(&mut self, count: u32) {
        match self {
            BlockExecutionContext::V0(v0) => v0.hpmn_count = count,
        }
    }

    fn set_withdrawal_transactions(&mut self, transactions: BTreeMap<Txid, Vec<u8>>) {
        match self {
            BlockExecutionContext::V0(v0) => v0.withdrawal_transactions = transactions,
        }
    }

    fn set_block_platform_state(&mut self, state: PlatformState) {
        match self {
            BlockExecutionContext::V0(v0) => v0.block_platform_state = state,
        }
    }

    fn set_proposer_results(&mut self, results: Option<ResponsePrepareProposal>) {
        match self {
            BlockExecutionContext::V0(v0) => v0.proposer_results = results,
        }
    }
}

impl BlockExecutionContextV0MutableGetters for BlockExecutionContext {
    fn block_state_info_mut(&mut self) -> &mut BlockStateInfo {
        match self {
            BlockExecutionContext::V0(v0) => v0.block_state_info_mut(),
        }
    }

    fn epoch_info_mut(&mut self) -> &mut EpochInfo {
        match self {
            BlockExecutionContext::V0(v0) => v0.epoch_info_mut(),
        }
    }

    fn withdrawal_transactions_mut(&mut self) -> &mut BTreeMap<Txid, Vec<u8>> {
        match self {
            BlockExecutionContext::V0(v0) => v0.withdrawal_transactions_mut(),
        }
    }

    fn block_platform_state_mut(&mut self) -> &mut PlatformState {
        match self {
            BlockExecutionContext::V0(v0) => v0.block_platform_state_mut(),
        }
    }

    fn proposer_results_mut(&mut self) -> Option<&mut ResponsePrepareProposal> {
        match self {
            BlockExecutionContext::V0(v0) => v0.proposer_results_mut(),
        }
    }
}

impl BlockExecutionContextV0OwnedGetters for BlockExecutionContext {
    /// Consumes the object and returns the owned `BlockStateInfo`.
    fn block_state_info_owned(self) -> BlockStateInfo {
        match self {
            BlockExecutionContext::V0(v0) => v0.block_state_info,
        }
    }

    /// Consumes the object and returns the owned `EpochInfo`.
    fn epoch_info_owned(self) -> EpochInfo {
        match self {
            BlockExecutionContext::V0(v0) => v0.epoch_info,
        }
    }

    /// Consumes the object and returns the owned `withdrawal_transactions`.
    fn withdrawal_transactions_owned(self) -> BTreeMap<Txid, Vec<u8>> {
        match self {
            BlockExecutionContext::V0(v0) => v0.withdrawal_transactions,
        }
    }

    /// Consumes the object and returns the owned `PlatformState`.
    fn block_platform_state_owned(self) -> PlatformState {
        match self {
            BlockExecutionContext::V0(v0) => v0.block_platform_state,
        }
    }

    /// Consumes the object and returns the owned `ResponsePrepareProposal`.
    fn proposer_results_owned(self) -> Option<ResponsePrepareProposal> {
        match self {
            BlockExecutionContext::V0(v0) => v0.proposer_results,
        }
    }
}
