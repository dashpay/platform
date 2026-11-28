//! The Core RPC client of the shadow.

use crate::rpc::core::CoreHeight;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::consensus::encode::deserialize;
use dpp::dashcore::ephemerealdata::chain_lock::ChainLock;
use dpp::dashcore::{
    Block, BlockHash, Header, InstantLock, ProTxHash, QuorumHash, Transaction, Txid,
};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    AssetUnlockStatusResult, DMNState, ExtendedQuorumListResult, GetChainTipsResult,
    MasternodeListDiff, MnSyncStatus, QuorumInfoResult, QuorumType, SoftforkInfo,
};
use dpp::dashcore_rpc::json::GetRawTransactionResult;
use dpp::dashcore_rpc::Error;
use dpp::prelude::TimestampMillis;
use serde_json::Value;
use std::sync::Mutex;

/// The Core RPC client of the shadow: the primary's own, asked through a reference.
///
/// Every question is put to the primary's client as the primary puts it. A transaction is
/// not sent: the primary sends the withdrawals of a block, and what Core says of a second
/// sending only decides what a node logs.
///
/// An error Core answers the shadow with is noted, with the question it answered, up to
/// the first 64 since the notes were last cleared, which is where the shadow begins to
/// execute a block. What is done with the notes where a divergence is found is in the
/// documentation of the module above this one: the primary put its questions earlier
/// and can have been answered otherwise.
#[derive(Debug)]
pub struct ShadowCore<'a, C> {
    core: &'a C,
    errors: Mutex<Vec<String>>,
}

/// The most errors that are noted before the notes are cleared.
const ERRORS_NOTED: usize = 64;

impl<'a, C> ShadowCore<'a, C> {
    /// The client of a shadow whose primary asks `core`.
    pub(super) fn of(core: &'a C) -> Self {
        Self {
            core,
            errors: Mutex::new(Vec::new()),
        }
    }

    fn errors(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.errors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn noted<T>(&self, question: &'static str, answer: Result<T, Error>) -> Result<T, Error> {
        if let Err(error) = &answer {
            let mut errors = self.errors();
            if errors.len() < ERRORS_NOTED {
                errors.push(format!("{question}: {error}"));
            }
        }
        answer
    }

    /// The errors that are noted.
    pub(super) fn errors_noted(&self) -> Vec<String> {
        self.errors().clone()
    }

    /// Forgets the errors that are noted.
    pub(super) fn clear_errors(&self) {
        self.errors().clear();
    }

    /// Notes an error as Core's, for a test.
    #[cfg(any(test, feature = "mocks"))]
    pub(super) fn note(&self, error: &str) {
        self.errors().push(error.to_string());
    }
}

impl<C: CoreRPCLike> CoreRPCLike for ShadowCore<'_, C> {
    fn get_block_hash(&self, height: CoreHeight) -> Result<BlockHash, Error> {
        self.noted("get_block_hash", self.core.get_block_hash(height))
    }

    fn get_block_header(&self, block_hash: &BlockHash) -> Result<Header, Error> {
        self.noted("get_block_header", self.core.get_block_header(block_hash))
    }

    fn get_block_time_from_height(&self, height: CoreHeight) -> Result<TimestampMillis, Error> {
        self.noted(
            "get_block_time_from_height",
            self.core.get_block_time_from_height(height),
        )
    }

    fn get_best_chain_lock(&self) -> Result<ChainLock, Error> {
        self.noted("get_best_chain_lock", self.core.get_best_chain_lock())
    }

    fn submit_chain_lock(&self, chain_lock: &ChainLock) -> Result<u32, Error> {
        self.noted("submit_chain_lock", self.core.submit_chain_lock(chain_lock))
    }

    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        self.noted("get_transaction", self.core.get_transaction(tx_id))
    }

    fn get_asset_unlock_statuses(
        &self,
        indices: &[u64],
        core_chain_locked_height: u32,
    ) -> Result<Vec<AssetUnlockStatusResult>, Error> {
        self.noted(
            "get_asset_unlock_statuses",
            self.core
                .get_asset_unlock_statuses(indices, core_chain_locked_height),
        )
    }

    fn get_transaction_extended_info(
        &self,
        tx_id: &Txid,
    ) -> Result<GetRawTransactionResult, Error> {
        self.noted(
            "get_transaction_extended_info",
            self.core.get_transaction_extended_info(tx_id),
        )
    }

    fn get_optional_transaction_extended_info(
        &self,
        transaction_id: &Txid,
    ) -> Result<Option<GetRawTransactionResult>, Error> {
        self.noted(
            "get_optional_transaction_extended_info",
            self.core
                .get_optional_transaction_extended_info(transaction_id),
        )
    }

    fn get_fork_info(&self, name: &str) -> Result<Option<SoftforkInfo>, Error> {
        self.noted("get_fork_info", self.core.get_fork_info(name))
    }

    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error> {
        self.noted("get_block", self.core.get_block(block_hash))
    }

    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error> {
        self.noted("get_block_json", self.core.get_block_json(block_hash))
    }

    fn get_chain_tips(&self) -> Result<GetChainTipsResult, Error> {
        self.noted("get_chain_tips", self.core.get_chain_tips())
    }

    fn get_quorum_listextended(
        &self,
        height: Option<CoreHeight>,
    ) -> Result<ExtendedQuorumListResult, Error> {
        self.noted(
            "get_quorum_listextended",
            self.core.get_quorum_listextended(height),
        )
    }

    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        include_secret_key_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error> {
        self.noted(
            "get_quorum_info",
            self.core
                .get_quorum_info(quorum_type, hash, include_secret_key_share),
        )
    }

    fn get_protx_diff_with_masternodes(
        &self,
        base_block: Option<u32>,
        block: u32,
    ) -> Result<MasternodeListDiff, Error> {
        self.noted(
            "get_protx_diff_with_masternodes",
            self.core.get_protx_diff_with_masternodes(base_block, block),
        )
    }

    fn get_masternode_state(
        &self,
        pro_tx_hash: &ProTxHash,
        height: CoreHeight,
    ) -> Result<DMNState, Error> {
        self.noted(
            "get_masternode_state",
            self.core.get_masternode_state(pro_tx_hash, height),
        )
    }

    fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        max_height: Option<u32>,
    ) -> Result<bool, Error> {
        self.noted(
            "verify_instant_lock",
            self.core.verify_instant_lock(instant_lock, max_height),
        )
    }

    fn verify_chain_lock(&self, chain_lock: &ChainLock) -> Result<bool, Error> {
        self.noted("verify_chain_lock", self.core.verify_chain_lock(chain_lock))
    }

    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error> {
        self.noted("masternode_sync_status", self.core.masternode_sync_status())
    }

    /// Sends nothing. The answer is the id of the transaction the bytes hold, and the
    /// hash of no transaction where they hold none.
    fn send_raw_transaction(&self, transaction: &[u8]) -> Result<Txid, Error> {
        Ok(deserialize::<Transaction>(transaction)
            .map(|transaction| transaction.txid())
            .unwrap_or_else(|_| Txid::all_zeros()))
    }

    fn get_credit_pool_balance(&self, height: CoreHeight) -> Result<u64, Error> {
        self.noted(
            "get_credit_pool_balance",
            self.core.get_credit_pool_balance(height),
        )
    }

    fn get_coinbase_payload_version(&self, height: CoreHeight) -> Result<Option<u16>, Error> {
        self.noted(
            "get_coinbase_payload_version",
            self.core.get_coinbase_payload_version(height),
        )
    }
}
