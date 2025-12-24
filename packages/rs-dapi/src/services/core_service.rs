// Core service implementation

use crate::DapiError;
use crate::clients::CoreClient;
use crate::config::Config;
use crate::services::streaming_service::StreamingServiceImpl;
use dapi_grpc::core::v0::{
    BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetBestBlockHeightRequest,
    GetBestBlockHeightResponse, GetBlockRequest, GetBlockResponse, GetBlockchainStatusRequest,
    GetBlockchainStatusResponse, GetEstimatedTransactionFeeRequest,
    GetEstimatedTransactionFeeResponse, GetMasternodeStatusRequest, GetMasternodeStatusResponse,
    GetTransactionRequest, GetTransactionResponse, MasternodeListRequest, MasternodeListResponse,
    TransactionsWithProofsRequest, TransactionsWithProofsResponse, core_server::Core,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dashcore_rpc::dashcore::consensus::encode::deserialize as deserialize_tx;
use dashcore_rpc::dashcore::hashes::Hash;
use std::any::type_name_of_val;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info, trace, warn};

/// Core service implementation that handles blockchain and streaming operations.
///
/// Supports cheap Clone operation, no need to put it into Arc.
#[derive(Clone)]
pub struct CoreServiceImpl {
    pub streaming_service: Arc<StreamingServiceImpl>,
    pub config: Arc<Config>,
    pub core_client: CoreClient,
}

impl CoreServiceImpl {
    /// Build the Core service by wiring the streaming service, config, and RPC client.
    /// Used by server startup to prepare gRPC handlers.
    pub async fn new(
        streaming_service: Arc<StreamingServiceImpl>,
        config: Arc<Config>,
        core_client: CoreClient,
    ) -> Self {
        Self {
            streaming_service,
            config,
            core_client,
        }
    }
}

#[dapi_grpc::tonic::async_trait]
impl Core for CoreServiceImpl {
    type subscribeToBlockHeadersWithChainLocksStream =
        ReceiverStream<Result<BlockHeadersWithChainLocksResponse, Status>>;
    type subscribeToTransactionsWithProofsStream =
        ReceiverStream<Result<TransactionsWithProofsResponse, Status>>;
    type subscribeToMasternodeListStream = ReceiverStream<Result<MasternodeListResponse, Status>>;

    /// Fetch a block by height or hash, translating Core errors into gRPC statuses.
    async fn get_block(
        &self,
        request: Request<GetBlockRequest>,
    ) -> Result<Response<GetBlockResponse>, Status> {
        trace!("Received get_block request");
        let method = type_name_of_val(request.get_ref());
        let req = request.into_inner();
        let request_target = match req.block.as_ref() {
            Some(dapi_grpc::core::v0::get_block_request::Block::Height(height)) => {
                format!("height:{height}")
            }
            Some(dapi_grpc::core::v0::get_block_request::Block::Hash(hash)) => {
                format!("hash:{}", hash.trim())
            }
            None => "unspecified".to_string(),
        };

        let this = self;
        let result: Result<Response<GetBlockResponse>, Status> = async move {
            let block_bytes = match req.block {
                Some(dapi_grpc::core::v0::get_block_request::Block::Height(height)) => {
                    let hash = this
                        .core_client
                        .get_block_hash(height)
                        .await
                        .map_err(|err| match err {
                            DapiError::NotFound(_) => {
                                DapiError::InvalidArgument("Invalid block height".to_string())
                                    .into_legacy_status()
                            }
                            DapiError::InvalidArgument(msg) => {
                                DapiError::InvalidArgument(msg).into_legacy_status()
                            }
                            other => other.to_status(),
                        })?;
                    this.core_client
                        .get_block_bytes_by_hash(hash)
                        .await
                        .map_err(|err| match err {
                            DapiError::NotFound(_) => {
                                DapiError::NotFound("Block not found".to_string())
                                    .into_legacy_status()
                            }
                            other => other.to_status(),
                        })?
                }
                Some(dapi_grpc::core::v0::get_block_request::Block::Hash(hash_hex)) => {
                    if hash_hex.trim().is_empty() {
                        return Err(Status::invalid_argument("hash or height is not specified"));
                    }

                    this.core_client
                        .get_block_bytes_by_hash_hex(&hash_hex)
                        .await
                        .map_err(|err| match err {
                            DapiError::InvalidArgument(msg) => {
                                DapiError::InvalidArgument(msg).into_legacy_status()
                            }
                            DapiError::NotFound(_) => {
                                DapiError::NotFound("Block not found".to_string())
                                    .into_legacy_status()
                            }
                            other => other.to_status(),
                        })?
                }
                None => {
                    return Err(Status::invalid_argument("hash or height is not specified"));
                }
            };

            Ok(Response::new(GetBlockResponse { block: block_bytes }))
        }
        .await;

        match &result {
            Ok(_) => info!(method, %request_target, "request succeeded"),
            Err(status) => warn!(method, %request_target, error = %status, "request failed"),
        }

        result
    }

