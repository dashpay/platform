use crate::abci::app::StateSyncApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::snapshot::{
    SnapshotFetchingSession, STATE_SYNC_SUBTREES_BATCH_SIZE, SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS,
};
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_offer_snapshot;

/// Handles a snapshot offered by Tenderdash during state sync.
///
/// Accepting an offer wipes the local grovedb and opens a grovedb state sync session
/// targeting the light-client-verified app hash. A later offer for a higher height
/// replaces a session already in progress (also answered with Accept); an offer for a
/// lower or equal height than the session in progress is rejected.
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

    let platform_version = app.platform().state.load().current_platform_version()?;

    let mut session_write_guard = app.snapshot_fetching_session().write().map_err(|_| {
        AbciError::StateSyncInternalError(
            "offer_snapshot unable to lock session (poisoned)".to_string(),
        )
    })?;

    if let Some(session) = session_write_guard.as_ref() {
        // An offer at the same height is a legitimate snapshot restart (Tenderdash's
        // RETRY_SNAPSHOT flow) and replaces the session; only strictly older offers are
        // rejected.
        if offered_snapshot.height < session.snapshot.height {
            return Err(AbciError::StateSyncBadRequest(format!(
                "offer_snapshot already syncing snapshot at height {}, offered height {} is older",
                session.snapshot.height, offered_snapshot.height
            ))
            .into());
        }
        tracing::warn!(
            current_height = session.snapshot.height,
            offered_height = offered_snapshot.height,
            "[state_sync] offer_snapshot replacing session in progress",
        );
    }

    // Both the fresh-session and the replace-session paths wipe grovedb, start a new
    // grovedb sync session, and answer Accept.
    app.platform().drive.grove.wipe().map_err(|e| {
        AbciError::StateSyncInternalError(format!("offer_snapshot unable to wipe grovedb: {}", e))
    })?;

    // The wipe destroyed the state every lazily-loaded Drive cache was built from. Left
    // in place, those caches would be silently merged into the RESTORED state and fork the
    // node: `ProtocolVersionsCache` in particular keeps a `loaded` flag, so
    // `load_if_needed` would never re-read the restored version counters and the next block
    // would write vote counts derived from the wiped chain instead. Resetting the counter
    // wholesale (rather than `clear_global_cache`) is deliberate — it also clears that
    // flag, so the cache reloads from the restored state on first use.
    //
    // `system_data_contracts` is deliberately NOT cleared: those are compiled-in,
    // version-keyed contracts that never come from grovedb.
    let drive = &app.platform().drive;
    *drive.cache.protocol_versions_counter.write() = Default::default();
    drive.cache.data_contracts.clear();
    *drive.cache.genesis_time_ms.write() = None;

    let state_sync_info = app
        .platform()
        .drive
        .grove
        .start_snapshot_syncing(
            request_app_hash,
            STATE_SYNC_SUBTREES_BATCH_SIZE,
            wire_version,
            &platform_version.drive.grove_version,
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

    fn offer_at(height: u64, version: u32) -> proto::RequestOfferSnapshot {
        proto::RequestOfferSnapshot {
            snapshot: Some(proto::Snapshot {
                height,
                version,
                hash: vec![7u8; 32],
                metadata: vec![],
            }),
            app_hash: vec![7u8; 32],
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

        // A strictly lower height while syncing is rejected
        assert!(offer_snapshot(&app, offer_at(50, 1)).is_err());

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
    }
}
