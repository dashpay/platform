use crate::abci::app::{PlatformApplication, SnapshotManagerApplication};
use crate::abci::handler;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::snapshot::SnapshotManager;
use crate::rpc::core::CoreRPCLike;
use crate::utils::spawn_blocking_task_with_name_if_supported;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::abci_application_server as grpc_abci_server;
use tenderdash_abci::proto::tonic;

/// AbciApp is an implementation of gRPC ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Platform
    platform: Arc<Platform<C>>,
    core_rpc: Arc<C>,
    /// Snapshot manager
    snapshot_manager: SnapshotManager,
}

impl<C> PlatformApplication<C> for CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn platform(&self) -> &Platform<C> {
        self.platform.as_ref()
    }
}

impl<C> SnapshotManagerApplication for CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn snapshot_manager(&self) -> &SnapshotManager {
        &self.snapshot_manager
    }
}

impl<C> CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Create new ABCI app
    pub fn new(platform: Arc<Platform<C>>, core_rpc: Arc<C>) -> Self {
        let snapshot_manager = SnapshotManager::new(
            platform.config.abci.state_sync.checkpoints_path.clone(),
            platform.config.abci.state_sync.max_num_snapshots,
            platform.config.abci.state_sync.snapshots_frequency,
        );
        Self {
            platform,
            core_rpc,
            snapshot_manager,
        }
    }
}

impl<C> Debug for CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<CheckTxAbciApplication>")
    }
}

#[async_trait]
impl<C> grpc_abci_server::AbciApplication for CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    async fn echo(
        &self,
        request: tonic::Request<proto::RequestEcho>,
    ) -> Result<tonic::Response<proto::ResponseEcho>, tonic::Status> {
        let response = handler::echo(self, request.into_inner()).map_err(error_into_status)?;

        Ok(tonic::Response::new(response))
    }

    async fn check_tx(
        &self,
        request: tonic::Request<proto::RequestCheckTx>,
    ) -> Result<tonic::Response<proto::ResponseCheckTx>, tonic::Status> {
        let platform = Arc::clone(&self.platform);
        let core_rpc = Arc::clone(&self.core_rpc);

        let proto_request = request.into_inner();

        let check_tx_type = proto::CheckTxType::try_from(proto_request.r#type)
            .map_err(|_| tonic::Status::invalid_argument("invalid check tx type"))?;

        let thread_name = match check_tx_type {
            proto::CheckTxType::New => "check_tx",
            proto::CheckTxType::Recheck => "re_check_tx",
        };

        spawn_blocking_task_with_name_if_supported(thread_name, move || {
            let response = handler::check_tx(&platform, &core_rpc, proto_request)
                .map_err(error_into_status)?;

            Ok(tonic::Response::new(response))
        })?
        .await
        .map_err(|error| tonic::Status::internal(format!("check tx panics: {}", error)))?
    }

    async fn list_snapshots(
        &self,
        request: tonic::Request<proto::RequestListSnapshots>,
    ) -> Result<tonic::Response<proto::ResponseListSnapshots>, tonic::Status> {
        handler::list_snapshots(self, request.into_inner())
            .map(tonic::Response::new)
            .map_err(|e| tonic::Status::internal(format!("list_snapshots failed: {}", e)))
    }

    async fn load_snapshot_chunk(
        &self,
        request: tonic::Request<proto::RequestLoadSnapshotChunk>,
    ) -> Result<tonic::Response<proto::ResponseLoadSnapshotChunk>, tonic::Status> {
        handler::load_snapshot_chunk(self, request.into_inner())
            .map(tonic::Response::new)
            .map_err(|e| tonic::Status::internal(format!("load_snapshot_chunk failed: {}", e)))
    }
}

pub fn error_into_status(error: Error) -> tonic::Status {
    tonic::Status::internal(error.to_string())
}
