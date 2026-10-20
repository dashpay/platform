use crate::rpc::prefetch::CorePrefetcher;
use dpp::dashcore::consensus::encode::deserialize_partial;
use dpp::dashcore::ephemerealdata::chain_lock::ChainLock;
use dpp::dashcore::transaction::special_transaction::coinbase::CoinbasePayload;
use dpp::dashcore::transaction::special_transaction::TransactionPayload;
use dpp::dashcore::{Block, BlockHash, QuorumHash, Transaction, Txid};
use dpp::dashcore::{Header, InstantLock};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    AssetUnlockStatusResult, ExtendedQuorumDetails, ExtendedQuorumListResult, GetChainTipsResult,
    MasternodeListDiff, MnSyncStatus, QuorumInfoResult, QuorumType, SoftforkInfo,
};
use dpp::dashcore_rpc::json::GetRawTransactionResult;
use dpp::dashcore_rpc::jsonrpc::error::{Error as JsonRpcError, RpcError};
use dpp::dashcore_rpc::jsonrpc::simple_http::Error as HttpError;
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
/// The transaction decoder types a payload only when the payload decoder reads exactly the
/// bytes its length prefix frames, and keeps any other payload raw. A raw coinbase payload is
/// read with `deserialize_partial`, so bytes after the fields the payload decoder knows are
/// left unread rather than refused: a field a later Core version appends to the payload, as
/// version 4 appended `merkleRootAssetUnlocks` after the balance, does not stop the balance
/// from being read. The payload is the coinbase's last field and Core frames it exactly, so
/// reading it raw reads the same bytes as reading it in place. Core has only ever appended
/// fields to the payload, and its consensus rules (`CheckCbTx`) refuse versions it does not
/// know, so the balance stays where version 3 put it unless a Core release moves it, which
/// Platform would have to follow anyway.
pub(crate) fn credit_pool_balance_from_coinbase(coinbase: &[u8]) -> Result<u64, String> {
    let (transaction, _) = deserialize_partial::<Transaction>(coinbase)
        .map_err(|e| format!("coinbase cannot be decoded: {e}"))?;
    match transaction.special_transaction_payload {
        None => Ok(0),
        Some(TransactionPayload::CoinbasePayloadType(payload)) => {
            Ok(payload.asset_locked_amount.unwrap_or_default())
        }
        Some(TransactionPayload::RawPayloadType(COINBASE_TRANSACTION_TYPE, bytes)) => {
            let (payload, _) = deserialize_partial::<CoinbasePayload>(&bytes)
                .map_err(|e| format!("coinbase payload cannot be decoded: {e}"))?;
            Ok(payload.asset_locked_amount.unwrap_or_default())
        }
        Some(payload) => Err(format!(
            "coinbase carries a {:?} payload",
            payload.get_type()
        )),
    }
}

