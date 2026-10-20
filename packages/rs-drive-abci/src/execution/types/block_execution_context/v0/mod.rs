use crate::execution::types::block_state_info::BlockStateInfo;
use std::collections::BTreeMap;

use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use tenderdash_abci::proto::abci::ResponsePrepareProposal;

/// What our own `PrepareProposal` left behind for a later `ProcessProposal` to answer from.
///
/// Tenderdash computes the block hash only once `PrepareProposal` has returned, so the context
/// left behind carries no block hash and `ProcessProposal` has to recognise the prepared block
/// by the inputs it executed instead. Most of those are kept elsewhere already — height, round,
/// block time, proposer and core chain locked height in the block state info, the transactions
/// and the core chain lock update in the response — and the two below are the ones nothing else
/// retains.
#[derive(Debug, Clone)]
pub struct ProposerResults {
    /// The response `PrepareProposal` returned, replayed verbatim when the block coming back
    /// through `ProcessProposal` is the one that was prepared.
    pub response: ResponsePrepareProposal,
    /// The app version this node voted for in the block it prepared.
    /// `update_validator_proposed_app_version` writes it to state, so it moves the app hash.
    pub proposed_app_version: u64,
    /// The quorum the prepared block's unsigned withdrawal transactions were built against,
    /// and which is asked to sign them on extend vote.
    pub validator_set_quorum_hash: [u8; 32],
}

/// V0 of the Block execution context
#[derive(Debug, Clone)]
pub struct BlockExecutionContextV0 {
    /// Block info
    pub block_state_info: BlockStateInfo,
    /// Epoch info
    pub epoch_info: EpochInfo,
    /// Unsigned withdrawal transactions to be available for extend and verify votes handlers
    pub unsigned_withdrawal_transactions: UnsignedWithdrawalTxs,
    /// Recent address balance changes
    pub block_address_balance_changes: BTreeMap<PlatformAddress, Credits>,
    /// Block state
    pub block_platform_state: PlatformState,
    /// The prepare proposal results if proposed by us
    pub proposer_results: Option<ProposerResults>,
}
/// A trait defining getter methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0Getters {
    /// Returns the block_state_info field.
    fn block_state_info(&self) -> &BlockStateInfo;

    /// Returns a reference of the epoch_info field.
    fn epoch_info(&self) -> &EpochInfo;

    /// Returns a reference of the withdrawal_transactions field.
    fn unsigned_withdrawal_transactions(&self) -> &UnsignedWithdrawalTxs;

    /// Returns a reference of the block_platform_state field.
    fn block_platform_state(&self) -> &PlatformState;

    /// Returns a reference of the proposer_results field.
    fn proposer_results(&self) -> Option<&ProposerResults>;
}

/// A trait defining setter methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0Setters {
    /// Sets the block_state_info field.
    fn set_block_state_info(&mut self, info: BlockStateInfo);

    /// Sets the epoch_info field.
    fn set_epoch_info(&mut self, info: EpochInfo);

    /// Sets the withdrawal_transactions field.
    fn set_unsigned_withdrawal_transactions(&mut self, transactions: UnsignedWithdrawalTxs);

    /// Sets the block_platform_state field.
    fn set_block_platform_state(&mut self, state: PlatformState);

    /// Sets the proposer_results field.
    fn set_proposer_results(&mut self, results: Option<ProposerResults>);
}

/// A trait defining methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0MutableGetters {
    /// Returns a mutable reference to the block_state_info field.
    fn block_state_info_mut(&mut self) -> &mut BlockStateInfo;

    /// Returns a mutable reference to the epoch_info field.
    fn epoch_info_mut(&mut self) -> &mut EpochInfo;

    /// Returns a mutable reference to the block_platform_state field.
    fn block_platform_state_mut(&mut self) -> &mut PlatformState;

    /// Returns a mutable reference to the proposer_results field.
    fn proposer_results_mut(&mut self) -> Option<&mut ProposerResults>;

    /// Returns a mut reference of the withdrawal_transactions field.
    fn unsigned_withdrawal_transactions_mut(&mut self) -> &mut UnsignedWithdrawalTxs;
}

/// A trait defining methods for interacting with a BlockExecutionContextV0.
pub trait BlockExecutionContextV0OwnedGetters {
    /// Consumes the BlockExecutionContextV0 and returns the block_state_info field.
    fn block_state_info_owned(self) -> BlockStateInfo;

    /// Consumes the BlockExecutionContextV0 and returns the epoch_info field.
    fn epoch_info_owned(self) -> EpochInfo;

    /// Consumes the BlockExecutionContextV0 and returns the block_platform_state field.
    fn block_platform_state_owned(self) -> PlatformState;

    /// Consumes the BlockExecutionContextV0 and returns the proposer_results field.
    fn proposer_results_owned(self) -> Option<ProposerResults>;
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

    /// Returns a reference to the unsigned withdrawal transactions
    fn unsigned_withdrawal_transactions(&self) -> &UnsignedWithdrawalTxs {
        &self.unsigned_withdrawal_transactions
    }

    /// Returns a reference to the block_platform_state field.
    fn block_platform_state(&self) -> &PlatformState {
        &self.block_platform_state
    }

    /// Returns a reference to the proposer_results field.
    fn proposer_results(&self) -> Option<&ProposerResults> {
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
    /// Sets the withdrawal_transactions field.
    fn set_unsigned_withdrawal_transactions(&mut self, transactions: UnsignedWithdrawalTxs) {
        self.unsigned_withdrawal_transactions = transactions;
    }
    /// Sets the block_platform_state field.
    fn set_block_platform_state(&mut self, state: PlatformState) {
        self.block_platform_state = state;
    }
    /// Sets the proposer_results field.
    fn set_proposer_results(&mut self, results: Option<ProposerResults>) {
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

    /// Returns a mutable reference to the block_platform_state field.
    fn block_platform_state_mut(&mut self) -> &mut PlatformState {
        &mut self.block_platform_state
    }

    /// Returns a mutable reference to the proposer_results field.
    fn proposer_results_mut(&mut self) -> Option<&mut ProposerResults> {
        self.proposer_results.as_mut()
    }

    fn unsigned_withdrawal_transactions_mut(&mut self) -> &mut UnsignedWithdrawalTxs {
        &mut self.unsigned_withdrawal_transactions
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

    /// Consumes the object and returns the owned `PlatformState`.
    fn block_platform_state_owned(self) -> PlatformState {
        self.block_platform_state
    }

    /// Consumes the object and returns the owned `ProposerResults`.
    fn proposer_results_owned(self) -> Option<ProposerResults> {
        self.proposer_results
    }
}
