use crate::abci::app::StateSyncApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::snapshot::{
    decode_snapshot_metadata, wipe_drive_for_restore, write_restore_sentinel,
    SnapshotFetchingSession, STATE_SYNC_SUBTREES_BATCH_SIZE,
    SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS,
};
use crate::rpc::core::CoreRPCLike;
use dpp::version::v15::PROTOCOL_VERSION_15;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_offer_snapshot;

/// Handles a snapshot offered by Tenderdash during state sync.
///
/// Accepting an offer wipes the local grovedb and opens a grovedb state sync session
/// targeting the light-client-verified app hash. Any accepted-format offer replaces a
/// session already in progress (also answered with Accept), whatever height it carries.
pub fn offer_snapshot<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    request: proto::RequestOfferSnapshot,
) -> Result<proto::ResponseOfferSnapshot, Error>
where
    A: StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike,
{
    let request_app_hash: [u8; 32] = request.app_hash.try_into().map_err(|_| {
        AbciError::StateSyncBadRequest("offer_snapshot invalid app_hash length".to_string())
    })?;
    let offered_snapshot = request.snapshot.ok_or(AbciError::StateSyncBadRequest(
        "offer_snapshot empty snapshot in request".to_string(),
    ))?;

    tracing::debug!(
        height = offered_snapshot.height,
        version = offered_snapshot.version,
        "[state_sync] api offer_snapshot",
    );

    // The grovedb wire version of the whole transfer is the OFFERED snapshot's version,
    // validated against the single supported set. Unsupported versions ask Tenderdash to
    // reject every snapshot of this format and try others.
    let wire_version = u16::try_from(offered_snapshot.version)
        .ok()
        .filter(|version| SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS.contains(version));
    let Some(wire_version) = wire_version else {
        tracing::warn!(
            height = offered_snapshot.height,
            version = offered_snapshot.version,
            supported = ?SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS,
            "[state_sync] offer_snapshot rejecting unsupported snapshot version",
        );
        return Ok(proto::ResponseOfferSnapshot {
            result: response_offer_snapshot::Result::RejectFormat.into(),
        });
    };

    // The Platform version of the SNAPSHOT, which is what every grovedb call of this
    // transfer must run under. It cannot come from `self.state`: a node that state syncs
    // has no saved state, so its in-memory platform state is still at the initial protocol
    // version and would hand grovedb the wrong (much older) version table than the one the
    // serving node generated the chunks with.
    let snapshot_platform_version =
        decode_snapshot_metadata(&offered_snapshot.metadata).and_then(|protocol_version| {
            // Only versions that write the reduced platform state can be restored at all;
            // anything else is refused here rather than after a full transfer.
            (protocol_version >= PROTOCOL_VERSION_15)
                .then(|| PlatformVersion::get(protocol_version).ok())
                .flatten()
        });
    let Some(snapshot_platform_version) = snapshot_platform_version else {
        tracing::warn!(
            height = offered_snapshot.height,
            metadata = hex::encode(&offered_snapshot.metadata),
            "[state_sync] offer_snapshot rejecting a snapshot without a usable platform version in its metadata",
        );
        return Ok(proto::ResponseOfferSnapshot {
            result: response_offer_snapshot::Result::Reject.into(),
        });
    };

    let mut session_write_guard = app.snapshot_fetching_session().write().map_err(|_| {
        AbciError::StateSyncInternalError(
            "offer_snapshot unable to lock session (poisoned)".to_string(),
        )
    })?;

    if let Some(session) = session_write_guard.as_ref() {
        // Every offer Tenderdash makes is Tenderdash resetting the transfer, so it always
        // replaces the session in progress — including one for a LOWER height.
        //
        // The height in a snapshot descriptor is peer-supplied and untrusted (only the
        // `app_hash` is light-client verified), so refusing to go backwards would hand a
        // peer a wedge: advertise a high snapshot, withhold its chunks, and Tenderdash's
        // fallback to an honest peer's older checkpoint would then be answered with an
        // ABCI exception that aborts state sync altogether. Replacing is safe because the
        // restore is only ever accepted against the verified app hash of whatever offer
        // won.
        tracing::warn!(
            current_height = session.snapshot.height,
            offered_height = offered_snapshot.height,
            "[state_sync] offer_snapshot replacing session in progress",
        );
    }

    // Mark the database as under restore BEFORE destroying it. From here until the
    // restore completes, the node may be in a state that cannot serve consensus, and the
    // only thing that can tell a restarted process so is this marker: without it, startup
    // finds a database that disagrees with its platform state and cannot distinguish an
    // interrupted restore from corruption. See `Platform::open_with_client`.
    write_restore_sentinel(
        &app.platform().config.db_path,
        &request_app_hash,
        offered_snapshot.height,
    )
    .map_err(|e| {
        AbciError::StateSyncInternalError(format!(
            "offer_snapshot unable to record the restore sentinel: {}",
            e
        ))
    })?;

    // Both the fresh-session and the replace-session paths wipe grovedb (dropping the
    // caches derived from it), start a new grovedb sync session, and answer Accept.
    wipe_drive_for_restore(&app.platform().drive).map_err(|e| {
        AbciError::StateSyncInternalError(format!("offer_snapshot unable to wipe grovedb: {}", e))
    })?;

    let state_sync_info = app
        .platform()
        .drive
        .grove
        .start_snapshot_syncing(
            request_app_hash,
            STATE_SYNC_SUBTREES_BATCH_SIZE,
            wire_version,
            &snapshot_platform_version.drive.grove_version,
        )
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "offer_snapshot unable to start snapshot syncing session: {}",
                e
            ))
        })?;

    *session_write_guard = Some(SnapshotFetchingSession {
        snapshot: offered_snapshot,
        app_hash: request_app_hash,
        wire_version,
        platform_version: snapshot_platform_version,
        state_sync_info,
    });

    Ok(proto::ResponseOfferSnapshot {
        result: response_offer_snapshot::Result::Accept.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::FullAbciApplication;
    use crate::test::helpers::setup::TestPlatformBuilder;

    use crate::platform_types::snapshot::encode_snapshot_metadata;

    fn offer_at(height: u64, version: u32) -> proto::RequestOfferSnapshot {
        offer_at_with_metadata(
            height,
            version,
            encode_snapshot_metadata(PROTOCOL_VERSION_15),
        )
    }

    fn offer_at_with_metadata(
        height: u64,
        version: u32,
        metadata: Vec<u8>,
    ) -> proto::RequestOfferSnapshot {
        proto::RequestOfferSnapshot {
            snapshot: Some(proto::Snapshot {
                height,
                version,
                hash: vec![7u8; 32],
                metadata,
            }),
            app_hash: vec![7u8; 32],
        }
    }

    /// The snapshot's Platform version drives every grovedb call of the transfer, so an
    /// offer that does not carry a usable one must be refused BEFORE the database is
    /// wiped — and refused as a per-snapshot Reject, so Tenderdash keeps walking its
    /// ladder instead of aborting state sync.
    #[test]
    fn offer_snapshot_rejects_offers_without_a_usable_platform_version() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);

        for metadata in [
            vec![],                                            // absent
            vec![0u8; 3],                                      // wrong length
            encode_snapshot_metadata(1),                       // pre-v15, cannot be restored
            encode_snapshot_metadata(u32::MAX),                // unknown version
            encode_snapshot_metadata(PROTOCOL_VERSION_15 - 1), // last version before v15
        ] {
            let response = offer_snapshot(&app, offer_at_with_metadata(100, 1, metadata.clone()))
                .expect("should not error");
            assert_eq!(
                response.result,
                i32::from(response_offer_snapshot::Result::Reject),
                "metadata {:?} must be rejected",
                metadata
            );
            assert!(
                app.snapshot_fetching_session.read().unwrap().is_none(),
                "a rejected offer must not open a session",
            );
        }
    }

    #[test]
    fn offer_snapshot_rejects_unsupported_version_with_reject_format() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);

        let response = offer_snapshot(&app, offer_at(100, 999)).expect("should not error");
        assert_eq!(
            response.result,
            i32::from(response_offer_snapshot::Result::RejectFormat)
        );
        assert!(app.snapshot_fetching_session.read().unwrap().is_none());
    }

    #[test]
    fn offer_snapshot_accepts_fresh_and_replacing_offers() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);

        // Fresh session is accepted
        let response = offer_snapshot(&app, offer_at(100, 1)).expect("should accept fresh offer");
        assert_eq!(
            response.result,
            i32::from(response_offer_snapshot::Result::Accept)
        );

        // A LOWER height while syncing is Tenderdash falling back to another available
        // snapshot after the higher one turned out to be unservable. It must replace the
        // session and be accepted, otherwise a peer that advertises a high snapshot and
        // then withholds its chunks could block the fallback.
        let response =
            offer_snapshot(&app, offer_at(50, 1)).expect("should accept an older fallback offer");
        assert_eq!(
            response.result,
            i32::from(response_offer_snapshot::Result::Accept)
        );
        assert_eq!(
            app.snapshot_fetching_session
                .read()
                .unwrap()
                .as_ref()
                .expect("session must exist")
                .snapshot
                .height,
            50,
        );

        // Bring the session back up to 100 for the restart check below
        offer_snapshot(&app, offer_at(100, 1)).expect("should accept offer");

        // A same-height re-offer is a snapshot restart (Tenderdash RETRY_SNAPSHOT):
        // the session is replaced and the offer accepted
        let response = offer_snapshot(&app, offer_at(100, 1)).expect("should accept restart");
        assert_eq!(
            response.result,
            i32::from(response_offer_snapshot::Result::Accept)
        );

        // A newer snapshot replaces the session and MUST also answer Accept
        // (the old prototype returned the default UNKNOWN result here)
        let response = offer_snapshot(&app, offer_at(200, 1)).expect("should accept newer offer");
        assert_eq!(
            response.result,
            i32::from(response_offer_snapshot::Result::Accept)
        );
        let session_guard = app.snapshot_fetching_session.read().unwrap();
        let session = session_guard.as_ref().expect("session must exist");
        assert_eq!(session.snapshot.height, 200);
        assert_eq!(session.wire_version, 1);
        assert_eq!(
            session.platform_version.protocol_version, PROTOCOL_VERSION_15,
            "the session must run under the SNAPSHOT's platform version",
        );
    }
}
