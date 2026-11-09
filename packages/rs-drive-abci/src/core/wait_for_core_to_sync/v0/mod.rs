use tenderdash_abci::CancellationToken;

use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use std::fmt::Debug;
use std::time::Duration;

const CORE_SYNC_STATUS_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Blocks execution until Core is synced, then checks that Core answers the credit pool balance
/// read block execution needs from protocol version 14.
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

    if cancel.is_cancelled() {
        return Ok(());
    }

    // From protocol version 14 every block reads Core's credit pool balance from a chain locked
    // block's coinbase (`getspecialtxes`). Core loads the consensus user's `rpcwhitelist` only
    // when Core itself starts, so a Core not restarted since its whitelist gained the method
    // answers every other call until that version activates and refuses every block after.
    // Ask once here, so such a node fails to start instead.
    let chain_lock = core_rpc.get_best_chain_lock()?;
    if let Err(error) = core_rpc.get_credit_pool_balance(chain_lock.block_height) {
        tracing::error!(
            ?error,
            "core cannot read the credit pool balance (getspecialtxes); restart core so it loads the rpc whitelist of drive's consensus user"
        );
        return Err(error.into());
    }

    Ok(())
}
