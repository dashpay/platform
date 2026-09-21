use crate::rpc::prefetch::CorePrefetcher;
use dpp::dashcore::ephemerealdata::chain_lock::ChainLock;
use dpp::dashcore::{Amount, Header, InstantLock};
use dpp::dashcore::{Block, BlockHash, QuorumHash, Transaction, Txid};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    AssetUnlockStatusResult, ExtendedQuorumDetails, ExtendedQuorumListResult, GetChainTipsResult,
    MasternodeListDiff, MnSyncStatus, QuorumInfoResult, QuorumType, SoftforkInfo,
};
use dpp::dashcore_rpc::json::GetRawTransactionResult;
use dpp::dashcore_rpc::{Auth, Client, Error, RpcApi};
use dpp::prelude::TimestampMillis;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Information returned by QuorumListExtended
pub type QuorumListExtendedInfo = HashMap<QuorumHash, ExtendedQuorumDetails>;

/// Core height must be of type u32 (Platform heights are u64)
pub type CoreHeight = u32;

/// The credit pool as Core's `getcreditpoolinfo` RPC reports it for one block: the balance
/// after that block, the balance one window (`window_blocks`) earlier and the asset unlock
/// limit Core applies to the next block.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct CreditPoolInfo {
    /// The height the pool is reported at
    pub height: CoreHeight,
    /// The pool balance after that block
    #[serde(with = "dpp::dashcore::amount::serde::as_btc")]
    pub balance: Amount,
    /// The total of asset unlocks Core admits in the next block
    #[serde(rename = "currentlimit", with = "dpp::dashcore::amount::serde::as_btc")]
    pub current_limit: Amount,
    /// The amount unlocked in the blocks of the window
    #[serde(
        rename = "unlockedinwindow",
        with = "dpp::dashcore::amount::serde::as_btc"
    )]
    pub unlocked_in_window: Amount,
    /// The number of blocks in the limit's window
    #[serde(rename = "windowblocks")]
    pub window_blocks: u32,
    /// The height of the block whose balance the window starts from, -1 when the chain is
    /// shorter than the window
    #[serde(rename = "windowstartheight")]
    pub window_start_height: i64,
    /// The pool balance after the window start block, 0 when that block has no credit pool
    #[serde(
        rename = "windowstartbalance",
        with = "dpp::dashcore::amount::serde::as_btc"
    )]
    pub window_start_balance: Amount,
}

/// Core RPC interface
#[cfg_attr(any(feature = "mocks", test), mockall::automock)]
pub trait CoreRPCLike {
    /// Get block hash by height
    fn get_block_hash(&self, height: CoreHeight) -> Result<BlockHash, Error>;

    /// Get block hash by height
    fn get_block_header(&self, block_hash: &BlockHash) -> Result<Header, Error>;

    /// Get block time of a chain locked core height
    fn get_block_time_from_height(&self, height: CoreHeight) -> Result<TimestampMillis, Error>;

    /// Get the best chain lock
    fn get_best_chain_lock(&self) -> Result<ChainLock, Error>;

    /// Submit a chain lock
    fn submit_chain_lock(&self, chain_lock: &ChainLock) -> Result<u32, Error>;

    /// Get transaction
    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error>;

    /// Get asset unlock statuses
    fn get_asset_unlock_statuses(
        &self,
        indices: &[u64],
        core_chain_locked_height: u32,
    ) -> Result<Vec<AssetUnlockStatusResult>, Error>;

    /// Get the credit pool and the asset unlock limit at a height (`getcreditpoolinfo`)
    fn get_credit_pool_info(&self, height: CoreHeight) -> Result<CreditPoolInfo, Error>;

    /// Get transaction
    fn get_transaction_extended_info(&self, tx_id: &Txid)
        -> Result<GetRawTransactionResult, Error>;

    /// Get optional transaction extended info
    /// Returns None if transaction doesn't exists
    fn get_optional_transaction_extended_info(
        &self,
        transaction_id: &Txid,
    ) -> Result<Option<GetRawTransactionResult>, Error> {
        match self.get_transaction_extended_info(transaction_id) {
            Ok(transaction_info) => Ok(Some(transaction_info)),
            // Return None if transaction with specified tx id is not present
            Err(Error::JsonRpc(dpp::dashcore_rpc::jsonrpc::error::Error::Rpc(
                dpp::dashcore_rpc::jsonrpc::error::RpcError {
                    code: CORE_RPC_INVALID_ADDRESS_OR_KEY,
                    ..
                },
            ))) => Ok(None),
            Err(e) => Err(e),
        }
    }

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
    fn verify_chain_lock(&self, chain_lock: &ChainLock) -> Result<bool, Error>;

    /// Returns masternode sync status
    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error>;

    /// Sends raw transaction to the network
    fn send_raw_transaction(&self, transaction: &[u8]) -> Result<Txid, Error>;
}

