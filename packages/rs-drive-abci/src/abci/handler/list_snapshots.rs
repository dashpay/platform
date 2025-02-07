use crate::abci::app::{PlatformApplication, SnapshotManagerApplication};
use crate::abci::handler::error::error_into_exception;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::snapshot::Snapshot;
use crate::rpc::core::CoreRPCLike;
use drive::grovedb::GroveDb;
use std::path::Path;
use tenderdash_abci::proto::abci as proto;

pub fn list_snapshots<A, C>(
    app: &A,
    _: proto::RequestListSnapshots,
) -> Result<proto::ResponseListSnapshots, Error>
where
    A: SnapshotManagerApplication + PlatformApplication<C>,
    C: CoreRPCLike,
{
    println!("[state_sync] api list_snapshots called");
    tracing::trace!("[state_sync] api list_snapshots called");
    let snapshots = app
        .snapshot_manager()
        .get_snapshots(&*app.platform().drive.grove)
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "list_snapshots unable to get snapshots: {}",
                e
            ))
        })?;

    let mut response: proto::ResponseListSnapshots = Default::default();
    let convert_snapshots = |s: Snapshot| -> proto::Snapshot {
        proto::Snapshot {
            height: s.height as u64,
            version: s.version as u32,
            hash: s.hash.to_vec(),
            metadata: s.metadata,
        }
    };
    let checkpoint_exists = |s: &Snapshot| -> bool { Path::new(&s.path).exists() };

    response.snapshots = snapshots
        .into_iter()
        .filter(checkpoint_exists)
        .map(convert_snapshots)
        .collect();
    Ok(response)
}
