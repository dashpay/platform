// Core service implementation

use crate::cache::LruResponseCache;
use crate::clients::CoreClient;
use crate::config::Config;
use crate::services::streaming_service::{FilterType, StreamingServiceImpl};
use dapi_grpc::core::v0::{
    core_server::Core, BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetBestBlockHeightRequest,
    GetBestBlockHeightResponse, GetBlockRequest, GetBlockResponse, GetBlockchainStatusRequest,
    GetBlockchainStatusResponse, GetEstimatedTransactionFeeRequest,
    GetEstimatedTransactionFeeResponse, GetMasternodeStatusRequest, GetMasternodeStatusResponse,
    GetTransactionRequest, GetTransactionResponse, MasternodeListRequest, MasternodeListResponse,
    TransactionsWithProofsRequest, TransactionsWithProofsResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dashcore_rpc::dashcore::hashes::Hash;
use std::sync::Arc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::trace;

/// Core service implementation that handles blockchain and streaming operations
#[derive(Clone)]
pub struct CoreServiceImpl {
    pub streaming_service: Arc<StreamingServiceImpl>,
    pub config: Arc<Config>,
    pub core_client: CoreClient,
    pub core_cache: LruResponseCache,
}

impl CoreServiceImpl {
    pub async fn new(
        streaming_service: Arc<StreamingServiceImpl>,
        config: Arc<Config>,
        core_client: CoreClient,
    ) -> Self {
        let invalidation_subscription = streaming_service
            .subscriber_manager
            .add_subscription(FilterType::CoreNewBlockHash)
            .await;

        Self {
            streaming_service,
            config,
            core_client,
            core_cache: LruResponseCache::new(1024, invalidation_subscription),
        }
    }
}

#[dapi_grpc::tonic::async_trait]
impl Core for CoreServiceImpl {
    type subscribeToBlockHeadersWithChainLocksStream =
        UnboundedReceiverStream<Result<BlockHeadersWithChainLocksResponse, Status>>;
    type subscribeToTransactionsWithProofsStream =
        UnboundedReceiverStream<Result<TransactionsWithProofsResponse, Status>>;
    type subscribeToMasternodeListStream =
        UnboundedReceiverStream<Result<MasternodeListResponse, Status>>;

    async fn get_block(
        &self,
        request: Request<GetBlockRequest>,
    ) -> Result<Response<GetBlockResponse>, Status> {
        trace!("Received get_block request");
        let req = request.into_inner();

        let block_bytes = match req.block {
            Some(dapi_grpc::core::v0::get_block_request::Block::Height(height)) => {
                let hash = self
                    .core_client
                    .get_block_hash(height)
                    .await
                    .map_err(tonic::Status::from)?;
                self.core_client
                    .get_block_bytes_by_hash(hash)
                    .await
                    .map_err(tonic::Status::from)?
            }
            Some(dapi_grpc::core::v0::get_block_request::Block::Hash(hash_hex)) => self
                .core_client
                .get_block_bytes_by_hash_hex(&hash_hex)
                .await
                .map_err(tonic::Status::from)?,
            None => {
                return Err(Status::invalid_argument(
                    "either height or hash must be provided",
                ))
            }
        };

        Ok(Response::new(GetBlockResponse { block: block_bytes }))
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        trace!("Received get_transaction request");
        let txid = request.into_inner().id;

        let info = self
            .core_client
            .get_transaction_info(&txid)
            .await
            .map_err(tonic::Status::from)?;

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

    async fn get_best_block_height(
        &self,
        _request: Request<GetBestBlockHeightRequest>,
    ) -> Result<Response<GetBestBlockHeightResponse>, Status> {
        trace!("Received get_best_block_height request");
        let height = self
            .core_client
            .get_block_count()
            .await
            .map_err(tonic::Status::from)?;

        Ok(Response::new(GetBestBlockHeightResponse { height }))
    }

    async fn broadcast_transaction(
        &self,
        request: Request<BroadcastTransactionRequest>,
    ) -> Result<Response<BroadcastTransactionResponse>, Status> {
        trace!("Received broadcast_transaction request");
        let req = request.into_inner();
        let _allow_high_fees = req.allow_high_fees;
        let _bypass_limits = req.bypass_limits;

        // NOTE: dashcore-rpc Client does not expose options for allowhighfees/bypasslimits.
        // We broadcast as-is. Future: add support if library exposes those options.
        let txid = self
            .core_client
            .send_raw_transaction(&req.transaction)
            .await
            .map_err(tonic::Status::from)?;

        Ok(Response::new(BroadcastTransactionResponse {
            transaction_id: txid,
        }))
    }

    async fn get_blockchain_status(
        &self,
        _request: Request<GetBlockchainStatusRequest>,
    ) -> Result<Response<GetBlockchainStatusResponse>, Status> {
        trace!("Received get_blockchain_status request");
        let (bc_info, net_info) = tokio::join!(
            self.core_client.get_blockchain_info(),
            self.core_client.get_network_info()
        );

        let bc_info = bc_info.map_err(tonic::Status::from)?;
        let net_info = net_info.map_err(tonic::Status::from)?;

        use dapi_grpc::core::v0::get_blockchain_status_response as respmod;

        // Version
        let version = respmod::Version {
            protocol: net_info.protocol_version as u32,
            software: net_info.version as u32,
            agent: net_info.subversion.clone(),
        };

        // Time
        let time = respmod::Time {
            now: chrono::Utc::now().timestamp() as u32,
            offset: net_info.time_offset as i32,
            median: bc_info.median_time as u32,
        };

        // Status and sync progress
        let sync_progress = bc_info.verification_progress;
        let status = if !bc_info.warnings.is_empty() {
            respmod::Status::Error as i32
        } else if sync_progress >= 0.9999 {
            respmod::Status::Ready as i32
        } else {
            respmod::Status::Syncing as i32
        };

        // Chain
        let best_block_hash_bytes = bc_info.best_block_hash.to_byte_array().to_vec();
        let chain_work_bytes = bc_info.chainwork.clone();
        let chain = respmod::Chain {
            name: bc_info.chain,
            headers_count: bc_info.headers as u32,
            blocks_count: bc_info.blocks as u32,
            best_block_hash: best_block_hash_bytes,
            difficulty: bc_info.difficulty,
            chain_work: chain_work_bytes,
            is_synced: status == respmod::Status::Ready as i32,
            sync_progress,
        };

        // Network
        let network = respmod::Network {
            peers_count: net_info.connections as u32,
            fee: Some(respmod::NetworkFee {
                relay: net_info.relay_fee.to_dash(),
                incremental: net_info.incremental_fee.to_dash(),
            }),
        };

        let response = GetBlockchainStatusResponse {
            version: Some(version),
            time: Some(time),
            status,
            sync_progress,
            chain: Some(chain),
            network: Some(network),
        };

        Ok(Response::new(response))
    }

    async fn get_masternode_status(
        &self,
        _request: Request<GetMasternodeStatusRequest>,
    ) -> Result<Response<GetMasternodeStatusResponse>, Status> {
        trace!("Received get_masternode_status request");
        use dapi_grpc::core::v0::get_masternode_status_response::Status as MnStatus;
        use dashcore_rpc::json::MasternodeState as CoreStatus;

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

    async fn get_estimated_transaction_fee(
        &self,
        request: Request<GetEstimatedTransactionFeeRequest>,
    ) -> Result<Response<GetEstimatedTransactionFeeResponse>, Status> {
        trace!("Received get_estimated_transaction_fee request");
        let blocks = request.into_inner().blocks.clamp(1, 1000) as u16;
        let fee = self
            .core_client
            .estimate_smart_fee_btc_per_kb(blocks)
            .await
            .map_err(tonic::Status::from)?
            .unwrap_or(0.0);

        Ok(Response::new(GetEstimatedTransactionFeeResponse { fee }))
    }

    async fn subscribe_to_block_headers_with_chain_locks(
        &self,
        request: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<Response<<Self as Core>::subscribeToBlockHeadersWithChainLocksStream>, Status> {
        trace!("Received subscribe_to_block_headers_with_chain_locks request");
        self.streaming_service
            .subscribe_to_block_headers_with_chain_locks_impl(request)
            .await
    }

    async fn subscribe_to_transactions_with_proofs(
        &self,
        request: Request<TransactionsWithProofsRequest>,
    ) -> Result<Response<Self::subscribeToTransactionsWithProofsStream>, Status> {
        trace!("Received subscribe_to_transactions_with_proofs request");
        self.streaming_service
            .subscribe_to_transactions_with_proofs_impl(request)
            .await
    }

    async fn subscribe_to_masternode_list(
        &self,
        request: Request<MasternodeListRequest>,
    ) -> Result<Response<Self::subscribeToMasternodeListStream>, Status> {
        trace!("Received subscribe_to_masternode_list request");
        self.streaming_service
            .subscribe_to_masternode_list_impl(request)
            .await
    }
}