#[derive(Debug)]
/// Default implementation of Dash Core RPC using DashCoreRPC client
pub struct DefaultCoreRPC {
    inner: Client,
    /// Speculative fetcher for the next core height, on its own connection.
    /// `None` when a second connection could not be opened.
    prefetcher: Option<CorePrefetcher>,
}

// TODO: Create errors for these error codes in dashcore_rpc

/// TX is invalid due to consensus rules
pub const CORE_RPC_TX_CONSENSUS_ERROR: i32 = -26;
/// Tx already broadcasted and included in the chain
pub const CORE_RPC_TX_ALREADY_IN_CHAIN: i32 = -27;
/// Client still warming up
pub const CORE_RPC_ERROR_IN_WARMUP: i32 = -28;
/// Dash is not connected
pub const CORE_RPC_CLIENT_NOT_CONNECTED: i32 = -9;
/// Still downloading initial blocks
pub const CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD: i32 = -10;
/// Parse error
pub const CORE_RPC_PARSE_ERROR: i32 = -32700;
/// Invalid address or key
pub const CORE_RPC_INVALID_ADDRESS_OR_KEY: i32 = -5;
/// Invalid, missing or duplicate parameter
pub const CORE_RPC_INVALID_PARAMETER: i32 = -8;

macro_rules! retry {
    ($action:expr) => {{
        /// Maximum number of retry attempts
        const MAX_RETRIES: u32 = 4;
        /// // Multiplier for Fibonacci sequence
        const FIB_MULTIPLIER: u64 = 1;

        fn fibonacci(n: u32) -> u64 {
            match n {
                0 => 0,
                1 => 1,
                _ => fibonacci(n - 1) + fibonacci(n - 2),
            }
        }

        let mut last_err = None;
        let result = (0..MAX_RETRIES).find_map(|i| {
            match $action {
                Ok(result) => Some(Ok(result)),
                Err(e) => {
                    match e {
                        dpp::dashcore_rpc::Error::JsonRpc(
                            // Retry on transport connection error
                            dpp::dashcore_rpc::jsonrpc::error::Error::Transport(_)
                            | dpp::dashcore_rpc::jsonrpc::error::Error::Rpc(
                                // Retry on Core RPC "not ready" errors
                                dpp::dashcore_rpc::jsonrpc::error::RpcError {
                                    code:
                                        CORE_RPC_ERROR_IN_WARMUP
                                        | CORE_RPC_CLIENT_NOT_CONNECTED
                                        | CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD,
                                    ..
                                },
                            ),
                        ) => {
                            // Delay before next try
                            last_err = Some(e);
                            let delay = fibonacci(i + 2) * FIB_MULTIPLIER;
                            std::thread::sleep(Duration::from_secs(delay));
                            None
                        }
                        _ => Some(Err(e)),
                    }
                }
            }
        });

        result.unwrap_or_else(|| Err(last_err.unwrap()))
    }};
}

impl DefaultCoreRPC {
    /// Create new instance
    pub fn open(url: &str, username: String, password: String) -> Result<Self, Error> {
        let prefetcher = CorePrefetcher::new(url, username.clone(), password.clone());
        if prefetcher.is_none() {
            tracing::warn!(
                "could not open a second Core RPC connection; masternode and quorum updates will be fetched on the critical path"
            );
        }
        Ok(DefaultCoreRPC {
            inner: Client::new(url, Auth::UserPass(username, password))?,
            prefetcher,
        })
    }
}

impl CoreRPCLike for DefaultCoreRPC {
    fn get_block_hash(&self, height: u32) -> Result<BlockHash, Error> {
        retry!(self.inner.get_block_hash(height))
    }

    fn get_block_header(&self, block_hash: &BlockHash) -> Result<Header, Error> {
        retry!(self.inner.get_block_header(block_hash))
    }

    fn get_block_time_from_height(&self, height: CoreHeight) -> Result<TimestampMillis, Error> {
        let block_hash = self.get_block_hash(height)?;
        let block_header = self.get_block_header(&block_hash)?;
        let block_time = block_header.time as u64 * 1000;
        Ok(block_time)
    }

    fn get_best_chain_lock(&self) -> Result<ChainLock, Error> {
        retry!(self.inner.get_best_chain_lock())
    }