    /// Retrieve transaction details including confirmations and lock states.
    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        trace!("Received get_transaction request");
        let method = type_name_of_val(request.get_ref());
        let txid = request.into_inner().id;
        let log_txid = txid.trim().to_owned();

        let result: Result<Response<GetTransactionResponse>, Status> = async move {
            if txid.trim().is_empty() {
                return Err(Status::invalid_argument("id is not specified"));
            }

            let info =
                self.core_client
                    .get_transaction_info(&txid)
                    .await
                    .map_err(|err| match err {
                        DapiError::NotFound(_) => {
                            DapiError::NotFound("Transaction not found".to_string())
                                .into_legacy_status()
                        }
                        DapiError::InvalidArgument(msg) => {
                            DapiError::InvalidArgument(msg).into_legacy_status()
                        }
                        DapiError::Client(msg) => DapiError::Client(msg).into_legacy_status(),
                        other => other.to_status(),
                    })?;

            let transaction = info.hex.clone();
            let block_hash = info
                .blockhash
                .map(|h| hex::decode(h.to_string()).unwrap_or_default())
                .unwrap_or_default();
            let height = match info.height {
                Some(h) if h >= 0 => h as u32,
                _ => 0,
            };
            let confirmations = info.confirmations.unwrap_or(0);
            let is_instant_locked = info.instantlock_internal;
            let is_chain_locked = info.chainlock;

            let response = GetTransactionResponse {
                transaction,
                block_hash,
                height,
                confirmations,
                is_instant_locked,
                is_chain_locked,
            };
            Ok(Response::new(response))
        }
        .await;

        match &result {
            Ok(_) => info!(method, txid = log_txid.as_str(), "request succeeded"),
            Err(status) => {
                warn!(method, txid = log_txid.as_str(), error = %status, "request failed")
            }
        }

