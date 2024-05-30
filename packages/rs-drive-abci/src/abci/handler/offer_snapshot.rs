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
    if request.app_hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequest(
            "offer_snapshot invalid app_hash in request".to_string(),
        )));
    }

    let mut request_app_hash = [0u8; 32];
    request_app_hash.copy_from_slice(&request.app_hash);

    match request.snapshot {
        None => Err(Error::Abci(AbciError::BadRequest(
            "offer_snapshot missing snapshot in request".to_string(),
        ))),
        Some(offered_snapshot) => {
            match app.snapshot_fetching_session().write() {
                Ok(mut session_write) => {
                    // Now `session_write` is a mutable reference to the inner data
                    match *session_write {
                        Some(ref mut session) => {
                            // Access and modify `session` here
                            if offered_snapshot.height <= session.snapshot.height {
                                return Err(Error::Abci(
                                    AbciError::BadRequest(
                                        "offer_snapshot already syncing newest height"
                                            .to_string(),
                                    ),
                                ));
                            }

                            match app.platform().drive.grove.wipe() {
                                Ok(_) => {
                                    let response = proto::ResponseOfferSnapshot::default();

                                    match app.platform().drive.grove.start_snapshot_syncing(request_app_hash, CURRENT_STATE_SYNC_VERSION) {
                                        Ok((_, state_sync_info)) => {
                                            session.snapshot = offered_snapshot;
                                            session.app_hash = request.app_hash;
                                            session.state_sync_info = state_sync_info;

                                            Ok(response)
                                        }
                                        Err(e) => Err(Error::Abci(
                                            AbciError::BadRequest(format!(
                                                "offer_snapshot unable start_snapshot_syncing:{}",
                                                e
                                            )),
                                        )),
                                    }
                                }
                                Err(e) => Err(Error::Abci(
                                    AbciError::BadRequest(format!(
                                        "offer_snapshot unable to wipe grovedb:{}",
                                        e
                                    )),
                                )),
                            }
                        }
                        None => Err(Error::Abci(AbciError::BadRequest(
                            "offer_snapshot unable to lock session".to_string(),
                        ))),
                    }
                }
                Err(_poisoned) => {
                    Err(Error::Abci(AbciError::BadRequest(
                        "offer_snapshot unable to lock session (poisoned)".to_string(),
                    )))
                }
            }
        }
    }
}
