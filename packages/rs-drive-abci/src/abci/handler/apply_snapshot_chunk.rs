use std::pin::Pin;
use crate::abci::app::{BlockExecutionApplication, PlatformApplication, SnapshotManagerApplication, StateSyncApplication, TransactionalApplication};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::platform_types::cleaned_abci_messages::finalized_block_cleaned_request::v0::FinalizeBlockCleanedRequest;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use std::sync::atomic::Ordering;
use std::sync::LockResult;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::{ResponseApplySnapshotChunk, ResponseOfferSnapshot};
use drive::grovedb::replication::{CURRENT_STATE_SYNC_VERSION, MultiStateSyncSession};
use crate::abci::AbciError;
use crate::abci::handler::error::error_into_exception;

pub fn apply_snapshot_chunk<'a, A, C>(
    app: &A,
    request: proto::RequestApplySnapshotChunk,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
    where
        A: PlatformApplication<C>
        + SnapshotManagerApplication
        + StateSyncApplication<'a>,
        C: CoreRPCLike,
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
                            if (next_chunk_ids.is_empty()) {
                                if (session.state_sync_info.is_sync_completed()) {
                                    is_state_sync_completed = true;
                                }
                            }
                            if !is_state_sync_completed {
                                return Ok(proto::ResponseApplySnapshotChunk {
                                    result:
                                    proto::response_apply_snapshot_chunk::Result::Accept
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
                    )))
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
