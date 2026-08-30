use crate::abci::app::StateSyncApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::snapshot::{
    clear_restore_sentinel_best_effort, wipe_drive_for_restore, MAX_STATE_SYNC_CHUNK_ID_SIZE,
    MAX_STATE_SYNC_CHUNK_SIZE,
};
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

    let mut session_write_guard = app.snapshot_fetching_session().write().map_err(|_| {
        AbciError::StateSyncInternalError(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        )
    })?;

    // The version the SNAPSHOT was produced at, pinned when the offer was accepted. Using
    // `self.state`'s version here would be wrong: a state-syncing node has no saved state,
    // so it is still on the initial protocol version and would decode, verify and hash the
    // restored trees under a different (older) grovedb table than the one that generated
    // the chunks.
    let platform_version = session_write_guard
        .as_ref()
        .ok_or(AbciError::StateSyncBadRequest(
            "apply_snapshot_chunk no state sync session in progress".to_string(),
        ))?
        .platform_version;
    let grove_version = &platform_version.drive.grove_version;

    {
        let session = session_write_guard
            .as_mut()
            .expect("session presence was just checked");

        let reject_senders = if request.sender.is_empty() {
            vec![]
        } else {
            vec![request.sender.clone()]
        };

        // Cap peer-supplied sizes before anything decodes them (issue #3773).
        //
        // These are TRANSFER faults, not reasons to abort state sync: an application
        // error here would reach Tenderdash as an ABCI exception, killing the whole
        // restore and leaving the node on the wiped database the offer created (with the
        // restore sentinel still set). Both caps are therefore answered with the same
        // recoverable ladder the other malformed-chunk paths use.
        if request.chunk.len() > MAX_STATE_SYNC_CHUNK_SIZE {
            // Oversized chunk DATA: the chunk id itself is still fine, so ban the sender
            // and have Tenderdash refetch exactly this chunk from someone else.
            tracing::warn!(
                chunk_id = hex::encode(&request.chunk_id),
                sender = request.sender,
                chunk_len = request.chunk.len(),
                limit = MAX_STATE_SYNC_CHUNK_SIZE,
                "[state_sync] apply_snapshot_chunk oversized chunk, rejecting the sender and requesting refetch",
            );
            return Ok(proto::ResponseApplySnapshotChunk {
                result: response_apply_snapshot_chunk::Result::Retry.into(),
                refetch_chunks: vec![request.chunk_id],
                reject_senders,
                next_chunks: vec![],
            });
        }
        if request.chunk_id.len() > MAX_STATE_SYNC_CHUNK_ID_SIZE {
            // An oversized chunk id is intrinsically invalid — refetching it would ask for
            // the same impossible id again — so restart the snapshot instead.
            tracing::warn!(
                chunk_id_len = request.chunk_id.len(),
                sender = request.sender,
                limit = MAX_STATE_SYNC_CHUNK_ID_SIZE,
                "[state_sync] apply_snapshot_chunk oversized chunk id, requesting snapshot restart",
            );
            return Ok(proto::ResponseApplySnapshotChunk {
                result: response_apply_snapshot_chunk::Result::RetrySnapshot.into(),
                refetch_chunks: vec![],
                reject_senders,
                next_chunks: vec![],
            });
        }

        let wire_version = session.wire_version;
        let next_chunk_ids = match session.state_sync_info.apply_chunk(
            &request.chunk_id,
            &request.chunk,
            wire_version,
            grove_version,
        ) {
            Ok(next_chunk_ids) => next_chunk_ids,
            Err(e) => {
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

    // grovedb only makes the session durable once its own root-hash check passes, so a
    // failure here leaves nothing committed — but the database is still WIPED from the
    // offer, so the node cannot be left as it is. Route it through the same recovery path
    // as every later failure, which also keeps Tenderdash's snapshot ladder moving instead
    // of aborting state sync with an exception.
    if let Err(e) = app
        .platform()
        .drive
        .grove
        .commit_session(session.state_sync_info, grove_version)
    {
        return reject_restored_snapshot(app, &format!("unable to commit the session: {}", e));
    }

    tracing::debug!("[state_sync] transfer complete, verifying grovedb");

    // From here on the session is COMMITTED: grovedb durably holds the restored state
    // while the platform state still describes the node from before the sync. Every
    // failure below must therefore go through `reject_restored_snapshot`, which puts the
    // node back to an empty, self-consistent slate and asks Tenderdash for another
    // snapshot. Returning an error instead would leave the node holding a database its
    // platform state knows nothing about, and the `info` handler panics on exactly that
    // mismatch — a crash loop that no restart can clear.
    let incorrect_hashes =
        match app
            .platform()
            .drive
            .grove
            .verify_grovedb(None, true, false, grove_version)
        {
            Ok(incorrect_hashes) => incorrect_hashes,
            Err(e) => {
                return reject_restored_snapshot(
                    app,
                    &format!("unable to verify the restored grovedb: {}", e),
                );
            }
        };
    if !incorrect_hashes.is_empty() {
        let paths: Vec<String> = incorrect_hashes
            .keys()
            .take(5)
            .map(|path| path.iter().map(hex::encode).collect::<Vec<_>>().join("/"))
            .collect();
        return reject_restored_snapshot(
            app,
            &format!(
                "grovedb verification failed with {} incorrect hashes, first paths: [{}]",
                incorrect_hashes.len(),
                paths.join(", ")
            ),
        );
    }

    // Rebuild the in-memory platform state from the reduced platform state contained in
    // the restored snapshot. This re-derives masternode lists and quorums from Core and
    // must leave the grovedb root hash untouched; the equality check below proves it.
    //
    // This is also where a snapshot taken before the reduced platform state existed
    // (pre-v15) is refused. Refusing earlier would be better, but grovedb does not expose
    // the session's transaction, so the Misc tree cannot be probed before the commit —
    // see the note on `reject_restored_snapshot`.
    if let Err(e) = app
        .platform()
        .reconstruct_platform_state(&session.app_hash, platform_version)
    {
        return reject_restored_snapshot(
            app,
            &format!("unable to reconstruct the platform state: {}", e),
        );
    }

    let drive_app_hash = match app
        .platform()
        .drive
        .grove
        .root_hash(None, grove_version)
        .unwrap()
    {
        Ok(drive_app_hash) => drive_app_hash,
        Err(e) => {
            return reject_restored_snapshot(
                app,
                &format!("unable to get the restored app hash: {}", e),
            );
        }
    };

    if drive_app_hash != session.app_hash {
        tracing::error!(
            state_sync_app_hash = hex::encode(session.app_hash),
            drive_app_hash = hex::encode(drive_app_hash),
            "[state_sync] restored grovedb root hash does not match the snapshot app hash",
        );
        return reject_restored_snapshot(
            app,
            &format!(
                "grovedb verification failed with incorrect app hash: {}",
                hex::encode(drive_app_hash)
            ),
        );
    }

    // The restore is complete and the node is self-consistent again, so the marker that
    // tells a restarting process to wipe can go. This is deliberately the LAST step, after
    // `reconstruct_platform_state` has committed the platform state to aux storage, and
    // deliberately best-effort: a successful restore must not be turned into an ABCI error
    // by a `remove_file` hiccup.
    clear_restore_sentinel_best_effort(&app.platform().config.db_path);

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

/// Puts the node back to an empty, self-consistent slate after a restore that was already
/// committed to grovedb turned out to be unusable, and asks Tenderdash to try a different
/// snapshot.
///
/// Ideally an unusable snapshot would be detected BEFORE `commit_session`, by probing the
/// Misc tree through the session's still-open transaction. grovedb keeps that transaction
/// private (`MultiStateSyncSession::transaction`, no accessor), so there is no way to read
/// the restored state before it lands. Until grovedb exposes it, this is the containment:
/// undo the commit by wiping, and let Tenderdash pick another snapshot.
///
/// The restore sentinel is deliberately LEFT IN PLACE. The database is empty, but the
/// in-memory platform state may still describe the chain the offer wiped, so the node is
/// not yet provably consistent. Everything that can happen next resolves it: another
/// `offer_snapshot` re-wipes and re-marks, a successful restore clears it, an `init_chain`
/// clears it, and a restart before any of those wipes and comes up empty.
///
/// `REJECT_SNAPSHOT` rather than an error is what keeps Tenderdash walking its ladder: it
/// discards this snapshot, tries the next, and falls back to block sync when it runs out.
/// An ABCI exception here would abort state sync altogether.
fn reject_restored_snapshot<'a, 'db: 'a, A, C>(
    app: &'a A,
    reason: &str,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
where
    A: StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike + 'db,
{
    tracing::error!(
        reason,
        "[state_sync] restored snapshot is unusable, wiping and asking for another one",
    );

    wipe_drive_for_restore(&app.platform().drive).map_err(|e| {
        AbciError::StateSyncInternalError(format!(
            "apply_snapshot_chunk unable to wipe after rejecting a snapshot ({}): {}",
            reason, e
        ))
    })?;

    Ok(proto::ResponseApplySnapshotChunk {
        result: response_apply_snapshot_chunk::Result::RejectSnapshot.into(),
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
    use crate::platform_types::snapshot::encode_snapshot_metadata;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::v15::PROTOCOL_VERSION_15;

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

    fn offer_a_snapshot(app: &FullAbciApplication<crate::rpc::core::MockCoreRPCLike>) -> Vec<u8> {
        let target_app_hash = vec![7u8; 32];
        offer_snapshot(
            app,
            proto::RequestOfferSnapshot {
                snapshot: Some(proto::Snapshot {
                    height: 100,
                    version: 1,
                    hash: target_app_hash.clone(),
                    metadata: encode_snapshot_metadata(PROTOCOL_VERSION_15),
                }),
                app_hash: target_app_hash.clone(),
            },
        )
        .expect("should accept offer");
        target_app_hash
    }

    /// An oversized chunk or chunk id is a recoverable transfer fault, not a reason to
    /// abort state sync with an ABCI exception: the session must survive and Tenderdash
    /// must be given a way forward.
    #[test]
    fn apply_snapshot_chunk_caps_sizes_without_aborting_the_transfer() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);
        let target_app_hash = offer_a_snapshot(&app);

        // Oversized chunk data: ban the sender and refetch exactly this chunk
        let response = apply_snapshot_chunk(
            &app,
            proto::RequestApplySnapshotChunk {
                chunk_id: target_app_hash.clone(),
                chunk: vec![0u8; MAX_STATE_SYNC_CHUNK_SIZE + 1],
                sender: "fat-peer".to_string(),
            },
        )
        .expect("an oversized chunk must not abort the ABCI request");
        assert_eq!(
            response.result,
            i32::from(response_apply_snapshot_chunk::Result::Retry)
        );
        assert_eq!(response.refetch_chunks, vec![target_app_hash]);
        assert_eq!(response.reject_senders, vec!["fat-peer".to_string()]);
        assert!(
            app.snapshot_fetching_session.read().unwrap().is_some(),
            "the session must survive an oversized chunk"
        );

        // Oversized chunk id: intrinsically invalid, so restart the snapshot
        let response = apply_snapshot_chunk(
            &app,
            proto::RequestApplySnapshotChunk {
                chunk_id: vec![0u8; MAX_STATE_SYNC_CHUNK_ID_SIZE + 1],
                chunk: vec![],
                sender: "fat-peer".to_string(),
            },
        )
        .expect("an oversized chunk id must not abort the ABCI request");
        assert_eq!(
            response.result,
            i32::from(response_apply_snapshot_chunk::Result::RetrySnapshot)
        );
        assert!(response.refetch_chunks.is_empty());
        assert_eq!(response.reject_senders, vec!["fat-peer".to_string()]);
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
                    metadata: encode_snapshot_metadata(PROTOCOL_VERSION_15),
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
