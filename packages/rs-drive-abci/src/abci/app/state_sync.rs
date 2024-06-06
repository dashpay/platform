use crate::abci::app::PlatformApplication;
use crate::abci::handler;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::utils::spawn_blocking_task_with_name_if_supported;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::mem::take;
use std::sync::{Arc, RwLock};
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_offer_snapshot::Result::Accept;
use tenderdash_abci::proto::abci::{
    abci_application_server as grpc_abci_server, ResponseListSnapshots,
};
use tenderdash_abci::proto::tonic;
use tokio::sync::Mutex;
//use dapi_grpc::platform::proto::abci::response_offer_snapshot;
use drive::error::Error::GroveDB;
use drive::grovedb::{GroveDb, Transaction};
//use dapi_grpc::platform::proto::abci::response_offer_snapshot;
use crate::platform_types::snapshot::{Snapshot, SnapshotFetchingSession, SnapshotManager};

/// AbciApp is an implementation of gRPC ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct StateSyncAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Platform
    platform: Arc<Platform<C>>,
    /// Snapshot manager
    snapshot_manager: SnapshotManager,
}

impl<C> PlatformApplication<C> for StateSyncAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn platform(&self) -> &Platform<C> {
        self.platform.as_ref()
    }
}

impl<C> StateSyncAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Create new ABCI app
    pub fn new(platform: Arc<Platform<C>>) -> Self {
        let snapshot_manager = SnapshotManager::new(
            platform
                .config
                .state_sync_config
                .checkpoints_path
                .to_str()
                .unwrap()
                .to_string(),
            platform.config.state_sync_config.max_num_snapshots,
            platform.config.state_sync_config.snapshots_frequency,
        );
        Self {
            platform,
            snapshot_manager,
        }
    }
}

impl<C> Debug for StateSyncAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<StateSyncAbciApplication>")
    }
}

#[async_trait]
impl<C> grpc_abci_server::AbciApplication for StateSyncAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    async fn list_snapshots(
        &self,
        _: tonic::Request<proto::RequestListSnapshots>,
    ) -> Result<tonic::Response<proto::ResponseListSnapshots>, tonic::Status> {
        tracing::trace!("[state_sync] api list_snapshots called");
        let snapshots = self
            .snapshot_manager
            .get_snapshots(&self.platform.drive.grove)
            .map_err(|e| tonic::Status::internal(format!("list_snapshots failed: {}", e)))?;
        let mut response: proto::ResponseListSnapshots = Default::default();
        let convert_snapshots = |s: Snapshot| -> proto::Snapshot {
            proto::Snapshot {
                height: s.height as u64,
                version: s.version as u32,
                hash: s.hash.to_vec(),
                metadata: s.metadata,
            }
        };
        let checkpoint_exists = |s: &Snapshot| -> bool {
            match GroveDb::open(&s.path) {
                Ok(_) => true,
                Err(_) => false,
            }
        };

        response.snapshots = snapshots
            .into_iter()
            .filter(checkpoint_exists)
            .map(convert_snapshots)
            .collect();
        Ok(tonic::Response::new(response))
    }

    async fn load_snapshot_chunk(
        &self,
        request: tonic::Request<proto::RequestLoadSnapshotChunk>,
    ) -> Result<tonic::Response<proto::ResponseLoadSnapshotChunk>, tonic::Status> {
        let request_snapshot_chunk = request.into_inner();
        tracing::trace!(
            "[state_sync] api load_snapshot_chunk height:{} chunk_id:{}",
            request_snapshot_chunk.height,
            hex::encode(&request_snapshot_chunk.chunk_id)
        );
        let matched_snapshot = self
            .snapshot_manager
            .get_snapshot_at_height(
                &self.platform.drive.grove,
                request_snapshot_chunk.height as i64,
            )
            .map_err(|_| tonic::Status::internal("load_snapshot_chunk failed".to_string()))?
            .ok_or_else(|| tonic::Status::internal("load_snapshot_chunk failed"))?;
        let db = GroveDb::open(&matched_snapshot.path)
            .map_err(|e| tonic::Status::internal(format!("load_snapshot_chunk failed: {}", e)))?;
        let chunk = db
            .fetch_chunk(
                &request_snapshot_chunk.chunk_id,
                None,
                request_snapshot_chunk.version as u16,
            )
            .map_err(|e| tonic::Status::internal(format!("load_snapshot_chunk failed: {}", e)))?;
        let mut response = proto::ResponseLoadSnapshotChunk::default();
        response.chunk = chunk;
        Ok(tonic::Response::new(response))
    }
}
