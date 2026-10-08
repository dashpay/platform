use crate::rpc::prefetch::CorePrefetcher;
use dpp::dashcore::consensus::encode::deserialize_partial;
use dpp::dashcore::ephemerealdata::chain_lock::ChainLock;
use dpp::dashcore::transaction::special_transaction::TransactionPayload;
use dpp::dashcore::{Block, BlockHash, QuorumHash, Transaction, Txid};
use dpp::dashcore::{Header, InstantLock};
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

/// The special transaction type of a coinbase (`TRANSACTION_COINBASE` in Dash Core).
const COINBASE_TRANSACTION_TYPE: u16 = 5;

/// Reads Core's credit pool balance after a block, in duffs, from the block's serialized
/// coinbase transaction; `0` when it carries no payload or its payload predates version 3,
/// before the credit pool existed, which is how Core's own unlock limit reads such a block.
///
/// Decoded with `deserialize_partial`: the pinned payload decoder reads the fields of version 3
/// for every later version and stops after the balance, while the version 4 payload Core v24
/// requires appends `merkleRootAssetUnlocks` after it. A strict `deserialize`, and so a whole
/// `Block` decode, refuses those unread bytes. Core has only ever appended fields to the
/// payload, and its consensus rules (`CheckCbTx`) refuse versions it does not know, so the
/// balance stays where version 3 put it unless a Core release moves it, which Platform would
/// have to follow anyway.
pub(crate) fn credit_pool_balance_from_coinbase(coinbase: &[u8]) -> Result<u64, String> {
    let (transaction, _) = deserialize_partial::<Transaction>(coinbase)
        .map_err(|e| format!("coinbase cannot be decoded: {e}"))?;
    match transaction.special_transaction_payload {
        None => Ok(0),
        Some(TransactionPayload::CoinbasePayloadType(payload)) => {
            Ok(payload.asset_locked_amount.unwrap_or_default())
        }
        Some(payload) => Err(format!(
            "coinbase carries a {:?} payload",
            payload.get_type()
        )),
    }
}

/// Core height must be of type u32 (Platform heights are u64)
pub type CoreHeight = u32;
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

    /// Get Core's credit pool balance after the block at `height`, in duffs, read from the
    /// block's coinbase (only the coinbase is transferred). Only ask for a chain locked height:
    /// the answer is then the same on every node.
    fn get_credit_pool_balance(&self, height: CoreHeight) -> Result<u64, Error>;
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

    fn get_credit_pool_balance(&self, height: CoreHeight) -> Result<u64, Error> {
        let block_hash = self.get_block_hash(height)?;
        // Only the coinbase, as raw hex: the special transactions of type 5, the first one,
        // verbosity 1. An empty answer is a coinbase without a payload, before DIP3.
        let args = [
            Value::String(block_hash.to_string()),
            Value::from(COINBASE_TRANSACTION_TYPE),
            Value::from(1),
            Value::from(0),
            Value::from(1),
        ];
        let coinbases = retry!(self.inner.call::<Vec<String>>("getspecialtxes", &args))?;
        let Some(coinbase) = coinbases.first() else {
            return Ok(0);
        };
        let coinbase = hex::decode(coinbase).map_err(|e| {
            Error::UnexpectedStructure(format!("getspecialtxes answered invalid hex: {e}"))
        })?;
        credit_pool_balance_from_coinbase(&coinbase).map_err(Error::UnexpectedStructure)
    }
}

#[cfg(test)]
mod tests {
    use super::credit_pool_balance_from_coinbase;
    use dpp::dashcore::bls_sig_utils::BLSSignature;
    use dpp::dashcore::consensus::serialize;
    use dpp::dashcore::hash_types::{MerkleRootMasternodeList, MerkleRootQuorums};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::transaction::special_transaction::coinbase::CoinbasePayload;
    use dpp::dashcore::transaction::special_transaction::TransactionPayload;
    use dpp::dashcore::{OutPoint, ScriptBuf, Transaction, TxIn, TxOut};

    const BALANCE_DUFFS: u64 = 3_700_000_000_000;

    /// A coinbase as the pinned rust-dashcore encodes it.
    fn coinbase(payload: Option<TransactionPayload>) -> Vec<u8> {
        serialize(&Transaction {
            version: 3,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: ScriptBuf::from(vec![0x51, 0x51]),
                sequence: u32::MAX,
                witness: Default::default(),
            }],
            output: vec![TxOut {
                value: 5_000_000,
                script_pubkey: ScriptBuf::from(vec![0x76; 25]),
            }],
            special_transaction_payload: payload,
        })
    }

    fn version_3_payload() -> TransactionPayload {
        TransactionPayload::CoinbasePayloadType(CoinbasePayload {
            version: 3,
            height: 1_000,
            merkle_root_masternode_list: MerkleRootMasternodeList::from_byte_array([1; 32]),
            merkle_root_quorums: MerkleRootQuorums::from_byte_array([2; 32]),
            best_cl_height: Some(30),
            best_cl_signature: Some(BLSSignature::from([3; 96])),
            asset_locked_amount: Some(BALANCE_DUFFS),
        })
    }

    #[test]
    fn should_read_the_balance_of_a_version_3_coinbase() {
        assert_eq!(
            credit_pool_balance_from_coinbase(&coinbase(Some(version_3_payload()))),
            Ok(BALANCE_DUFFS)
        );
    }

    /// Core v24 blocks carry a version 4 payload, which appends `merkleRootAssetUnlocks`
    /// after the balance; the pinned transaction decoder cannot read it.
    #[test]
    fn should_read_the_balance_of_a_version_4_coinbase() {
        let version_3 = coinbase(Some(version_3_payload()));
        // The payload is last: its 1-byte length (175), then the payload itself.
        let payload_start = version_3.len() - 175;
        assert_eq!(version_3[payload_start - 1], 175);
        let mut version_4 = version_3[..payload_start - 1].to_vec();
        version_4.push(175 + 32);
        version_4.extend_from_slice(&4u16.to_le_bytes());
        version_4.extend_from_slice(&version_3[payload_start + 2..]);
        version_4.extend_from_slice(&[0xaa; 32]);

        assert_eq!(
            credit_pool_balance_from_coinbase(&version_4),
            Ok(BALANCE_DUFFS)
        );
    }

    #[test]
    fn should_read_no_balance_from_a_coinbase_before_the_credit_pool() {
        let version_2 = TransactionPayload::CoinbasePayloadType(CoinbasePayload {
            version: 2,
            height: 1_000,
            merkle_root_masternode_list: MerkleRootMasternodeList::from_byte_array([1; 32]),
            merkle_root_quorums: MerkleRootQuorums::from_byte_array([2; 32]),
            best_cl_height: None,
            best_cl_signature: None,
            asset_locked_amount: None,
        });
        assert_eq!(
            credit_pool_balance_from_coinbase(&coinbase(Some(version_2))),
            Ok(0)
        );
        assert_eq!(credit_pool_balance_from_coinbase(&coinbase(None)), Ok(0));
    }

    #[test]
    fn should_fail_on_a_truncated_coinbase() {
        let full = coinbase(Some(version_3_payload()));
        // Cut inside the payload, before the balance.
        assert!(credit_pool_balance_from_coinbase(&full[..full.len() - 60]).is_err());
        assert!(credit_pool_balance_from_coinbase(&full[..40]).is_err());
    }
}
