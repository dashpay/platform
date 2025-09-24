use crate::cache::{LruResponseCache, make_cache_key};
use crate::error::MapToDapiResult;
use crate::{DAPIResult, DapiError};
use dashcore_rpc::{self, Auth, Client, RpcApi, dashcore, jsonrpc};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::trace;
use zeroize::Zeroizing;

const CORE_RPC_GUARD_PERMITS: usize = 2;

#[derive(Debug, Clone)]
pub struct CoreClient {
    client: Arc<Client>,
    cache: LruResponseCache,
    access_guard: Arc<CoreRpcAccessGuard>,
}

impl CoreClient {
    pub fn new(url: String, user: String, pass: Zeroizing<String>) -> DAPIResult<Self> {
        let client = Client::new(&url, Auth::UserPass(user, pass.to_string()))
            .map_err(|e| DapiError::client(format!("Failed to create Core RPC client: {}", e)))?;
        Ok(Self {
            client: Arc::new(client),
            // Default capacity; immutable responses are small and de-duped by key
            cache: LruResponseCache::with_capacity(1024),
            access_guard: Arc::new(CoreRpcAccessGuard::new(CORE_RPC_GUARD_PERMITS)),
        })
    }

    async fn guarded_blocking_call<F, R, E>(
        &self,
        op: F,
    ) -> Result<Result<R, E>, tokio::task::JoinError>
    where
        F: FnOnce(Arc<Client>) -> Result<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        let permit = self.access_guard.acquire().await;
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            let result = op(client);
            drop(permit);
            result
        })
        .await
    }

    pub async fn get_block_count(&self) -> DAPIResult<u32> {
        trace!("Core RPC: get_block_count");
        let height = self
            .guarded_blocking_call(|client| client.get_block_count())
            .await
            .to_dapi_result()?;

        Ok(height as u32)
    }

    pub async fn get_transaction_info(
        &self,
        txid_hex: &str,
    ) -> DAPIResult<dashcore_rpc::json::GetRawTransactionResult> {
        use std::str::FromStr;
        trace!("Core RPC: get_raw_transaction_info");

        let txid = dashcore_rpc::dashcore::Txid::from_str(txid_hex)
            .map_err(|e| DapiError::InvalidArgument(format!("invalid txid: {}", e)))?;
        let info = self
            .guarded_blocking_call(move |client| client.get_raw_transaction_info(&txid, None))
            .await
            .to_dapi_result()?;
        Ok(info)
    }

    pub async fn send_raw_transaction(&self, raw: &[u8]) -> DAPIResult<String> {
        trace!("Core RPC: send_raw_transaction");
        let raw_vec = raw.to_vec();
        let txid = self
            .guarded_blocking_call(move |client| client.send_raw_transaction(&raw_vec))
            .await
            .to_dapi_result()?;
        Ok(txid.to_string())
    }

    /// Fetches a block hash by its height.
    /// Uses caching to avoid repeated calls for the same height.
    pub async fn get_block_hash(
        &self,
        height: u32,
    ) -> DAPIResult<dashcore_rpc::dashcore::BlockHash> {
        use std::str::FromStr;
        trace!("Core RPC: get_block_hash");

        let key = make_cache_key("get_block_hash", &height);

        let this = self.clone();

        let bytes = self
            .cache
            .get_or_try_insert::<_, _, _, DapiError>(key, move || {
                let this = this.clone();
                let target_height = height;
                async move {
                    let hash = this
                        .guarded_blocking_call(move |client| client.get_block_hash(target_height))
                        .await
                        .to_dapi_result()?;
                    Ok(hash.to_string().into_bytes())
                }
            })
            .await?;

        let s = String::from_utf8(bytes.to_vec())
            .map_err(|e| DapiError::client(format!("invalid utf8 in cached hash: {}", e)))?;
        let hash = dashcore_rpc::dashcore::BlockHash::from_str(&s)
            .map_err(|e| DapiError::client(format!("invalid cached hash: {}", e)))?;
        Ok(hash)
    }

    /// Fetches and decodes a block by its hash.
    /// Wrapper around `get_block_bytes_by_hash` that also decodes the block.
    pub async fn get_block_by_hash(
        &self,
        hash: dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<dashcore::Block> {
        trace!("Core RPC: get_block (bytes)");
        let block_bytes = self.get_block_bytes_by_hash(hash).await?;

        dashcore::consensus::encode::deserialize(&block_bytes).map_err(|e| {
            DapiError::InvalidData(format!("Failed to decode block data from core: {e}"))
        })
    }

    /// Fetches a block's raw bytes by its hash.
    /// Uses caching to avoid repeated calls for the same hash.
    pub async fn get_block_bytes_by_hash(
        &self,
        hash: dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<Vec<u8>> {
        trace!("Core RPC: get_block (bytes)");

        // Use cache-or-populate with immutable key by hash
        let key = make_cache_key("get_block_bytes_by_hash", &hash);

        let this = self.clone();
        let block = self
            .cache
            .get_or_try_insert::<_, _, _, DapiError>(key, move || {
                let this = this.clone();
                let hash = hash;
                async move {
                    // We use get_block_hex to workaround dashcore serialize/deserialize issues
                    // (eg. UnsupportedSegwitFlag(0), UnknownSpecialTransactionType(58385))
                    let block_hex = this
                        .guarded_blocking_call(move |client| client.get_block_hex(&hash))
                        .await
                        .to_dapi_result()?;

                    hex::decode(&block_hex).map_err(|e| {
                        DapiError::InvalidData(format!(
                            "Failed to decode hex block data from core: {e}"
                        ))
                    })
                }
            })
            .await?;

        Ok(block)
    }

    pub async fn get_block_bytes_by_hash_hex(&self, hash_hex: &str) -> DAPIResult<Vec<u8>> {
        use std::str::FromStr;
        if hash_hex.trim().is_empty() {
            return Err(DapiError::InvalidArgument(
                "hash is not specified".to_string(),
            ));
        }

        let hash = dashcore_rpc::dashcore::BlockHash::from_str(hash_hex)
            .map_err(|e| DapiError::InvalidArgument(format!("invalid block hash: {}", e)))?;
        self.get_block_bytes_by_hash(hash).await
    }

    /// Fetch raw transactions (as bytes) for a block by hash without full block deserialization.
    pub async fn get_block_transactions_bytes_by_hash(
        &self,
        hash: dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<Vec<Vec<u8>>> {
        trace!("Core RPC: get_block (verbosity=2) -> tx hex list");

        // Use cache-or-populate with immutable key by hash
        let key = make_cache_key("get_block_transactions_bytes_by_hash", &hash);

        let this = self.clone();
        let transactions = self
            .cache
            .get_or_try_insert::<_, _, _, DapiError>(key, move || {
                let this = this.clone();
                let hash_hex = hash.to_string();
                async move {
                    let value: serde_json::Value = this
                        .guarded_blocking_call(move |client| {
                            let params = [
                                serde_json::Value::String(hash_hex),
                                serde_json::Value::Number(serde_json::Number::from(2)),
                            ];
                            client.call("getblock", &params)
                        })
                        .await
                        .to_dapi_result()?;

                    let obj = value.as_object().ok_or_else(|| {
                        DapiError::invalid_data("getblock verbosity 2 did not return an object")
                    })?;
                    let txs_val = obj.get("tx").ok_or_else(|| {
                        DapiError::invalid_data("getblock verbosity 2 missing 'tx' field")
                    })?;
                    let arr = txs_val
                        .as_array()
                        .ok_or_else(|| DapiError::invalid_data("getblock 'tx' is not an array"))?;

                    let mut out: Vec<Vec<u8>> = Vec::with_capacity(arr.len());
                    for txv in arr.iter() {
                        if let Some(tx_obj) = txv.as_object()
                            && let Some(h) = tx_obj.get("hex").and_then(|v| v.as_str())
                        {
                            let raw = hex::decode(h).map_err(|e| {
                                DapiError::invalid_data(format!("invalid tx hex: {}", e))
                            })?;
                            out.push(raw);
                            continue;
                        }
                        return Err(DapiError::invalid_data(
                            "getblock verbosity 2 'tx' entries missing 'hex'",
                        ));
                    }
                    Ok(out)
                }
            })
            .await?;

        Ok(transactions)
    }

    pub async fn get_mempool_txids(&self) -> DAPIResult<Vec<dashcore_rpc::dashcore::Txid>> {
        trace!("Core RPC: get_raw_mempool");
        self.guarded_blocking_call(|client| client.get_raw_mempool())
            .await
            .to_dapi_result()
    }

    pub async fn get_raw_transaction(
        &self,
        txid: dashcore_rpc::dashcore::Txid,
    ) -> DAPIResult<dashcore::Transaction> {
        trace!("Core RPC: get_raw_transaction");
        self.guarded_blocking_call(move |client| client.get_raw_transaction(&txid, None))
            .await
            .to_dapi_result()
    }

    /// Fetches block header information by its hash.
    /// Uses caching to avoid repeated calls for the same hash.
    pub async fn get_block_header_info(
        &self,
        hash: &dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<dashcore_rpc::json::GetBlockHeaderResult> {
        trace!("Core RPC: get_block_header_info");

        let key = make_cache_key("get_block_header_info", hash);

        let this = self.clone();
        let info = self
            .cache
            .get_or_try_insert::<_, _, _, DapiError>(key, move || {
                let this = this.clone();
                let h = *hash;
                async move {
                    let header = this
                        .guarded_blocking_call(move |client| client.get_block_header_info(&h))
                        .await
                        .to_dapi_result()?;
                    let v = serde_json::to_vec(&header)
                        .map_err(|e| DapiError::client(format!("serialize header: {}", e)))?;
                    let parsed: dashcore_rpc::json::GetBlockHeaderResult =
                        serde_json::from_slice(&v)
                            .map_err(|e| DapiError::client(format!("deserialize header: {}", e)))?;
                    Ok(parsed)
                }
            })
            .await?;

        Ok(info)
    }

    pub async fn get_best_chain_lock(
        &self,
    ) -> DAPIResult<Option<dashcore_rpc::dashcore::ChainLock>> {
        trace!("Core RPC: get_best_chain_lock");
        match self
            .guarded_blocking_call(|client| client.get_best_chain_lock())
            .await
        {
            Ok(Ok(chain_lock)) => Ok(Some(chain_lock)),
            Ok(Err(dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(rpc))))
                if rpc.code == -32603 =>
            {
                // Dash Core returns -32603 when no chain lock is available yet
                Ok(None)
            }
            Ok(Err(e)) => Err(DapiError::from(e)),
            Err(e) => Err(DapiError::from(e)),
        }
    }

    pub async fn mn_list_diff(
        &self,
        base_block: &dashcore_rpc::dashcore::BlockHash,
        block: &dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<serde_json::Value> {
        trace!("Core RPC: protx diff");
        let base_hex = base_block.to_string();
        let block_hex = block.to_string();
        let diff = self
            .guarded_blocking_call(move |client| {
                let params = [
                    serde_json::Value::String("diff".to_string()),
                    serde_json::Value::String(base_hex),
                    serde_json::Value::String(block_hex),
                ];
                client.call("protx", &params)
            })
            .await
            .to_dapi_result()?;
        Ok(diff)
    }

    pub async fn get_blockchain_info(
        &self,
    ) -> DAPIResult<dashcore_rpc::json::GetBlockchainInfoResult> {
        trace!("Core RPC: get_blockchain_info");
        let info = self
            .guarded_blocking_call(|client| client.get_blockchain_info())
            .await
            .to_dapi_result()?;
        Ok(info)
    }

    pub async fn get_network_info(&self) -> DAPIResult<dashcore_rpc::json::GetNetworkInfoResult> {
        trace!("Core RPC: get_network_info");
        let info = self
            .guarded_blocking_call(|client| client.get_network_info())
            .await
            .to_dapi_result()?;
        Ok(info)
    }

    pub async fn estimate_smart_fee_btc_per_kb(&self, blocks: u16) -> DAPIResult<Option<f64>> {
        trace!("Core RPC: estimatesmartfee");
        let result = self
            .guarded_blocking_call(move |client| client.estimate_smart_fee(blocks, None))
            .await
            .to_dapi_result()?;
        Ok(result.fee_rate.map(|a| a.to_dash()))
    }

    pub async fn get_masternode_status(&self) -> DAPIResult<dashcore_rpc::json::MasternodeStatus> {
        trace!("Core RPC: masternode status");
        let st = self
            .guarded_blocking_call(|client| client.get_masternode_status())
            .await
            .to_dapi_result()?;
        Ok(st)
    }

    pub async fn mnsync_status(&self) -> DAPIResult<dashcore_rpc::json::MnSyncStatus> {
        trace!("Core RPC: mnsync status");
        let st = self
            .guarded_blocking_call(|client| client.mnsync_status())
            .await
            .to_dapi_result()?;
        Ok(st)
    }

    pub async fn get_masternode_pos_penalty(
        &self,
        pro_tx_hash_hex: &str,
    ) -> DAPIResult<Option<u32>> {
        use std::collections::HashMap;
        trace!("Core RPC: masternode list (filter)");
        let filter = pro_tx_hash_hex.to_string();
        let map: HashMap<String, dashcore_rpc::json::Masternode> = self
            .guarded_blocking_call(move |client| {
                client.get_masternode_list(Some("json"), Some(&filter))
            })
            .await
            .to_dapi_result()?;

        // Find the entry matching the filter
        if let Some((_k, v)) = map.into_iter().next() {
            return Ok(Some(v.pos_penalty_score));
        }
        Ok(None)
    }
}

#[derive(Debug)]
struct CoreRpcAccessGuard {
    semaphore: Arc<Semaphore>,
}

impl CoreRpcAccessGuard {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    async fn acquire(&self) -> OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Core RPC access guard semaphore not closed")
    }
}
