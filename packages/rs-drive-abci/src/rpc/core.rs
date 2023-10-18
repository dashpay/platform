use dashcore_rpc::dashcore::ephemerealdata::chain_lock::ChainLock;
use dashcore_rpc::dashcore::{Block, BlockHash, QuorumHash, Transaction, Txid};
use dashcore_rpc::dashcore_rpc_json::{
    ExtendedQuorumDetails, ExtendedQuorumListResult, GetBestChainLockResult, GetChainTipsResult,
    GetTransactionLockedResult, MasternodeListDiff, MnSyncStatus, QuorumInfoResult, QuorumType,
    SoftforkInfo,
};
use dashcore_rpc::json::GetTransactionResult;
use dashcore_rpc::{Auth, Client, Error, RpcApi};
use dpp::dashcore::{hashes::Hash, InstantLock};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tenderdash_abci::proto::types::CoreChainLock;

/// Information returned by QuorumListExtended
pub type QuorumListExtendedInfo = HashMap<QuorumHash, ExtendedQuorumDetails>;

/// Core height must be of type u32 (Platform heights are u64)
pub type CoreHeight = u32;
/// Core RPC interface
#[cfg_attr(any(feature = "mocks", test), mockall::automock)]
pub trait CoreRPCLike {
    /// Get block hash by height
    fn get_block_hash(&self, height: CoreHeight) -> Result<BlockHash, Error>;

    /// Get block hash by height
    fn get_best_chain_lock(&self) -> Result<CoreChainLock, Error>;

    /// Get transaction
    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error>;

    /// Get transaction finalization status
    fn get_transactions_are_chain_locked(
        &self,
        tx_ids: Vec<Txid>,
    ) -> Result<Vec<GetTransactionLockedResult>, Error>;

    /// Get transaction
    fn get_transaction_extended_info(&self, tx_id: &Txid) -> Result<GetTransactionResult, Error>;

    /// Get block by hash
    fn get_fork_info(&self, name: &str) -> Result<Option<SoftforkInfo>, Error>;

    /// Get block by hash
    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error>;

    /// Get block by hash in JSON format
    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error>;

    /// Get chain tips
    fn get_chain_tips(&self) -> Result<GetChainTipsResult, Error>;

    /// Get list of quorums by type at a given height.
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

    /// Verify Instant Lock signature
    /// If `max_height` is provided the chain lock will be verified
    /// against quorums available at this height
    fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        max_height: Option<u32>,
    ) -> Result<bool, Error>;

    /// Verify a chain lock signature
    /// If `max_height` is provided the chain lock will be verified
    /// against quorums available at this height
    fn verify_chain_lock(
        &self,
        chain_lock: &ChainLock,
        max_height: Option<u32>,
    ) -> Result<bool, Error>;

    /// Returns masternode sync status
    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error>;
}

#[derive(Debug)]
/// Default implementation of Dash Core RPC using DashCoreRPC client
pub struct DefaultCoreRPC {
    inner: Client,
}

macro_rules! retry {
    ($action:expr) => {{
        /// Maximum number of retry attempts
        const MAX_RETRIES: u32 = 4;
        /// // Multiplier for Fibonacci sequence
        const FIB_MULTIPLIER: u64 = 1;
        /// Client still warming up
        const CORE_RPC_ERROR_IN_WARMUP: i32 = -28;
        /// Dash is not connected
        const CORE_RPC_CLIENT_NOT_CONNECTED: i32 = -9;
        /// Still downloading initial blocks
        const CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD: i32 = -10;

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
                    match e {
                        dashcore_rpc::Error::JsonRpc(
                            // Retry on transport connection error
                            dashcore_rpc::jsonrpc::error::Error::Transport(_)
                            | dashcore_rpc::jsonrpc::error::Error::Rpc(
                                // Retry on Core RPC "not ready" errors
                                dashcore_rpc::jsonrpc::error::RpcError {
                                    code:
                                        CORE_RPC_ERROR_IN_WARMUP
                                        | CORE_RPC_CLIENT_NOT_CONNECTED
                                        | CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD,
                                    ..
                                },
                            ),
                        ) => {
                            last_err = Some(e);
                            let delay = fibonacci(i + 2) * FIB_MULTIPLIER;
                            std::thread::sleep(Duration::from_secs(delay));
                        }
                        _ => return Err(e),
                    };
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
            core_block_hash: blockhash.to_byte_array().to_vec(),
            signature,
        })
    }

    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        retry!(self.inner.get_raw_transaction(tx_id, None))
    }

    fn get_transactions_are_chain_locked(
        &self,
        tx_ids: Vec<Txid>,
    ) -> Result<Vec<GetTransactionLockedResult>, Error> {
        retry!(self.inner.get_transaction_are_locked(&tx_ids))
    }

    fn get_transaction_extended_info(&self, tx_id: &Txid) -> Result<GetTransactionResult, Error> {
        retry!(self.inner.get_transaction(tx_id, None))
    }

    fn get_fork_info(&self, name: &str) -> Result<Option<SoftforkInfo>, Error> {
        retry!(self
            .inner
            .get_blockchain_info()
            .map(|blockchain_info| blockchain_info.softforks.get(name).cloned()))
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

    /// Verify Instant Lock signature
    /// If `max_height` is provided the chain lock will be verified
    /// against quorums available at this height
    fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        max_height: Option<u32>,
    ) -> Result<bool, Error> {
        tracing::debug!(
            method = "verify_instant_lock",
            "instant_lock {:?} max_height {:?}",
            instant_lock,
            max_height
        );

        let request_id = hex::encode(instant_lock.request_id()?);

        retry!(self.inner.get_verifyislock(
            request_id.as_str(),
            &instant_lock.txid.to_hex(),
            hex::encode(instant_lock.signature).as_str(),
            max_height,
        ))
    }

    /// Verify a chain lock signature
    /// If `max_height` is provided the chain lock will be verified
    /// against quorums available at this height
    fn verify_chain_lock(
        &self,
        chain_lock: &ChainLock,
        max_height: Option<u32>,
    ) -> Result<bool, Error> {
        tracing::debug!(
            method = "verify_chain_lock",
            "chain lock {:?} max height {:?}",
            chain_lock,
            max_height,
        );

        retry!(self.inner.get_verifychainlock(
            hex::encode(chain_lock.block_hash).as_str(),
            hex::encode(chain_lock.signature).as_str(),
            max_height
        ))
    }

    /// Returns masternode sync status
    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error> {
        retry!(self.inner.mnsync_status())
    }
}
