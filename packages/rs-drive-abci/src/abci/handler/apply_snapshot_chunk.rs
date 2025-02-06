use crate::abci::app::{SnapshotFetchingApplication, SnapshotManagerApplication};
use crate::abci::AbciError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use drive::grovedb_storage::rocksdb_storage::RocksDbStorage;
use tenderdash_abci::proto::abci as proto;

pub fn apply_snapshot_chunk<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    request: proto::RequestApplySnapshotChunk,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
where
    A: SnapshotManagerApplication + SnapshotFetchingApplication<'db, C> + 'db,
{
    tracing::trace!(
        "[state_sync] api apply_snapshot_chunk chunk_id:{}",
        hex::encode(&request.chunk_id)
    );
    let mut is_state_sync_completed: bool = false;
    // Lock first the RwLock
    let mut session_write_guard = app.snapshot_fetching_session().write().map_err(|_| {
        AbciError::StateSyncInternalError(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        )
    })?;
    {
        let session = session_write_guard
            .as_mut()
            .ok_or(AbciError::StateSyncInternalError(
                "apply_snapshot_chunk unable to lock session".to_string(),
            ))?;
        let next_chunk_ids = session
            .state_sync_info
            .apply_chunk(
                &app.platform().drive.grove,
                &request.chunk_id,
                &request.chunk,
                1u16,
                &PlatformVersion::latest().drive.grove_version,
            )
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "apply_snapshot_chunk unable to apply chunk:{}",
                    e
                ))
            })?;
        if next_chunk_ids.is_empty() && session.state_sync_info.is_sync_completed() {
            is_state_sync_completed = true;
        }
        tracing::debug!(
            is_state_sync_completed,
            "state_sync apply_snapshot_chunk",
        );
        if !is_state_sync_completed {
            return Ok(proto::ResponseApplySnapshotChunk {
                result: proto::response_apply_snapshot_chunk::Result::Accept.into(),
                refetch_chunks: vec![], // TODO: Check when this is needed
                reject_senders: vec![], // TODO: Check when this is needed
                next_chunks: next_chunk_ids,
            });
        }
    }
    {
        // State sync is completed, consume session and commit it
        let session = session_write_guard
            .take()
            .ok_or(AbciError::StateSyncInternalError(
                "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
            ))?;
        let state_sync_info = session.state_sync_info;
        app.platform()
            .drive
            .grove
            .commit_session(state_sync_info)
            .map_err(|e| {
                AbciError::StateSyncInternalError(
                    "apply_snapshot_chunk unable to commit session".to_string(),
                )
            })?;
        println!("[state_sync] state sync completed. verifying");
        let incorrect_hashes = app
            .platform()
            .drive
            .grove
            .verify_grovedb(
                None,
                true,
                false,
                &PlatformVersion::latest().drive.grove_version,
            )
            .map_err(|e| {
                AbciError::StateSyncInternalError(
                    "apply_snapshot_chunk unable to verify grovedb".to_string(),
                )
            })?;
        if incorrect_hashes.len() > 0 {
            Err(AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk grovedb verification failed with {} incorrect hashes",
                incorrect_hashes.len()
            )))?;
        }
        return Ok(proto::ResponseApplySnapshotChunk {
            result: proto::response_apply_snapshot_chunk::Result::CompleteSnapshot.into(),
            refetch_chunks: vec![],
            reject_senders: vec![],
            next_chunks: vec![],
        });
    }
}
