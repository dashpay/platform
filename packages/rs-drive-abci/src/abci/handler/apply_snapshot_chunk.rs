use crate::abci::app::{BlockExecutionApplication, PlatformApplication, StateSyncApplication, TransactionalApplication};
use crate::error::Error;
use tenderdash_abci::proto::abci as proto;
use crate::abci::AbciError;
use crate::platform_types::snapshot::SnapshotFetchingSession;

pub fn apply_snapshot_chunk<'a, A, C: 'a>(
    app: &'a A,
    request: proto::RequestApplySnapshotChunk,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
    where
        A: PlatformApplication<C> + TransactionalApplication<'a> + StateSyncApplication<'a> + BlockExecutionApplication,
{

    let mut session_write_guard = app.snapshot_fetching_session().write().unwrap();

    match session_write_guard.as_mut() {
        Some(session) => {
            // Use the transaction as a reference and consume the session
            match app.platform().drive.grove.apply_chunk(
                &mut session.state_sync_info,
                (&request.chunk_id, request.chunk),
                1u16
            ) {
                Ok(next_chunk_ids) => {
                    return Ok(proto::ResponseApplySnapshotChunk {
                        result: proto::response_apply_snapshot_chunk::Result::Accept.into(),
                        refetch_chunks: vec![],
                        reject_senders: vec![],
                        next_chunks: next_chunk_ids,
                    });
                }
                Err(e) => {
                    return Err(Error::Abci(AbciError::BadRequest(
                        format!("offer_snapshot unable to wipe grovedb:{}", e)
                    )));
                }
            }
        }
        None => {
            // Handle the case where there is no transaction
            return Err(Error::Abci(AbciError::BadRequest(
                "offer_snapshot unable to lock session".to_string()
            )));
        }
    }
}
