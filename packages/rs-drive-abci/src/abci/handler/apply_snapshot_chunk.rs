use crate::abci::app::StateSyncApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::snapshot::{MAX_STATE_SYNC_CHUNK_ID_SIZE, MAX_STATE_SYNC_CHUNK_SIZE};
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_apply_snapshot_chunk;

/// Applies one chunk of a state sync snapshot to the grovedb sync session.
///
/// A chunk grovedb rejects does not kill the whole transfer: Tenderdash is asked to
/// refetch that chunk (from a different peer, if it identified the sender). When the
/// last chunk lands, the session is committed, grovedb is verified against the target
/// app hash, and the platform state is reconstructed from the reduced platform state
/// contained in the restored snapshot.
pub fn apply_snapshot_chunk<'a, 'db: 'a, A, C>(
    app: &'a A,
    request: proto::RequestApplySnapshotChunk,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
where
    A: StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike + 'db,
{
    tracing::trace!(
        chunk_id = hex::encode(&request.chunk_id),
        chunk_len = request.chunk.len(),
        "[state_sync] api apply_snapshot_chunk",
    );

    // Cap peer-supplied sizes before anything decodes them (issue #3773)
    if request.chunk_id.len() > MAX_STATE_SYNC_CHUNK_ID_SIZE {
        return Err(AbciError::StateSyncBadRequest(format!(
            "apply_snapshot_chunk chunk id of {} bytes exceeds the {} byte limit",
            request.chunk_id.len(),
            MAX_STATE_SYNC_CHUNK_ID_SIZE
        ))
        .into());
    }
    if request.chunk.len() > MAX_STATE_SYNC_CHUNK_SIZE {
        return Err(AbciError::StateSyncBadRequest(format!(
            "apply_snapshot_chunk chunk of {} bytes exceeds the {} byte limit",
            request.chunk.len(),
            MAX_STATE_SYNC_CHUNK_SIZE
        ))
        .into());
    }

    let platform_version = app.platform().state.load().current_platform_version()?;
    let grove_version = &platform_version.drive.grove_version;

    let mut session_write_guard = app.snapshot_fetching_session().write().map_err(|_| {
        AbciError::StateSyncInternalError(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        )
    })?;

    {
        let session = session_write_guard
            .as_mut()
            .ok_or(AbciError::StateSyncBadRequest(
                "apply_snapshot_chunk no state sync session in progress".to_string(),
            ))?;

        let wire_version = session.wire_version;
        let next_chunk_ids = match session.state_sync_info.apply_chunk(
            &request.chunk_id,
            &request.chunk,
            wire_version,
            grove_version,
        ) {
            Ok(next_chunk_ids) => next_chunk_ids,
            Err(e) => {
                let reject_senders = if request.sender.is_empty() {
                    vec![]
                } else {
                    vec![request.sender.clone()]
                };

                // grovedb removes a chunk id from its pending set before processing it,
                // so a chunk it has already seen (e.g. the refetch of one it rejected)
                // cannot be re-applied within this session: ask Tenderdash to restart
                // the snapshot instead (a same-height re-offer, which we accept).
                // The string match is brittle by necessity (grovedb only exposes
                // InternalError(String) here); if the wording ever changes, the fallback
                // below is still safe — Tenderdash retries the chunk until it gives up
                // and restarts the snapshot itself.
                if matches!(&e, drive::grovedb::Error::InternalError(message) if message.contains("not expected"))
                {
                    tracing::warn!(
                        chunk_id = hex::encode(&request.chunk_id),
                        sender = request.sender,
                        error = ?e,
                        "[state_sync] apply_snapshot_chunk cannot re-apply a chunk in this session, requesting snapshot restart",
                    );
                    return Ok(proto::ResponseApplySnapshotChunk {
                        result: response_apply_snapshot_chunk::Result::RetrySnapshot.into(),
                        refetch_chunks: vec![],
                        reject_senders,
                        next_chunks: vec![],
                    });
                }

                // A chunk grovedb cannot apply (corrupted or tampered data) is
                // recoverable: keep the session and ask Tenderdash to refetch the chunk,
                // banning the peer that sent it so the refetch goes elsewhere.
                tracing::warn!(
                    chunk_id = hex::encode(&request.chunk_id),
                    sender = request.sender,
                    error = ?e,
                    "[state_sync] apply_snapshot_chunk rejected a chunk, requesting refetch",
                );
                return Ok(proto::ResponseApplySnapshotChunk {
                    result: response_apply_snapshot_chunk::Result::Retry.into(),
                    refetch_chunks: vec![request.chunk_id],
                    reject_senders,
                    next_chunks: vec![],
                });
            }
        };

        if !session.state_sync_info.is_sync_completed() {
            return Ok(proto::ResponseApplySnapshotChunk {
                result: response_apply_snapshot_chunk::Result::Accept.into(),
                refetch_chunks: vec![],
                reject_senders: vec![],
                next_chunks: next_chunk_ids,
            });
        }

        if !next_chunk_ids.is_empty() {
            return Err(AbciError::StateSyncInternalError(
                "apply_snapshot_chunk session is completed but next_chunk_ids is not empty"
                    .to_string(),
            )
            .into());
        }
    }

    // The transfer is complete: consume the session and commit it
    let session = session_write_guard
        .take()
        .expect("session presence was just checked");

    app.platform()
        .drive
        .grove
        .commit_session(session.state_sync_info, grove_version)
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk unable to commit session: {}",
                e
            ))
        })?;

    tracing::debug!("[state_sync] transfer complete, verifying grovedb");

    let incorrect_hashes = app
        .platform()
        .drive
        .grove
        .verify_grovedb(None, true, false, grove_version)
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk unable to verify grovedb: {}",
                e
            ))
        })?;
    if !incorrect_hashes.is_empty() {
        let paths: Vec<String> = incorrect_hashes
            .keys()
            .take(5)
            .map(|path| path.iter().map(hex::encode).collect::<Vec<_>>().join("/"))
            .collect();
        return Err(AbciError::StateSyncInternalError(format!(
            "apply_snapshot_chunk grovedb verification failed with {} incorrect hashes, first paths: [{}]",
            incorrect_hashes.len(),
            paths.join(", ")
        ))
        .into());
    }

    // Rebuild the in-memory platform state from the reduced platform state contained in
    // the restored snapshot. This re-derives masternode lists and quorums from Core and
    // must leave the grovedb root hash untouched; the equality check below proves it.
    app.platform()
        .reconstruct_platform_state(&session.app_hash, platform_version)?;

    let drive_app_hash = app
        .platform()
        .drive
        .grove
        .root_hash(None, grove_version)
        .unwrap()
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk unable to get app hash: {}",
                e
            ))
        })?;

    if drive_app_hash != session.app_hash {
        tracing::error!(
            state_sync_app_hash = hex::encode(session.app_hash),
            drive_app_hash = hex::encode(drive_app_hash),
            "[state_sync] restored grovedb root hash does not match the snapshot app hash",
        );
        return Err(AbciError::StateSyncInternalError(format!(
            "apply_snapshot_chunk grovedb verification failed with incorrect app hash: {}",
            hex::encode(drive_app_hash)
        ))
        .into());
    }

    tracing::info!(
        height = session.snapshot.height,
        app_hash = hex::encode(session.app_hash),
        "state_sync completed",
    );

    Ok(proto::ResponseApplySnapshotChunk {
        result: response_apply_snapshot_chunk::Result::CompleteSnapshot.into(),
        refetch_chunks: vec![],
        reject_senders: vec![],
        next_chunks: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::FullAbciApplication;
    use crate::abci::handler::offer_snapshot;
    use crate::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn apply_snapshot_chunk_without_session_is_rejected() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);

        assert!(apply_snapshot_chunk(
            &app,
            proto::RequestApplySnapshotChunk {
                chunk_id: vec![1u8; 32],
                chunk: vec![],
                sender: String::new(),
            },
        )
        .is_err());
    }

    #[test]
    fn apply_snapshot_chunk_caps_sizes_before_decoding() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);

        assert!(apply_snapshot_chunk(
            &app,
            proto::RequestApplySnapshotChunk {
                chunk_id: vec![0u8; MAX_STATE_SYNC_CHUNK_ID_SIZE + 1],
                chunk: vec![],
                sender: String::new(),
            },
        )
        .is_err());

        assert!(apply_snapshot_chunk(
            &app,
            proto::RequestApplySnapshotChunk {
                chunk_id: vec![1u8; 32],
                chunk: vec![0u8; MAX_STATE_SYNC_CHUNK_SIZE + 1],
                sender: String::new(),
            },
        )
        .is_err());
    }

    #[test]
    fn apply_snapshot_chunk_asks_for_refetch_of_a_bad_chunk() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);

        let target_app_hash = vec![7u8; 32];
        offer_snapshot(
            &app,
            proto::RequestOfferSnapshot {
                snapshot: Some(proto::Snapshot {
                    height: 100,
                    version: 1,
                    hash: target_app_hash.clone(),
                    metadata: vec![],
                }),
                app_hash: target_app_hash.clone(),
            },
        )
        .expect("should accept offer");

        // Garbage bytes for the root chunk: grovedb rejects them, and the session must
        // survive with a Retry + refetch of exactly that chunk, banning the sender.
        let response = apply_snapshot_chunk(
            &app,
            proto::RequestApplySnapshotChunk {
                chunk_id: target_app_hash.clone(),
                chunk: vec![0xde, 0xad, 0xbe, 0xef],
                sender: "peer-1".to_string(),
            },
        )
        .expect("bad chunk should not error the session");

        assert_eq!(
            response.result,
            i32::from(response_apply_snapshot_chunk::Result::Retry)
        );
        assert_eq!(response.refetch_chunks, vec![target_app_hash]);
        assert_eq!(response.reject_senders, vec!["peer-1".to_string()]);
        assert!(
            app.snapshot_fetching_session.read().unwrap().is_some(),
            "the session must survive a bad chunk"
        );
    }
}
