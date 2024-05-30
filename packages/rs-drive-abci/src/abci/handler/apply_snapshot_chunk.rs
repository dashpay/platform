use tenderdash_abci::proto::abci as proto;

use crate::abci::app::{SnapshotManagerApplication, StateSyncApplication};
use crate::abci::AbciError;
use crate::error::Error;

pub fn apply_snapshot_chunk<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    request: proto::RequestApplySnapshotChunk,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
{
    let mut is_state_sync_completed: bool = false;
    match app.snapshot_fetching_session().write() {
        Ok(mut session_write_guard) => {
            match session_write_guard.as_mut() {
                Some(session) => {
                    match session.state_sync_info.apply_chunk(
                        &app.platform().drive.grove,
                        (&request.chunk_id, request.chunk),
                        1u16,
                    ) {
                        Ok(next_chunk_ids) => {
                            if next_chunk_ids.is_empty() {
                                if session.state_sync_info.is_sync_completed() {
                                    is_state_sync_completed = true;
                                }
                            }
                            if !is_state_sync_completed {
                                return Ok(proto::ResponseApplySnapshotChunk {
                                    result: proto::response_apply_snapshot_chunk::Result::Accept
                                        .into(),
                                    refetch_chunks: vec![],
                                    reject_senders: vec![],
                                    next_chunks: next_chunk_ids,
                                });
                            }
                        }
                        Err(e) => {
                            return Err(Error::Abci(AbciError::BadRequest(format!(
                                "apply_snapshot_chunk unable to apply chunk:{}",
                                e
                            ))))
                        }
                    }
                }
                None => {
                    // Handle the case where there is no transaction
                    return Err(Error::Abci(AbciError::BadRequest(
                        "apply_snapshot_chunk unable to lock session".to_string(),
                    )));
                }
            }
            // state_sync is completed (is_state_sync_completed is true) we need to commit the transaction
            match session_write_guard.take() {
                None => Err(Error::Abci(AbciError::BadRequest(
                    "apply_snapshot_chunk unable to lock session (for commit)".to_string(),
                ))),
                Some(session) => {
                    let state_sync_info = session.state_sync_info;
                    app.platform().drive.grove.commit_session(state_sync_info);
                    return Ok(proto::ResponseApplySnapshotChunk {
                        result: proto::response_apply_snapshot_chunk::Result::Accept.into(),
                        refetch_chunks: vec![],
                        reject_senders: vec![],
                        next_chunks: vec![],
                    });
                }
            }
        }
        Err(_poisoned) => {
            return Err(Error::Abci(AbciError::BadRequest(
                "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
            )))
        }
    }
}
