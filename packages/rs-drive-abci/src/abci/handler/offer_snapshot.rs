use crate::abci::app::{SnapshotManagerApplication, StateSyncApplication};
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::snapshot::SnapshotFetchingSession;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_offer_snapshot;

const SUBTREES_BATCH_SIZE: usize = 64;

pub fn offer_snapshot<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    request: proto::RequestOfferSnapshot,
) -> Result<proto::ResponseOfferSnapshot, Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
{
    let request_app_hash: [u8; 32] = request.app_hash.try_into().map_err(|_| {
        AbciError::StateSyncBadRequest("offer_snapshot invalid app_hash length".to_string())
    })?;
    let offered_snapshot = request.snapshot.ok_or(AbciError::StateSyncBadRequest(
        "offer_snapshot empty snapshot in request".to_string(),
    ))?;
    tracing::trace!(
        "[state_sync] api offer_snapshot height:{}",
        offered_snapshot.height
    );

    let platform_version = app.platform().state.load().current_platform_version()?;

    let mut session_write_guard = app.snapshot_fetching_session().write().map_err(|_| {
        AbciError::StateSyncInternalError(
            "offer_snapshot unable to lock session (poisoned)".to_string(),
        )
    })?;
    if session_write_guard.is_none() {
        // No session currently, start a new one.
        app.platform().drive.grove.wipe().map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "offer_snapshot unable to wipe grovedb:{}",
                e
            ))
        })?;
        let state_sync_info = app
            .platform()
            .drive
            .grove
            .start_snapshot_syncing(
                request_app_hash,
                SUBTREES_BATCH_SIZE,
                platform_version.drive_abci.state_sync.protocol_version,
                &platform_version.drive.grove_version,
            )
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "offer_snapshot unable to start snapshot syncing session:{}",
                    e
                ))
            })?;
        let session = SnapshotFetchingSession::new(
            offered_snapshot,
            request_app_hash.to_vec(),
            state_sync_info,
        );
        *session_write_guard = Some(session);
        let mut response = proto::ResponseOfferSnapshot::default();
        response.result = i32::from(response_offer_snapshot::Result::Accept);
        Ok(response)
    } else {
        // Already syncing another snapshot session
        let session = session_write_guard
            .as_mut()
            .ok_or(AbciError::StateSyncInternalError(
                "offer_snapshot unable to lock session".to_string(),
            ))?;
        tracing::warn!(
            "[state_sync] api offer_snapshot already syncing height:{}",
            session.snapshot.height
        );
        if offered_snapshot.height < session.snapshot.height {
            return Err(Error::Abci(AbciError::StateSyncBadRequest(
                "offer_snapshot already syncing newest height".to_string(),
            )));
        }
        app.platform().drive.grove.wipe().map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "offer_snapshot unable to wipe grovedb:{}",
                e
            ))
        })?;
        let state_sync_info = app
            .platform()
            .drive
            .grove
            .start_snapshot_syncing(
                request_app_hash,
                SUBTREES_BATCH_SIZE,
                platform_version.drive_abci.state_sync.protocol_version,
                &platform_version.drive.grove_version,
            )
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "offer_snapshot unable to start snapshot syncing session:{}",
                    e
                ))
            })?;
        session.snapshot = offered_snapshot;
        session.app_hash = request_app_hash.to_vec();
        session.state_sync_info = state_sync_info;
        let response = proto::ResponseOfferSnapshot::default();
        Ok(response)
    }
}