/// Reads the version of a block's coinbase payload from the block's serialized coinbase
/// transaction; `None` when it carries no payload. The version is the payload's first field,
/// so a raw payload is read the same way as in [`credit_pool_balance_from_coinbase`].
pub(crate) fn coinbase_payload_version_from_coinbase(
    coinbase: &[u8],
) -> Result<Option<u16>, String> {
    let (transaction, _) = deserialize_partial::<Transaction>(coinbase)
        .map_err(|e| format!("coinbase cannot be decoded: {e}"))?;
    match transaction.special_transaction_payload {
        None => Ok(None),
        Some(TransactionPayload::CoinbasePayloadType(payload)) => Ok(Some(payload.version)),
        Some(TransactionPayload::RawPayloadType(COINBASE_TRANSACTION_TYPE, bytes)) => {
            let (payload, _) = deserialize_partial::<CoinbasePayload>(&bytes)
                .map_err(|e| format!("coinbase payload cannot be decoded: {e}"))?;
            Ok(Some(payload.version))
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

    /// Get the state of the Core deployment `name` at the chain tip, `None` when Core does not
    /// report it
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

    /// Get the version of the coinbase payload of the block at `height`, `None` for a coinbase
    /// without a payload (only the coinbase is transferred). Only ask for a chain locked height:
    /// the answer is then the same on every node.
    fn get_coinbase_payload_version(&self, height: CoreHeight) -> Result<Option<u16>, Error>;
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
/// Method not found
pub const CORE_RPC_METHOD_NOT_FOUND: i32 = -32601;

/// The HTTP status Core refuses a request with, such as a method missing from the RPC user's
/// `rpcwhitelist`
const HTTP_FORBIDDEN: u16 = 403;

/// Whether Core refused the request with HTTP 403 and no JSON-RPC answer
fn is_http_forbidden(error: &Error) -> bool {
    let Error::JsonRpc(JsonRpcError::Transport(transport)) = error else {
        return false;
    };
    matches!(
        transport.downcast_ref::<HttpError>(),
        Some(HttpError::HttpErrorCode(HTTP_FORBIDDEN))
    )
}

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
                        // Core answers HTTP 403 to a request it will never allow, such as a
                        // method missing from the RPC user's rpcwhitelist, so asking again
                        // only delays the error
                        _ if is_http_forbidden(&e) => Some(Err(e)),
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

    /// The serialized coinbase of the block at `height`, `None` for a coinbase without a payload.
    fn coinbase_with_payload(&self, height: CoreHeight) -> Result<Option<Vec<u8>>, Error> {
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
            return Ok(None);
        };
        hex::decode(coinbase).map(Some).map_err(|e| {
            Error::UnexpectedStructure(format!("getspecialtxes answered invalid hex: {e}"))
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
        match retry!(self.inner.get_deployment_info(None)) {
            Ok(deployment_info) => Ok(deployment_info.deployments.get(name).cloned()),
            // Core before v24 has no getdeploymentinfo, and reports the same deployments as
            // getblockchaininfo's softforks. Core v24 reports them there only with
            // -deprecatedrpc=softforks.
            Err(Error::JsonRpc(JsonRpcError::Rpc(RpcError {
                code: CORE_RPC_METHOD_NOT_FOUND,
                ..
            }))) => retry!(self
                .inner
                .get_blockchain_info()
                .map(|blockchain_info| blockchain_info.softforks.get(name).cloned())),
            // Core refuses a method missing from the RPC user's rpcwhitelist with HTTP 403. Core
            // before v24 still reports the deployment in softforks. Core v24 started without
            // -deprecatedrpc=softforks reports none there, so when softforks has no entry for
            // the fork, the refusal is returned rather than `None`.
            Err(e) if is_http_forbidden(&e) => {
                tracing::warn!(
                    "Core refused getdeploymentinfo with HTTP 403; add it to the Core RPC user's rpcwhitelist. Reading the {name} deployment from getblockchaininfo softforks instead"
                );
                let blockchain_info = retry!(self.inner.get_blockchain_info())?;
                match blockchain_info.softforks.get(name) {
                    Some(fork_info) => Ok(Some(fork_info.clone())),
                    None => Err(e),
                }
            }
            Err(e) => Err(e),
        }
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
        let Some(coinbase) = self.coinbase_with_payload(height)? else {
            return Ok(0);
        };
        credit_pool_balance_from_coinbase(&coinbase).map_err(Error::UnexpectedStructure)
    }

    fn get_coinbase_payload_version(&self, height: CoreHeight) -> Result<Option<u16>, Error> {
        let Some(coinbase) = self.coinbase_with_payload(height)? else {
            return Ok(None);
        };
        coinbase_payload_version_from_coinbase(&coinbase).map_err(Error::UnexpectedStructure)
    }
}

#[cfg(test)]
mod tests {
    use super::{coinbase_payload_version_from_coinbase, credit_pool_balance_from_coinbase};
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
            merkle_root_asset_unlocks: None,
        })
    }

    #[test]
    fn should_read_the_balance_of_a_version_3_coinbase() {
        assert_eq!(
            credit_pool_balance_from_coinbase(&coinbase(Some(version_3_payload()))),
            Ok(BALANCE_DUFFS)
        );
    }

    /// The coinbase of `version_3_payload` raised to `version`, with
    /// `after_balance` after its version 3 fields.
    fn raised_coinbase(version: u16, after_balance: &[u8]) -> Vec<u8> {
        let version_3 = coinbase(Some(version_3_payload()));
        // The payload is last: its 1-byte length (175), then the payload itself.
        let payload_start = version_3.len() - 175;
        assert_eq!(version_3[payload_start - 1], 175);
        let mut grown = version_3[..payload_start - 1].to_vec();
        grown.push(u8::try_from(175 + after_balance.len()).expect("1-byte length"));
        grown.extend_from_slice(&version.to_le_bytes());
        grown.extend_from_slice(&version_3[payload_start + 2..]);
        grown.extend_from_slice(after_balance);
        grown
    }

    /// Core v24 blocks carry a version 4 payload, which appends `merkleRootAssetUnlocks`
    /// after the balance.
    #[test]
    fn should_read_the_balance_of_a_version_4_coinbase() {
        assert_eq!(
            credit_pool_balance_from_coinbase(&raised_coinbase(4, &[0xaa; 32])),
            Ok(BALANCE_DUFFS)
        );
    }

    /// Fields a later Core version appends after `merkleRootAssetUnlocks` are
    /// left unread, so the balance is still read.
    #[test]
    fn should_read_the_balance_of_a_coinbase_with_fields_after_version_4() {
        let mut after_balance = vec![0xaa; 32];
        after_balance.extend_from_slice(&[0xbb; 16]);
        assert_eq!(
            credit_pool_balance_from_coinbase(&raised_coinbase(4, &after_balance)),
            Ok(BALANCE_DUFFS)
        );
    }

    /// A payload version after 4, with a field after `merkleRootAssetUnlocks`:
    /// the payload decoder reads a version it does not know with the version 4
    /// layout, so the balance is still read from where version 3 put it.
    #[test]
    fn should_read_the_balance_of_a_coinbase_of_a_later_version() {
        let mut after_balance = vec![0xaa; 32];
        after_balance.extend_from_slice(&[0xbb; 16]);
        assert_eq!(
            credit_pool_balance_from_coinbase(&raised_coinbase(5, &after_balance)),
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
            merkle_root_asset_unlocks: None,
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

    /// The payload version tells whether Core's rules for that block required the version 4
    /// payload, which Core v24 does exactly at the blocks where DEPLOYMENT_V24 and V20 are active.
    #[test]
    fn should_read_the_payload_version_of_a_coinbase() {
        assert_eq!(
            coinbase_payload_version_from_coinbase(&coinbase(Some(version_3_payload()))),
            Ok(Some(3))
        );
        assert_eq!(
            coinbase_payload_version_from_coinbase(&raised_coinbase(4, &[0xaa; 32])),
            Ok(Some(4))
        );
        let mut after_balance = vec![0xaa; 32];
        after_balance.extend_from_slice(&[0xbb; 16]);
        assert_eq!(
            coinbase_payload_version_from_coinbase(&raised_coinbase(5, &after_balance)),
            Ok(Some(5))
        );
        assert_eq!(
            coinbase_payload_version_from_coinbase(&coinbase(None)),
            Ok(None)
        );
        let full = coinbase(Some(version_3_payload()));
        assert!(coinbase_payload_version_from_coinbase(&full[..40]).is_err());
    }

    /// Reading a Core deployment over a real JSON-RPC connection, from a mock Core that serves
    /// answers recorded from Dash Core v24.0.0-rc.3 and v23.1.8.
    mod fork_info {
        use crate::rpc::core::{CoreRPCLike, DefaultCoreRPC};
        use dpp::dashcore_rpc::dashcore_rpc_json::{SoftforkInfo, SoftforkType};
        use dpp::dashcore_rpc::jsonrpc::error::{Error as JsonRpcError, RpcError};
        use dpp::dashcore_rpc::jsonrpc::simple_http::Error as HttpError;
        use dpp::dashcore_rpc::Error;
        use serde_json::{json, Value};
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::{TcpListener, TcpStream};
        use std::sync::{Arc, Mutex};
        use std::thread;

        /// The `getblockchaininfo` result of Dash Core v24.0.0-rc.3 on regtest at height 440: no
        /// `softforks`, which Core v24 reports only with `-deprecatedrpc=softforks`.
        const CORE_V24_GETBLOCKCHAININFO: &str = r#"{
          "chain": "regtest",
          "blocks": 440,
          "headers": 440,
          "bestblockhash": "74b4fb2caeeebbc7cd5dcc50b58532d983f0e5c87f3ac729c6d47f0ace76b97d",
          "difficulty": 4.656542373906925e-10,
          "time": 1791945745,
          "mediantime": 1791945744,
          "verificationprogress": 1,
          "initialblockdownload": false,
          "chainwork": "0000000000000000000000000000000000000000000000000000000000000372",
          "size_on_disk": 98178,
          "pruned": false,
          "warnings": "Make sure to encrypt your wallet and delete all non-encrypted backups after you have verified that the wallet works!"
        }"#;

        /// The `getdeploymentinfo` result of the same Core v24 node.
        const CORE_V24_GETDEPLOYMENTINFO: &str = r#"{
          "hash": "74b4fb2caeeebbc7cd5dcc50b58532d983f0e5c87f3ac729c6d47f0ace76b97d",
          "height": 440,
          "deployments": {
            "bip34": {"type": "buried", "active": true, "height": 1},
            "bip66": {"type": "buried", "active": true, "height": 1},
            "bip65": {"type": "buried", "active": true, "height": 1},
            "bip147": {"type": "buried", "active": true, "height": 0},
            "csv": {"type": "buried", "active": true, "height": 1},
            "dip0001": {"type": "buried", "active": true, "height": 1},
            "dip0003": {"type": "buried", "active": true, "height": 432},
            "dip0008": {"type": "buried", "active": true, "height": 1},
            "dip0020": {"type": "buried", "active": true, "height": 1},
            "dip0024": {"type": "buried", "active": true, "height": 1},
            "realloc": {"type": "buried", "active": true, "height": 1},
            "v19": {"type": "buried", "active": true, "height": 1},
            "v20": {"type": "buried", "active": true, "height": 432},
            "mn_rr": {"type": "buried", "active": true, "height": 432},
            "withdrawals": {"type": "buried", "active": false, "height": 600},
            "v24": {"type": "bip9", "active": false, "bip9": {"start_time": 0, "timeout": 9223372036854775807, "ehf": true, "min_activation_height": 0, "status": "defined", "since": 0, "status_next": "defined"}},
            "evo_shares": {"type": "bip9", "active": false, "bip9": {"start_time": 0, "timeout": 9223372036854775807, "ehf": true, "min_activation_height": 0, "status": "defined", "since": 0, "status_next": "defined"}},
            "testdummy": {"type": "bip9", "height": 432, "active": true, "bip9": {"start_time": 0, "timeout": 9223372036854775807, "ehf": false, "min_activation_height": 0, "status": "active", "since": 432, "status_next": "active"}}
          }
        }"#;

        /// The `getblockchaininfo` result of Dash Core v23.1.8 on regtest at height 440. Core v23
        /// has no `getdeploymentinfo`.
        const CORE_V23_GETBLOCKCHAININFO: &str = r#"{
          "chain": "regtest",
          "blocks": 440,
          "headers": 440,
          "bestblockhash": "609b799530c36b8ad9d40b6c4dc182e9d64679bfe6c9252ed60822bbd8c8e748",
          "difficulty": 4.656542373906925e-10,
          "time": 1791945759,
          "mediantime": 1791945758,
          "verificationprogress": 1,
          "initialblockdownload": false,
          "chainwork": "0000000000000000000000000000000000000000000000000000000000000372",
          "size_on_disk": 98178,
          "pruned": false,
          "softforks": {
            "bip34": {"type": "buried", "active": true, "height": 1},
            "bip66": {"type": "buried", "active": true, "height": 1},
            "bip65": {"type": "buried", "active": true, "height": 1},
            "bip147": {"type": "buried", "active": true, "height": 0},
            "csv": {"type": "buried", "active": true, "height": 1},
            "dip0001": {"type": "buried", "active": true, "height": 1},
            "dip0003": {"type": "buried", "active": true, "height": 432},
            "dip0008": {"type": "buried", "active": true, "height": 1},
            "dip0020": {"type": "buried", "active": true, "height": 1},
            "dip0024": {"type": "buried", "active": true, "height": 1},
            "realloc": {"type": "buried", "active": true, "height": 1},
            "v19": {"type": "buried", "active": true, "height": 1},
            "v20": {"type": "buried", "active": true, "height": 432},
            "mn_rr": {"type": "buried", "active": true, "height": 432},
            "withdrawals": {"type": "buried", "active": false, "height": 600},
            "v24": {"type": "bip9", "bip9": {"status": "defined", "start_time": 0, "timeout": 9223372036854775807, "min_activation_height": 0, "ehf": true, "since": 0}, "active": false},
            "testdummy": {"type": "bip9", "bip9": {"status": "active", "start_time": 0, "timeout": 9223372036854775807, "min_activation_height": 0, "ehf": false, "since": 432}, "height": 432, "active": true}
          },
          "warnings": "Make sure to encrypt your wallet and delete all non-encrypted backups after you have verified that the wallet works!"
        }"#;

        /// How the mock Core answers a method it knows.
        #[derive(Clone, Copy)]
        enum Answer {
            /// HTTP 200 with this JSON-RPC `result`.
            Result(&'static str),
            /// This HTTP status with this JSON-RPC `error`.
            Error(u16, &'static str),
            /// This HTTP status and no body, as Core refuses a method missing from the RPC
            /// user's `rpcwhitelist` with 403.
            Refused(u16),
        }

        /// A Core JSON-RPC endpoint on a local port. A method it is not given an answer for gets
        /// what Core answers a method it does not have: HTTP 404 with error -32601.
        struct MockCore {
            url: String,
            asked: Arc<Mutex<Vec<String>>>,
        }

        impl MockCore {
            /// Answers a method with its first entry in `answers`, and with the method's next entry,
            /// if it has one, from the next request on.
            fn start(answers: Vec<(&'static str, Answer)>) -> Self {
                let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock core");
                let url = format!(
                    "http://{}",
                    listener.local_addr().expect("mock core address")
                );
                let asked = Arc::new(Mutex::new(Vec::new()));
                let recorder = Arc::clone(&asked);
                let answers = Arc::new(Mutex::new(answers));
                thread::spawn(move || {
                    for stream in listener.incoming() {
                        let Ok(stream) = stream else { return };
                        let answers = Arc::clone(&answers);
                        let recorder = Arc::clone(&recorder);
                        thread::spawn(move || serve(stream, &answers, &recorder));
                    }
                });
                MockCore { url, asked }
            }

            fn rpc(&self) -> DefaultCoreRPC {
                DefaultCoreRPC::open(&self.url, "user".to_string(), "password".to_string())
                    .expect("open Core RPC")
            }

            /// The methods asked so far, in order.
            fn asked(&self) -> Vec<String> {
                self.asked.lock().expect("asked lock").clone()
            }
        }

        /// The answer to `method`, moving on to the method's next entry if it has one.
        fn next_answer(
            answers: &Mutex<Vec<(&'static str, Answer)>>,
            method: &str,
        ) -> Option<Answer> {
            let mut answers = answers.lock().expect("answers lock");
            let mut entries = answers
                .iter()
                .enumerate()
                .filter(|(_, (known, _))| *known == method)
                .map(|(index, _)| index);
            match (entries.next(), entries.next()) {
                (Some(first), Some(_)) => Some(answers.remove(first).1),
                (Some(first), None) => Some(answers[first].1),
                (None, _) => None,
            }
        }

        /// Answers the requests of one connection until the client closes it.
        fn serve(
            stream: TcpStream,
            answers: &Mutex<Vec<(&'static str, Answer)>>,
            asked: &Mutex<Vec<String>>,
        ) {
            let mut writer = stream.try_clone().expect("clone mock core stream");
            let mut reader = BufReader::new(stream);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                let mut content_length = 0;
                loop {
                    line.clear();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(length) = line.to_ascii_lowercase().strip_prefix("content-length:")
                    {
                        content_length = length.trim().parse().expect("request content length");
                    }
                }
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).expect("read request body");
                let request: Value = serde_json::from_slice(&body).expect("request is JSON");
                let method = request["method"]
                    .as_str()
                    .expect("request method")
                    .to_string();
                asked.lock().expect("asked lock").push(method.clone());

                let json =
                    |text: &str| serde_json::from_str::<Value>(text).expect("answer is JSON");
                let (status, response) = match next_answer(answers, &method) {
                    Some(Answer::Result(result)) => (
                        200,
                        json!({ "result": json(result), "error": null, "id": request["id"] })
                            .to_string(),
                    ),
                    Some(Answer::Error(status, error)) => (
                        status,
                        json!({ "result": null, "error": json(error), "id": request["id"] })
                            .to_string(),
                    ),
                    Some(Answer::Refused(status)) => (status, String::new()),
                    None => (
                        404,
                        json!({
                            "result": null,
                            "error": { "code": -32601, "message": "Method not found" },
                            "id": request["id"],
                        })
                        .to_string(),
                    ),
                };
                write!(
                    writer,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{response}",
                    response.len()
                )
                .expect("write response");
            }
        }

        fn active_mn_rr() -> SoftforkInfo {
            SoftforkInfo {
                softfork_type: SoftforkType::Buried,
                active: true,
                height: Some(432),
                bip9: None,
            }
        }

        #[test]
        fn should_read_the_fork_from_getdeploymentinfo_on_core_v24() {
            let core = MockCore::start(vec![
                (
                    "getblockchaininfo",
                    Answer::Result(CORE_V24_GETBLOCKCHAININFO),
                ),
                (
                    "getdeploymentinfo",
                    Answer::Result(CORE_V24_GETDEPLOYMENTINFO),
                ),
            ]);

            let fork = core.rpc().get_fork_info("mn_rr");

            assert_eq!(fork.expect("Core v24 reports mn_rr"), Some(active_mn_rr()));
            assert_eq!(core.asked(), ["getdeploymentinfo"]);
        }

        #[test]
        fn should_read_the_fork_from_softforks_on_core_v23_which_has_no_getdeploymentinfo() {
            let core = MockCore::start(vec![(
                "getblockchaininfo",
                Answer::Result(CORE_V23_GETBLOCKCHAININFO),
            )]);

            let fork = core.rpc().get_fork_info("mn_rr");

            assert_eq!(fork.expect("Core v23 reports mn_rr"), Some(active_mn_rr()));
            assert_eq!(core.asked(), ["getdeploymentinfo", "getblockchaininfo"]);
        }

        #[test]
        fn should_not_read_softforks_when_getdeploymentinfo_fails_for_another_reason() {
            let core = MockCore::start(vec![
                (
                    "getblockchaininfo",
                    Answer::Result(CORE_V23_GETBLOCKCHAININFO),
                ),
                (
                    "getdeploymentinfo",
                    Answer::Error(500, r#"{"code": -1, "message": "unexpected failure"}"#),
                ),
            ]);

            let fork = core.rpc().get_fork_info("mn_rr");

            assert!(
                matches!(
                    fork,
                    Err(Error::JsonRpc(JsonRpcError::Rpc(RpcError { code: -1, .. })))
                ),
                "expected the getdeploymentinfo error, got {fork:?}"
            );
            assert_eq!(core.asked(), ["getdeploymentinfo"]);
        }

        #[test]
        fn should_read_the_fork_from_softforks_when_core_v23_refuses_getdeploymentinfo() {
            let core = MockCore::start(vec![
                (
                    "getblockchaininfo",
                    Answer::Result(CORE_V23_GETBLOCKCHAININFO),
                ),
                ("getdeploymentinfo", Answer::Refused(403)),
            ]);

            let fork = core.rpc().get_fork_info("mn_rr");

            assert_eq!(fork.expect("Core v23 reports mn_rr"), Some(active_mn_rr()));
            // A refusal is Core's answer, so getdeploymentinfo is not asked again.
            assert_eq!(core.asked(), ["getdeploymentinfo", "getblockchaininfo"]);
        }

        #[test]
        fn should_fail_with_the_refusal_when_core_v24_refuses_getdeploymentinfo() {
            let core = MockCore::start(vec![
                (
                    "getblockchaininfo",
                    Answer::Result(CORE_V24_GETBLOCKCHAININFO),
                ),
                ("getdeploymentinfo", Answer::Refused(403)),
            ]);

            let fork = core.rpc().get_fork_info("mn_rr");

            // Core v24 reports no softforks, so their absence must not read as a fork that is
            // not known yet.
            assert!(
                matches!(
                    &fork,
                    Err(Error::JsonRpc(JsonRpcError::Transport(transport)))
                        if matches!(
                            transport.downcast_ref::<HttpError>(),
                            Some(HttpError::HttpErrorCode(403))
                        )
                ),
                "expected the getdeploymentinfo refusal, got {fork:?}"
            );
            assert_eq!(core.asked(), ["getdeploymentinfo", "getblockchaininfo"]);
        }

        #[test]
        fn should_ask_getdeploymentinfo_again_while_core_warms_up() {
            let core = MockCore::start(vec![
                (
                    "getdeploymentinfo",
                    Answer::Error(500, r#"{"code": -28, "message": "Loading block index..."}"#),
                ),
                (
                    "getdeploymentinfo",
                    Answer::Result(CORE_V24_GETDEPLOYMENTINFO),
                ),
            ]);

            let fork = core.rpc().get_fork_info("mn_rr");

            assert_eq!(fork.expect("Core v24 reports mn_rr"), Some(active_mn_rr()));
            assert_eq!(core.asked(), ["getdeploymentinfo", "getdeploymentinfo"]);
        }

        #[test]
        fn should_report_no_fork_when_core_v24_does_not_know_it() {
            let core = MockCore::start(vec![(
                "getdeploymentinfo",
                Answer::Result(CORE_V24_GETDEPLOYMENTINFO),
            )]);

            let fork = core.rpc().get_fork_info("unknown_fork");

            assert_eq!(fork.expect("Core v24 answers"), None);
            assert_eq!(core.asked(), ["getdeploymentinfo"]);
        }
    }
}
