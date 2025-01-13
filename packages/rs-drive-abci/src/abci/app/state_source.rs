use crate::abci::app::{PlatformApplication, SnapshotManagerApplication};
use crate::abci::handler;
use crate::platform_types::platform::Platform;
use crate::platform_types::snapshot::SnapshotManager;
use crate::rpc::core::CoreRPCLike;
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
pub struct StateSourceAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Platform
    platform: Arc<Platform<C>>,
    /// Snapshot manager
    snapshot_manager: SnapshotManager,
}

impl<C> PlatformApplication<C> for StateSourceAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn platform(&self) -> &Platform<C> {
        self.platform.as_ref()
    }
}

impl<C> SnapshotManagerApplication for StateSourceAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn snapshot_manager(&self) -> &SnapshotManager {
        &self.snapshot_manager
    }
}

impl<C> StateSourceAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Create new ABCI app
    pub fn new(platform: Arc<Platform<C>>) -> Self {
        let snapshot_manager = SnapshotManager::new(
            platform
                .config
                .state_sync_config
                .checkpoints_path.clone(),
            platform.config.state_sync_config.max_num_snapshots,
            platform.config.state_sync_config.snapshots_frequency,
        );
        Self {
            platform,
            snapshot_manager,
        }
    }
}

impl<C> Debug for StateSourceAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<StateSourceAbciApplication>")
    }
}

#[async_trait]
impl<C> grpc_abci_server::AbciApplication for StateSourceAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
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
