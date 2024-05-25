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
                .checkpoints_path
                .to_str()
                .unwrap()
                .to_string(),
            None,
            None,
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
    // server
    async fn list_snapshots(
        &self,
        _: tonic::Request<proto::RequestListSnapshots>,
    ) -> Result<tonic::Response<proto::ResponseListSnapshots>, tonic::Status> {
        match self
            .snapshot_manager
            .get_snapshots(&self.platform.drive.grove)
        {
            Ok(snapshots) => {
                let mut response: proto::ResponseListSnapshots = Default::default();
                let convert_snapshots = |s: Snapshot| -> proto::Snapshot {
                    proto::Snapshot {
                        height: s.height as u64,
                        version: s.version as u32,
                        hash: s.hash.to_vec(),
                        metadata: s.metadata,
                    }
                };
                response.snapshots = snapshots.into_iter().map(convert_snapshots).collect();
                Ok(tonic::Response::new(response))
            }
            Err(e) => Err(tonic::Status::internal(format!(
                "list_snapshots failed:{}",
                e
            ))),
        }
    }

    async fn load_snapshot_chunk(
        &self,
        request: tonic::Request<proto::RequestLoadSnapshotChunk>,
    ) -> Result<tonic::Response<proto::ResponseLoadSnapshotChunk>, tonic::Status> {
        match self
            .snapshot_manager
            .get_snapshots(&self.platform.drive.grove)
        {
            Ok(snapshots) => {
                let request_snapshot_chunk = request.into_inner();
                let matched_snapshot_opt = snapshots
                    .iter()
                    .find(|&snapshot| snapshot.height == request_snapshot_chunk.height as i64);

                match matched_snapshot_opt {
                    Some(matched_snapshot) => match GroveDb::open(&matched_snapshot.path) {
                        Ok(db) => {
                            match db.fetch_chunk(
                                &request_snapshot_chunk.chunk_id,
                                None,
                                request_snapshot_chunk.version as u16,
                            ) {
                                Ok(chunk) => {
                                    let mut response = proto::ResponseLoadSnapshotChunk::default();
                                    response.chunk = chunk;
                                    Ok(tonic::Response::new(response))
                                }
                                Err(e) => {
                                    return Err(tonic::Status::internal(format!(
                                        "load_snapshot_chunk failed:{}",
                                        e
                                    )))
                                }
                            }
                        }
                        Err(e) => {
                            return Err(tonic::Status::internal(format!(
                                "load_snapshot_chunk failed:{}",
                                e
                            )))
                        }
                    },
                    None => {
                        return Err(tonic::Status::internal(
                            "load_snapshot_chunk failed".to_string(),
                        ))
                    }
                }
            }
            _ => {
                return Err(tonic::Status::internal(
                    "load_snapshot_chunk failed:".to_string(),
                ))
            }
        }
    }
}