        result
    }

    /// Return the best block height from Dash Core for legacy clients.
    async fn get_best_block_height(
        &self,
        request: Request<GetBestBlockHeightRequest>,
    ) -> Result<Response<GetBestBlockHeightResponse>, Status> {
        trace!("Received get_best_block_height request");
        let method = type_name_of_val(request.get_ref());
        let _ = request;
        let result: Result<Response<GetBestBlockHeightResponse>, Status> = async {
            let height = self
                .core_client
                .get_block_count()
                .await
                .map_err(tonic::Status::from)?;

            Ok(Response::new(GetBestBlockHeightResponse { height }))
        }
        .await;

        match &result {
            Ok(response) => info!(
                method,
                height = response.get_ref().height,
                "request succeeded"
            ),
            Err(status) => warn!(method, error = %status, "request failed"),
        }

        result
    }

    /// Validate and broadcast a transaction to Dash Core, returning its txid.
    async fn broadcast_transaction(
        &self,
        request: Request<BroadcastTransactionRequest>,
    ) -> Result<Response<BroadcastTransactionResponse>, Status> {
        trace!("Received broadcast_transaction request");
        let method = type_name_of_val(request.get_ref());
        let req = request.into_inner();
        let _allow_high_fees = req.allow_high_fees;
        let _bypass_limits = req.bypass_limits;

        let result: Result<Response<BroadcastTransactionResponse>, Status> = async {
            if req.transaction.is_empty() {
                return Err(Status::invalid_argument("transaction is not specified"));
            }

            if let Err(err) =
                deserialize_tx::<dashcore_rpc::dashcore::Transaction>(&req.transaction)
            {
                return Err(Status::invalid_argument(format!(
                    "invalid transaction: {}",
                    err
                )));
            }

            // NOTE: dashcore-rpc Client does not expose options for allowhighfees/bypasslimits.
            // We broadcast as-is. Future: add support if library exposes those options.
            let txid = self
                .core_client
                .send_raw_transaction(&req.transaction)
                .await
                .map_err(|err| match err {
                    DapiError::InvalidArgument(msg) => {
                        DapiError::InvalidArgument(format!("invalid transaction: {}", msg))
                            .into_legacy_status()
                    }
                    DapiError::FailedPrecondition(msg) => {
                        DapiError::FailedPrecondition(format!("Transaction is rejected: {}", msg))
                            .into_legacy_status()
                    }
                    DapiError::AlreadyExists(msg) => {
                        DapiError::AlreadyExists(format!("Transaction already in chain: {}", msg))
                            .into_legacy_status()
                    }
                    DapiError::Client(msg) => DapiError::Client(msg).into_legacy_status(),
                    other => other.to_status(),
                })?;

            Ok(Response::new(BroadcastTransactionResponse {
                transaction_id: txid,
            }))
        }
        .await;

        match &result {
            Ok(resp) => info!(
                method,
                txid = resp.get_ref().transaction_id,
                "request succeeded"
            ),
            Err(status) => warn!(method, error = %status, "request failed"),
        }

        result
    }

    /// Fetch blockchain status metrics (similar to `getblockchaininfo`).
    async fn get_blockchain_status(
        &self,
        request: Request<GetBlockchainStatusRequest>,
    ) -> Result<Response<GetBlockchainStatusResponse>, Status> {
        trace!("Received get_blockchain_status request");
        let method = type_name_of_val(request.get_ref());
        let _ = request;
        let result: Result<Response<GetBlockchainStatusResponse>, Status> = async {
            trace!("Fetching blockchain_info and network_info from Core");
            let (bc_info_res, net_info_res) = tokio::join!(
                self.core_client.get_blockchain_info(),
                self.core_client.get_network_info()
            );

            if let Err(ref err) = bc_info_res {
                debug!(error = ?err, "Failed to retrieve blockchain info from Core RPC");
            }
            if let Err(ref err) = net_info_res {
                debug!(error = ?err, "Failed to retrieve network info from Core RPC");
            }

            let bc_info = bc_info_res.ok();
            let net_info = net_info_res.ok();

            trace!(?bc_info, "Core blockchain info retrieved");
            trace!(?net_info, "Core network info retrieved");

            use dapi_grpc::core::v0::get_blockchain_status_response as respmod;

            // Version
            let version = net_info.as_ref().map(|info| respmod::Version {
                protocol: info.protocol_version as u32,
                software: info.version as u32,
                agent: info.subversion.clone(),
            });

            // Time
            let time = if let Some(bc) = &bc_info
                && let Some(net) = &net_info
            {
                let now = chrono::Utc::now().timestamp() as u32;
                let offset = net.time_offset as i32;
                let median = bc.median_time as u32;
                Some(respmod::Time {
                    now,
                    offset,
                    median,
                })
            } else {
                None
            };

            let (chain, status) = if let Some(info) = &bc_info {
                // Status and sync progress
                let sync_progress = info.verification_progress;
                let status = if !info.warnings.is_empty() {
                    respmod::Status::Error as i32
                } else if sync_progress >= 0.9999 {
                    respmod::Status::Ready as i32
                } else {
                    respmod::Status::Syncing as i32
                };

                // Chain
                let best_block_hash_bytes = info.best_block_hash.to_byte_array().to_vec();
                let chain_work_bytes = info.chainwork.clone();
                let chain = respmod::Chain {
                    name: info.chain.clone(),
                    headers_count: info.headers as u32,
                    blocks_count: info.blocks as u32,
                    best_block_hash: best_block_hash_bytes,
                    difficulty: info.difficulty,
                    chain_work: chain_work_bytes,
                    is_synced: status == respmod::Status::Ready as i32,
                    sync_progress,
                };
                (Some(chain), Some(status))
            } else {
                (None, None)
            };

            // Network
            let network = net_info.as_ref().map(|info| respmod::Network {
                peers_count: info.connections as u32,
                fee: Some(respmod::NetworkFee {
                    relay: info.relay_fee.to_dash(),
                    incremental: info.incremental_fee.to_dash(),
                }),
            });

            let response = GetBlockchainStatusResponse {
                version,
                time,
                status: status.unwrap_or(respmod::Status::Error as i32),
                sync_progress: chain.as_ref().map(|c| c.sync_progress).unwrap_or(0.0),
                chain,
                network,
            };

            trace!(
                status = status,
                sync_progress = response.sync_progress,
                "Prepared get_blockchain_status response"
            );

            Ok(Response::new(response))
        }
        .await;

        match &result {
            Ok(resp) => info!(
                method,
                status = resp.get_ref().status,
                sync_progress = resp.get_ref().sync_progress,
                "request succeeded"
            ),
            Err(status) => warn!(method, error = %status, "request failed"),
        }

        result
    }

    /// Return the masternode status for the current node via Dash Core.
    async fn get_masternode_status(
        &self,
        request: Request<GetMasternodeStatusRequest>,
    ) -> Result<Response<GetMasternodeStatusResponse>, Status> {
        trace!("Received get_masternode_status request");
        let method = type_name_of_val(request.get_ref());
        let _ = request;
        use dapi_grpc::core::v0::get_masternode_status_response::Status as MnStatus;
        use dashcore_rpc::json::MasternodeState as CoreStatus;

        let result: Result<Response<GetMasternodeStatusResponse>, Status> = async {
            // Query core for masternode status and overall sync status
            let (mn_status_res, mnsync_res) = tokio::join!(
                self.core_client.get_masternode_status(),
                self.core_client.mnsync_status()
            );

            let mn_status = mn_status_res.map_err(tonic::Status::from)?;
            let mnsync = mnsync_res.map_err(tonic::Status::from)?;

            // Map masternode state to gRPC enum
            let status_enum = match mn_status.state {
                CoreStatus::MasternodeWaitingForProtx => MnStatus::WaitingForProtx as i32,
                CoreStatus::MasternodePoseBanned => MnStatus::PoseBanned as i32,
                CoreStatus::MasternodeRemoved => MnStatus::Removed as i32,
                CoreStatus::MasternodeOperatorKeyChanged => MnStatus::OperatorKeyChanged as i32,
                CoreStatus::MasternodeProtxIpChanged => MnStatus::ProtxIpChanged as i32,
                CoreStatus::MasternodeReady => MnStatus::Ready as i32,
                CoreStatus::MasternodeError => MnStatus::Error as i32,
                CoreStatus::Nonrecognised | CoreStatus::Unknown => MnStatus::Unknown as i32,
            };

            // pro_tx_hash bytes
            let pro_tx_hash_hex = mn_status.pro_tx_hash.to_string();
            let pro_tx_hash_bytes = hex::decode(&pro_tx_hash_hex).unwrap_or_default();

            // Get PoSe penalty via masternode list filtered by protx hash
            let pose_penalty = match self
                .core_client
                .get_masternode_pos_penalty(&pro_tx_hash_hex)
                .await
            {
                Ok(Some(score)) => score,
                _ => 0,
            };

            // Sync flags and progress computed from AssetID (JS parity)
            let is_synced = mnsync.is_synced;
            let sync_progress = match mnsync.asset_id {
                999 => 1.0,     // FINISHED
                0 => 0.0,       // INITIAL
                1 => 1.0 / 3.0, // BLOCKCHAIN
                4 => 2.0 / 3.0, // GOVERNANCE (legacy numeric value)
                _ => 0.0,
            };

            let response = GetMasternodeStatusResponse {
                status: status_enum,
                pro_tx_hash: pro_tx_hash_bytes,
                pose_penalty,
                is_synced,
                sync_progress,
            };

            Ok(Response::new(response))
        }
        .await;

        match &result {
            Ok(resp) => info!(
                method,
                status = resp.get_ref().status,
                synced = resp.get_ref().is_synced,
                "request succeeded"
            ),
            Err(status) => warn!(method, error = %status, "request failed"),
        }

        result
    }

    /// Estimate smart fee rate for a confirmation target, preserving legacy units.
    async fn get_estimated_transaction_fee(
        &self,
        request: Request<GetEstimatedTransactionFeeRequest>,
    ) -> Result<Response<GetEstimatedTransactionFeeResponse>, Status> {
        trace!("Received get_estimated_transaction_fee request");
        let method = type_name_of_val(request.get_ref());
        let blocks = request.into_inner().blocks.clamp(1, 1000) as u16;

        let result: Result<Response<GetEstimatedTransactionFeeResponse>, Status> = async {
            let fee = self
                .core_client
                .estimate_smart_fee_btc_per_kb(blocks)
                .await
                .map_err(tonic::Status::from)?
                .unwrap_or(0.0);

            Ok(Response::new(GetEstimatedTransactionFeeResponse { fee }))
        }
        .await;

        match &result {
            Ok(resp) => info!(
                method,
                blocks,
                fee = resp.get_ref().fee,
                "request succeeded"
            ),
            Err(status) => warn!(method, blocks, error = %status, "request failed"),
        }

        result
    }

    /// Stream block headers with optional chain locks, selecting optimal delivery mode.
    async fn subscribe_to_block_headers_with_chain_locks(
        &self,
        request: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<Response<<Self as Core>::subscribeToBlockHeadersWithChainLocksStream>, Status> {
        trace!("Received subscribe_to_block_headers_with_chain_locks request");
        let method = type_name_of_val(request.get_ref());
        let result = self
            .streaming_service
            .subscribe_to_block_headers_with_chain_locks_impl(request)
            .await;

        match &result {
            Ok(_) => info!(method, "request succeeded"),
            Err(status) => warn!(method, error = %status, "request failed"),
        }

        result
    }

    /// Stream transactions accompanied by proofs via the streaming service.
    async fn subscribe_to_transactions_with_proofs(
        &self,
        request: Request<TransactionsWithProofsRequest>,
    ) -> Result<Response<Self::subscribeToTransactionsWithProofsStream>, Status> {
        trace!("Received subscribe_to_transactions_with_proofs request");
        let method = type_name_of_val(request.get_ref());
        let result = self
            .streaming_service
            .subscribe_to_transactions_with_proofs_impl(request)
            .await;

        match &result {
            Ok(_) => info!(method, "request succeeded"),
            Err(status) => warn!(method, error = %status, "request failed"),
        }

        result
    }

    /// Stream masternode list diffs using the masternode sync helper.
    async fn subscribe_to_masternode_list(
        &self,
        request: Request<MasternodeListRequest>,
    ) -> Result<Response<Self::subscribeToMasternodeListStream>, Status> {
        trace!("Received subscribe_to_masternode_list request");
        let method = type_name_of_val(request.get_ref());
        let result = self
            .streaming_service
            .subscribe_to_masternode_list_impl(request)
            .await;

        match &result {
            Ok(_) => info!(method, "request succeeded"),
            Err(status) => warn!(method, error = %status, "request failed"),
        }

        result
    }
}
