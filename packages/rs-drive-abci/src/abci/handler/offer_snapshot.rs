use tenderdash_abci::proto::abci as proto;
use drive::grovedb::replication::CURRENT_STATE_SYNC_VERSION;

use crate::abci::app::{SnapshotManagerApplication, StateSyncApplication};
use crate::abci::AbciError;
use crate::error::Error;

pub fn offer_snapshot<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    request: proto::RequestOfferSnapshot,
) -> Result<proto::ResponseOfferSnapshot, Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
{
    let request_app_hash: [u8; 32] = request.app_hash.try_into()
        .map_err(|_| AbciError::InvalidState(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        ))?;

    let offered_snapshot = request.snapshot
        .ok_or(AbciError::InvalidState(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        ))?;
    let mut session_write_guard = app.snapshot_fetching_session().write()
        .map_err(|_| AbciError::InvalidState(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        ))?;
    let session = session_write_guard.as_mut()
        .ok_or(AbciError::InvalidState(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        ))?;
    if offered_snapshot.height <= session.snapshot.height {
        return Err(Error::Abci(
            AbciError::BadRequest(
                "offer_snapshot already syncing newest height"
                    .to_string(),
            ),
        ));
    }
    app.platform().drive.grove.wipe()
        .map_err(|e|AbciError::InvalidState(
            format!(
                "offer_snapshot unable to wipe grovedb:{}",
                e
            )
        ))?;
    let (first_chunk_id, state_sync_info) = app.platform().drive.grove.start_snapshot_syncing(request_app_hash, CURRENT_STATE_SYNC_VERSION)
        .map_err(|e|AbciError::InvalidState(
            format!(
                "offer_snapshot unable to wipe grovedb:{}",
                e
            )
        ))?;
    session.snapshot = offered_snapshot;
    session.app_hash = request_app_hash.to_vec();
    session.state_sync_info = state_sync_info;

    let response = proto::ResponseOfferSnapshot::default();
    Ok(response)
}
