use tokio_util::sync::CancellationToken;

use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use std::fmt::Debug;
use std::time::Duration;

const CORE_SYNC_STATUS_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Blocks execution until Core is synced
pub fn wait_for_core_to_sync<C: CoreRPCLike + Debug>(
    core_rpc: &C,
    cancel: CancellationToken,
) -> Result<(), Error> {
    while !cancel.is_cancelled() {
        tracing::debug!(?core_rpc, "waiting for core rpc to start");
        let mn_sync_status = core_rpc.masternode_sync_status()?;

        if !mn_sync_status.is_synced || !mn_sync_status.is_blockchain_synced {
            std::thread::sleep(CORE_SYNC_STATUS_CHECK_TIMEOUT);

            tracing::info!("waiting for core to sync...");
        } else {
            break;
        }
    }

    Ok(())
}