    fn submit_chain_lock(&self, chain_lock: &ChainLock) -> Result<u32, Error> {
        retry!(self.inner.submit_chain_lock(chain_lock))
    }

    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        retry!(self.inner.get_raw_transaction(tx_id, None))
    }

    fn get_transaction_extended_info(
        &self,
        tx_id: &Txid,
    ) -> Result<GetRawTransactionResult, Error> {
        retry!(self.inner.get_raw_transaction_info(tx_id, None))
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
        // Block sync walks core heights in order, so the next call is almost
        // always for height + 1. Take the speculative answer when it is for the
        // height we were asked about, and start the next guess either way. The
        // prefetcher declines a guess past the chain lock, so at the tip this is
        // a no-op until Core locks the next block.
        let prefetched = height
            .zip(self.prefetcher.as_ref())
            .and_then(|(height, prefetcher)| prefetcher.take_quorum_list(height));

        let result = match prefetched {
            Some(list) => Ok(list),
            None => retry!(self.inner.get_quorum_listextended_reversed(height)),
        };

        if let (Ok(_), Some(height), Some(prefetcher)) = (&result, height, self.prefetcher.as_ref())
        {
            prefetcher.start_quorum_list(height + 1);
        }

        result
    }

    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        include_secret_key_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error> {
        retry!(self
            .inner
            .get_quorum_info_reversed(quorum_type, hash, include_secret_key_share))
    }

    fn get_protx_diff_with_masternodes(
        &self,
        base_block: Option<u32>,
        block: u32,
    ) -> Result<MasternodeListDiff, Error> {
        let base = base_block.unwrap_or(1);

        // Same reasoning as get_quorum_listextended: the next diff a syncing
        // node asks for is from this block to the one after it.
        let prefetched = self
            .prefetcher
            .as_ref()
            .and_then(|prefetcher| prefetcher.take_protx_diff(base, block));

        let result = match prefetched {
            Some(diff) => Ok(diff),
            None => retry!(self.inner.get_protx_listdiff(base, block)),
        };

        if let (Ok(_), Some(prefetcher)) = (&result, self.prefetcher.as_ref()) {
            prefetcher.start_protx_diff(block, block + 1);
        }

        result
    }

    /// Verify Instant Lock signature
    /// If `max_height` is provided the chain lock will be verified
    /// against quorums available at this height
    fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        max_height: Option<u32>,
    ) -> Result<bool, Error> {
        let request_id = instant_lock.request_id()?.to_string();
        let transaction_id = instant_lock.txid.to_string();
        let signature = hex::encode(instant_lock.signature);

        retry!(self
            .inner
            .get_verifyislock(&request_id, &transaction_id, &signature, max_height))
    }

    /// Verify a chain lock signature
    fn verify_chain_lock(&self, chain_lock: &ChainLock) -> Result<bool, Error> {
        let block_hash = chain_lock.block_hash.to_string();
        let signature = hex::encode(chain_lock.signature);

        retry!(self.inner.get_verifychainlock(
            block_hash.as_str(),
            &signature,
            Some(chain_lock.block_height)
        ))
    }

    /// Returns masternode sync status
    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error> {
        retry!(self.inner.mnsync_status())
    }

    fn send_raw_transaction(&self, transaction: &[u8]) -> Result<Txid, Error> {
        retry!(self.inner.send_raw_transaction(transaction))
    }

    fn get_asset_unlock_statuses(
        &self,
        indices: &[u64],
        core_chain_locked_height: u32,
    ) -> Result<Vec<AssetUnlockStatusResult>, Error> {
        retry!(self
            .inner
            .get_asset_unlock_statuses(indices, Some(core_chain_locked_height)))
    }

    fn get_credit_pool_info(&self, height: CoreHeight) -> Result<CreditPoolInfo, Error> {
        retry!(self.inner.call("getcreditpoolinfo", &[Value::from(height)]))
    }
}

#[cfg(test)]
mod tests {
    use super::CreditPoolInfo;
    use dpp::dashcore::Amount;

    #[test]
    fn credit_pool_info_deserializes_cores_getcreditpoolinfo_response() {
        // A literal `getcreditpoolinfo 2542758` answer from a mainnet node
        let body = r#"{
            "height": 2542758,
            "blockhash": "000000000000001a5c8b4e6c4b7b3b5a6a2f9d3b1e0c7a8f4d2e6b9c1a3f5e7d",
            "balance": 36932.83216583,
            "currentlimit": 4000.00000000,
            "unlockedinwindow": 12.50000000,
            "windowblocks": 576,
            "windowstartheight": 2542182,
            "windowstartbalance": 36685.60589396
        }"#;
        let info: CreditPoolInfo = serde_json::from_str(body).expect("expected to deserialize");
        assert_eq!(info.height, 2542758);
        assert_eq!(info.balance, Amount::from_sat(3_693_283_216_583));
        assert_eq!(info.current_limit, Amount::from_sat(400_000_000_000));
        assert_eq!(info.unlocked_in_window, Amount::from_sat(1_250_000_000));
        assert_eq!(info.window_blocks, 576);
        assert_eq!(info.window_start_height, 2542182);
        assert_eq!(
            info.window_start_balance,
            Amount::from_sat(3_668_560_589_396)
        );

        // A chain shorter than the window
        let body = r#"{"height": 10, "blockhash": "00", "balance": 0.00000000, "currentlimit": 0.00000000,
            "unlockedinwindow": 0.00000000, "windowblocks": 576, "windowstartheight": -1, "windowstartbalance": 0.00000000}"#;
        let info: CreditPoolInfo = serde_json::from_str(body).expect("expected to deserialize");
        assert_eq!(info.window_start_height, -1);
        assert_eq!(info.window_start_balance, Amount::from_sat(0));
    }
}
