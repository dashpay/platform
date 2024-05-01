use tenderdash_abci::CancellationToken;

use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use std::fmt::Debug;
use std::time::Duration;

const CORE_SYNC_STATUS_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Blocks execution until Core is synced
/// This isn't in consensus, however we still version it just in case we will upgrade it on a
/// version
pub fn wait_for_core_to_sync_v0<C: CoreRPCLike + Debug>(
    core_rpc: &C,
    cancel: CancellationToken,
) -> Result<(), Error> {
    tracing::info!(?core_rpc, "waiting for core rpc to start");

    while !cancel.is_cancelled() {
        let has_chain_locked = match core_rpc.get_best_chain_lock() {
            Ok(_) => true,
            Err(error) => {
                tracing::warn!(?error, "cannot get best chain lock");
                false
            }
        };

        let mn_sync_status = match core_rpc.masternode_sync_status() {
            Ok(status) => status,
            Err(error) => {
                tracing::warn!(?error, "cannot get masternode status, retrying");
                continue;
            }
        };

        if !has_chain_locked || !mn_sync_status.is_synced || !mn_sync_status.is_blockchain_synced {
            std::thread::sleep(CORE_SYNC_STATUS_CHECK_TIMEOUT);

            tracing::info!("waiting for core to sync...");
        } else {
            break;
        }
    }

    Ok(())
}
