use dashcore_rpc::dashcore::{Block, BlockHash, QuorumHash, Transaction, Txid};
use dashcore_rpc::dashcore_rpc_json::{
    Bip9SoftforkInfo, ExtendedQuorumDetails, ExtendedQuorumListResult, GetBestChainLockResult,
    GetChainTipsResult, MasternodeListDiff, MnSyncStatus, QuorumInfoResult, QuorumType,
};
use dashcore_rpc::json::GetTransactionResult;
use dashcore_rpc::{Auth, Client, Error, RpcApi};
use mockall::{automock, predicate::*};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tenderdash_abci::proto::types::CoreChainLock;

/// Information returned by QuorumListExtended
pub type QuorumListExtendedInfo = HashMap<QuorumHash, ExtendedQuorumDetails>;

/// Core height must be of type u32 (Platform heights are u64)
pub type CoreHeight = u32;
/// Core RPC interface
#[automock]
pub trait CoreRPCLike {
    /// Get block hash by height
    fn get_block_hash(&self, height: CoreHeight) -> Result<BlockHash, Error>;

    /// Get block hash by height
    fn get_best_chain_lock(&self) -> Result<CoreChainLock, Error>;

    /// Get transaction
    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error>;

    /// Get transaction
    fn get_transaction_extended_info(&self, tx_id: &Txid) -> Result<GetTransactionResult, Error>;

    /// Get block by hash
    fn get_fork_info(&self, name: &str) -> Result<Option<Bip9SoftforkInfo>, Error>;

    /// Get block by hash
    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error>;

    /// Get block by hash in JSON format
    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error>;

    /// Get chain tips
    fn get_chain_tips(&self) -> Result<GetChainTipsResult, Error>;

    /// Get list of quorums at a given height.
    ///
    /// See <https://dashcore.readme.io/v19.0.0/docs/core-api-ref-remote-procedure-calls-evo#quorum-listextended>
    fn get_quorum_listextended(
        &self,
        height: Option<CoreHeight>,
    ) -> Result<ExtendedQuorumListResult, Error>;

    /// Get quorum information.
    ///
    /// See <https://dashcore.readme.io/v19.0.0/docs/core-api-ref-remote-procedure-calls-evo#quorum-info>
    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        include_secret_key_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error>;

    /// Get the difference in masternode list, return masternodes as diff elements
    fn get_protx_diff_with_masternodes(
        &self,
        base_block: Option<u32>,
        block: u32,
    ) -> Result<MasternodeListDiff, Error>;

    // /// Get the detailed information about a deterministic masternode
    // fn get_protx_info(&self, pro_tx_hash: &ProTxHash) -> Result<ProTxInfo, Error>;

    /// Returns masternode sync status
    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error>;
}

/// Default implementation of Dash Core RPC using DashCoreRPC client
pub struct DefaultCoreRPC {
    inner: Client,
}

macro_rules! retry {
    ($action:expr) => {{
        const MAX_RETRIES: u32 = 4; // Maximum number of retry attempts
        const FIB_MULTIPLIER: u64 = 1; // Multiplier for Fibonacci sequence

        fn fibonacci(n: u32) -> u64 {
            match n {
                0 => 0,
                1 => 1,
                _ => fibonacci(n - 1) + fibonacci(n - 2),
            }
        }

        let mut last_err = None;
        for i in 0..MAX_RETRIES {
            match $action {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_err = Some(e);
                    let delay = fibonacci(i + 2) * FIB_MULTIPLIER;
                    std::thread::sleep(Duration::from_secs(delay));
                }
            }
        }
        Err(last_err.unwrap()) // Return the last error if all attempts fail
    }};
}

impl DefaultCoreRPC {
    /// Create new instance
    pub fn open(url: &str, username: String, password: String) -> Result<Self, Error> {
        Ok(DefaultCoreRPC {
            inner: Client::new(url, Auth::UserPass(username, password))?,
        })
    }
}

impl CoreRPCLike for DefaultCoreRPC {
    fn get_block_hash(&self, height: u32) -> Result<BlockHash, Error> {
        retry!(self.inner.get_block_hash(height))
    }

    fn get_best_chain_lock(&self) -> Result<CoreChainLock, Error> {
        //no need to retry on this one
        let GetBestChainLockResult {
            blockhash,
            height,
            signature,
            known_block: _,
        } = self.inner.get_best_chain_lock()?;
        Ok(CoreChainLock {
            core_block_height: height,
            core_block_hash: blockhash.to_vec(),
            signature,
        })
    }

    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        retry!(self.inner.get_raw_transaction(tx_id, None))
    }

    fn get_transaction_extended_info(&self, tx_id: &Txid) -> Result<GetTransactionResult, Error> {
        retry!(self.inner.get_transaction(tx_id, None))
    }

    fn get_fork_info(&self, name: &str) -> Result<Option<Bip9SoftforkInfo>, Error> {
        retry!(self
            .inner
            .get_blockchain_info()
            .map(|blockchain_info| blockchain_info.bip9_softforks.get(name).cloned()))
    }

    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error> {
        retry!(self.inner.get_block(block_hash))
    }

    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error> {
        retry!(self.inner.get_block_json(block_hash))
    }

    fn get_chain_tips(&self) -> Result<GetChainTipsResult, Error> {
        retry!(self.inner.get_chain_tips())
    }

    fn get_quorum_listextended(
        &self,
        height: Option<CoreHeight>,
    ) -> Result<ExtendedQuorumListResult, Error> {
        retry!(self.inner.get_quorum_listextended(height))
    }

    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        include_secret_key_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error> {
        retry!(self
            .inner
            .get_quorum_info(quorum_type, hash, include_secret_key_share))
    }

    fn get_protx_diff_with_masternodes(
        &self,
        base_block: Option<u32>,
        block: u32,
    ) -> Result<MasternodeListDiff, Error> {
        tracing::debug!(
            method = "get_protx_diff_with_masternodes",
            "base block {:?} block {}",
            base_block,
            block
        );
        retry!(self
            .inner
            .get_protx_listdiff(base_block.unwrap_or(1), block))
    }

    /// Returns masternode sync status
    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error> {
        retry!(self.inner.mnsync_status())
    }
}
