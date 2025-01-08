use crate::abci::app::{PlatformApplication, SnapshotManagerApplication};
use drive::grovedb::GroveDb;
use tenderdash_abci::proto::abci as proto;
//use platform_version::version::PlatformVersion;
use crate::abci::handler::error::error_into_exception;
use crate::abci::AbciError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

pub fn load_snapshot_chunk<A, C>(
    app: &A,
    request: proto::RequestLoadSnapshotChunk,
) -> Result<proto::ResponseLoadSnapshotChunk, Error>
where
    A: SnapshotManagerApplication + PlatformApplication<C>,
    C: CoreRPCLike,
{
    tracing::trace!(
        "[state_sync] api load_snapshot_chunk height:{} chunk_id:{}",
        request.height,
        hex::encode(&request.chunk_id)
    );
    let matched_snapshot = app
        .snapshot_manager()
        .get_snapshot_at_height(&*app.platform().drive.grove, request.height as i64)
        .map_err(|_| AbciError::StateSyncInternalError("load_snapshot_chunk failed".to_string()))?
        .ok_or_else(|| {
            AbciError::StateSyncInternalError("load_snapshot_chunk failed".to_string())
        })?;
    let db = GroveDb::open(&matched_snapshot.path).map_err(|e| {
        AbciError::StateSyncInternalError(format!("load_snapshot_chunk failed:{}", e))
    })?;
    let chunk = db
        .fetch_chunk(
            &request.chunk_id,
            None,
            request.version as u16,
            &PlatformVersion::latest().drive.grove_version,
        )
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!("load_snapshot_chunk failed:{}", e))
        })?;
    let mut response = proto::ResponseLoadSnapshotChunk::default();
    response.chunk = chunk;
    Ok(response)
}
