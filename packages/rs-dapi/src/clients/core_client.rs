use crate::cache::{LruResponseCache, make_cache_key};
use crate::clients::REQUEST_TIMEOUT;
use crate::error::MapToDapiResult;
use crate::{DAPIResult, DapiError};
use dashcore_rpc::{self, Auth, Client, RpcApi, dashcore, jsonrpc};
use std::any::type_name;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Instant, timeout_at};

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
    /// Create a Core RPC client with caching and concurrency guards.
    /// Wraps the dashcore RPC client and response cache.
    pub fn new(
        url: String,
        user: String,
        pass: Zeroizing<String>,
        cache_capacity_bytes: u64,
    ) -> DAPIResult<Self> {
        let client = Client::new(&url, Auth::UserPass(user, pass.to_string()))
            .map_err(|e| DapiError::client(format!("Failed to create Core RPC client: {}", e)))?;
        Ok(Self {
            client: Arc::new(client),
            cache: LruResponseCache::with_capacity("core_client", cache_capacity_bytes),
            access_guard: Arc::new(CoreRpcAccessGuard::new(CORE_RPC_GUARD_PERMITS)),
        })
    }

    /// Execute a blocking Core RPC call inside a limited concurrency pool.
    ///
    /// The permit moves into the blocking task, so it is released only when the
    /// call returns. A blocking call cannot be stopped once started: a caller
    /// that is cancelled or times out stops waiting, but the call keeps its permit
    /// until Core answers (the transport's socket timeouts bound how long that
    /// takes), so at most `CORE_RPC_GUARD_PERMITS` calls ever run at once.
    async fn guarded_blocking_call<F, R, E>(&self, op: F) -> Result<Result<R, E>, DapiError>
    where
        F: FnOnce(Arc<Client>) -> Result<R, E> + Send + 'static,
        R: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        self.guarded_blocking_call_until(Instant::now() + REQUEST_TIMEOUT, op)
            .await
    }

    /// [`Self::guarded_blocking_call`] with one `deadline` covering both the
    /// permit wait and the call.
    async fn guarded_blocking_call_until<F, R, E>(
        &self,
        deadline: Instant,
        op: F,
    ) -> Result<Result<R, E>, DapiError>
    where
        F: FnOnce(Arc<Client>) -> Result<R, E> + Send + 'static,
        R: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let timed_out = || {
            DapiError::timeout(format!(
                "Core RPC call of type {} did not complete",
                type_name::<F>()
            ))
        };

        let permit = timeout_at(deadline, self.access_guard.acquire())
            .await
            .map_err(|_| timed_out())?;
        let client = self.client.clone();
        let call = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            op(client)
        });

        timeout_at(deadline, call)
            .await
            .map_err(|_| timed_out())?
            .map_err(DapiError::TaskJoin)
    }

    /// Retrieve the current block count from Dash Core as a `u32`.
    pub async fn get_block_count(&self) -> DAPIResult<u32> {
        trace!("Core RPC: get_block_count");
        let height = self
            .guarded_blocking_call(|client| client.get_block_count())
            .await??;

        Ok(height)
    }

    /// Fetch verbose transaction metadata by txid hex string.
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
            .await??;
        Ok(info)
    }

    /// Broadcast a raw transaction byte slice and return its txid hex string.
    pub async fn send_raw_transaction(&self, raw: &[u8]) -> DAPIResult<String> {
        trace!("Core RPC: send_raw_transaction");
        let raw_vec = raw.to_vec();
        let txid = self
            .guarded_blocking_call(move |client| client.send_raw_transaction(&raw_vec))
            .await??;
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
                        .await??;
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
                    // Fetch the hex rather than a decoded block, so clients get the bytes
                    // exactly as Core serialized them, even in a format dashcore cannot decode.
                    let block_hex = this
                        .guarded_blocking_call(move |client| client.get_block_hex(&hash))
                        .await??;

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

    /// Retrieve serialized block header bytes for the given block hash.
    pub async fn get_block_header_bytes_by_hash(
        &self,
        hash: dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<Vec<u8>> {
        trace!("Core RPC: get_block_header");
        let header = self
            .guarded_blocking_call(move |client| client.get_block_header(&hash))
            .await??;

        let bytes = dashcore::consensus::encode::serialize(&header);
        Ok(bytes)
    }

    /// Convenience helper decoding a hash hex string before fetching block bytes.
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
                        .await??;

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

    /// List txids currently present in the Core mempool.
    pub async fn get_mempool_txids(&self) -> DAPIResult<Vec<dashcore_rpc::dashcore::Txid>> {
        trace!("Core RPC: get_raw_mempool");
        self.guarded_blocking_call(|client| client.get_raw_mempool())
            .await?
            .to_dapi_result()
    }

    /// Retrieve a raw transaction by txid, decoding it into a `Transaction`.
    pub async fn get_raw_transaction(
        &self,
        txid: dashcore_rpc::dashcore::Txid,
    ) -> DAPIResult<dashcore::Transaction> {
        trace!("Core RPC: get_raw_transaction");
        self.guarded_blocking_call(move |client| client.get_raw_transaction(&txid, None))
            .await?
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
                        .await??;
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

    /// Obtain the latest ChainLock if available, tolerating Core's "not ready" response.
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
            Err(e) => Err(e),
        }
    }

    /// Request a masternode list diff between two block hashes via `protx diff`.
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
            .await??;
        Ok(diff)
    }

    /// Fetch general blockchain state information from Dash Core.
    pub async fn get_blockchain_info(
        &self,
    ) -> DAPIResult<dashcore_rpc::json::GetBlockchainInfoResult> {
        trace!("Core RPC: get_blockchain_info");
        let info = self
            .guarded_blocking_call(|client| client.get_blockchain_info())
            .await??;
        Ok(info)
    }

    /// Fetch network-level statistics and connection details from Dash Core.
    pub async fn get_network_info(&self) -> DAPIResult<dashcore_rpc::json::GetNetworkInfoResult> {
        trace!("Core RPC: get_network_info");
        let info = self
            .guarded_blocking_call(|client| client.get_network_info())
            .await??;
        Ok(info)
    }

    /// Estimate the smart fee in Dash per KB for the target confirmation window.
    pub async fn estimate_smart_fee_btc_per_kb(&self, blocks: u16) -> DAPIResult<Option<f64>> {
        trace!("Core RPC: estimatesmartfee");
        let result = self
            .guarded_blocking_call(move |client| client.estimate_smart_fee(blocks, None))
            .await??;
        Ok(result.fee_rate.map(|a| a.to_dash()))
    }

    /// Query the local masternode status from Dash Core.
    pub async fn get_masternode_status(&self) -> DAPIResult<dashcore_rpc::json::MasternodeStatus> {
        trace!("Core RPC: masternode status");
        let st = self
            .guarded_blocking_call(|client| client.get_masternode_status())
            .await??;
        Ok(st)
    }

    /// Fetch the deterministic masternode synchronization status from Dash Core.
    pub async fn mnsync_status(&self) -> DAPIResult<dashcore_rpc::json::MnSyncStatus> {
        trace!("Core RPC: mnsync status");
        let st = self
            .guarded_blocking_call(|client| client.mnsync_status())
            .await??;
        Ok(st)
    }

    /// Retrieve the PoSe penalty score for the specified masternode ProTx hash.
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
            .await??;

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
    /// Construct a semaphore-backed guard limiting concurrent Core RPC calls.
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    /// Acquire a permit, ensuring at most `max_concurrent` active RPC requests.
    async fn acquire(&self) -> OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Core RPC access guard semaphore not closed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc as std_mpsc;
    use std::time::Duration;
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;

    /// Upper bound for a test step that should finish at a short deadline,
    /// far below `REQUEST_TIMEOUT`.
    const STEP_LIMIT: Duration = Duration::from_secs(5);

    type BlockedCall = JoinHandle<Result<Result<(), ()>, DapiError>>;

    /// A client whose calls in these tests never reach Core: each operation
    /// ignores the RPC client.
    fn test_client() -> CoreClient {
        CoreClient::new(
            "http://127.0.0.1:1".to_string(),
            "user".to_string(),
            Zeroizing::new("pass".to_string()),
            1024 * 1024,
        )
        .expect("client")
    }

    /// Start a call whose operation blocks until the returned sender is used
    /// or dropped. Returns once the operation runs, so it holds a permit.
    async fn start_blocked_call(
        client: &CoreClient,
        deadline: Instant,
    ) -> (BlockedCall, std_mpsc::Sender<()>) {
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = std_mpsc::channel::<()>();
        let caller = tokio::spawn({
            let client = client.clone();
            async move {
                client
                    .guarded_blocking_call_until(deadline, move |_| {
                        let _ = started_tx.send(());
                        let _ = release_rx.recv();
                        Ok::<_, ()>(())
                    })
                    .await
            }
        });
        started_rx.await.expect("call started");
        (caller, release_tx)
    }

    /// Wait until every permit is free again.
    async fn wait_for_all_permits(client: &CoreClient) {
        let all = tokio::time::timeout(
            STEP_LIMIT,
            client
                .access_guard
                .semaphore
                .acquire_many(CORE_RPC_GUARD_PERMITS as u32),
        )
        .await
        .expect("permits come back once the calls return")
        .expect("semaphore open");
        drop(all);
    }

    #[tokio::test]
    async fn should_keep_permits_until_cancelled_calls_return() {
        let client = test_client();

        let mut releases = Vec::new();
        for _ in 0..CORE_RPC_GUARD_PERMITS {
            let (caller, release) =
                start_blocked_call(&client, Instant::now() + REQUEST_TIMEOUT).await;
            releases.push(release);
            caller.abort();
            assert!(caller.await.expect_err("caller aborted").is_cancelled());
        }

        // Every caller is gone, but every call is still running on Core.
        assert_eq!(client.access_guard.semaphore.available_permits(), 0);

        drop(releases);
        wait_for_all_permits(&client).await;

        let result = client
            .guarded_blocking_call(|_| Ok::<_, ()>(7))
            .await
            .expect("call completes");
        assert_eq!(result, Ok(7));
    }

    #[tokio::test]
    async fn should_time_out_a_call_still_waiting_for_a_permit_at_its_deadline() {
        let client = test_client();

        let mut holders = Vec::new();
        for _ in 0..CORE_RPC_GUARD_PERMITS {
            holders.push(start_blocked_call(&client, Instant::now() + REQUEST_TIMEOUT).await);
        }

        let ran = Arc::new(AtomicBool::new(false));
        let result = tokio::time::timeout(
            STEP_LIMIT,
            client.guarded_blocking_call_until(Instant::now() + Duration::from_millis(50), {
                let ran = ran.clone();
                move |_| {
                    ran.store(true, Ordering::SeqCst);
                    Ok::<_, ()>(())
                }
            }),
        )
        .await
        .expect("the wait for a permit ends at the call's deadline");

        assert!(
            matches!(result, Err(DapiError::Timeout(_))),
            "expected a timeout, got {result:?}"
        );
        assert!(!ran.load(Ordering::SeqCst), "the queued call never ran");

        for (caller, release) in holders {
            release.send(()).expect("holder still running");
            assert_eq!(
                caller.await.expect("holder finished").expect("holder call"),
                Ok(())
            );
        }
        wait_for_all_permits(&client).await;
    }

    #[tokio::test]
    async fn should_time_out_a_running_call_at_the_same_deadline_and_keep_its_permit() {
        let client = test_client();

        // The permit is free, so the operation starts at once and the call
        // stage gets whatever is left of the same 100 ms deadline.
        let (caller, release) =
            start_blocked_call(&client, Instant::now() + Duration::from_millis(100)).await;
        let result = tokio::time::timeout(STEP_LIMIT, caller)
            .await
            .expect("the wait for the call ends at the same deadline")
            .expect("caller finished");

        assert!(
            matches!(result, Err(DapiError::Timeout(_))),
            "expected a timeout, got {result:?}"
        );
        // The caller gave up, but the operation is still running and keeps its permit.
        assert_eq!(
            client.access_guard.semaphore.available_permits(),
            CORE_RPC_GUARD_PERMITS - 1
        );

        drop(release);
        wait_for_all_permits(&client).await;
    }
}
