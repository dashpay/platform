use tenderdash_abci::CancellationToken;

use crate::error::Error;
use crate::rpc::core::{is_http_forbidden, CoreRPCLike};
use dpp::dashcore::ProTxHash;
use std::fmt::Debug;
use std::time::Duration;

const CORE_SYNC_STATUS_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Blocks execution until Core is synced, then checks that Core answers the credit pool balance
/// read block execution needs from protocol version 14, and does not refuse the `protx info`
/// read it needs from protocol version 15.
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

    // From protocol version 15 a block reads, with `protx info`, the operator payout of a
    // masternode whose operator key changed, for the same reason refused until Core restarts
    // with a whitelist that names it. Asked for a masternode Core does not list, an allowed
    // `protx info` answers that it has none, and a refused one answers HTTP 403.
    if let Err(error) =
        core_rpc.get_masternode_state(&ProTxHash::all_zeros(), chain_lock.block_height)
    {
        if is_http_forbidden(&error) {
            tracing::error!(
                ?error,
                "core refuses protx info; restart core so it loads the rpc whitelist of drive's consensus user"
            );
            return Err(error.into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::wait_for_core_to_sync_v0;
    use crate::error::Error;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::dashcore::bls_sig_utils::BLSSignature;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::dashcore_rpc::dashcore_rpc_json::{MnSyncAssetName, MnSyncStatus};
    use dpp::dashcore_rpc::jsonrpc::error::{Error as JsonRpcError, RpcError};
    use dpp::dashcore_rpc::jsonrpc::simple_http::Error as HttpError;
    use dpp::dashcore_rpc::Error as CoreRpcError;
    use tenderdash_abci::CancellationToken;

    /// A synced Core that reads the credit pool balance, and answers `protx info` with
    /// `masternode_state`.
    fn synced_core(
        masternode_state: impl Fn() -> CoreRpcError + Send + 'static,
    ) -> MockCoreRPCLike {
        let mut core = MockCoreRPCLike::new();
        core.expect_get_best_chain_lock().returning(|| {
            Ok(ChainLock {
                block_height: 100,
                block_hash: BlockHash::all_zeros(),
                signature: BLSSignature::from([0; 96]),
            })
        });
        core.expect_masternode_sync_status().returning(|| {
            Ok(MnSyncStatus {
                asset_id: 999,
                asset_name: MnSyncAssetName::Finished,
                asset_start_time: 0,
                attempt: 0,
                is_blockchain_synced: true,
                is_synced: true,
            })
        });
        core.expect_get_credit_pool_balance().returning(|_| Ok(0));
        core.expect_get_masternode_state()
            .returning(move |_, _| Err(masternode_state()));
        core
    }

    /// Core answers `protx info` for a masternode it does not list with an error; drive-abci
    /// starts.
    #[test]
    fn should_start_when_core_answers_protx_info() {
        let core = synced_core(|| {
            CoreRpcError::JsonRpc(JsonRpcError::Rpc(RpcError {
                code: -8,
                message: "not found".to_string(),
                data: None,
            }))
        });
        assert!(wait_for_core_to_sync_v0(&core, CancellationToken::new()).is_ok());
    }

    /// A Core that has not loaded a whitelist naming `protx info` refuses it with HTTP 403, and
    /// drive-abci does not start, instead of failing the first protocol version 15 block with
    /// an operator key change.
    #[test]
    fn should_not_start_when_core_refuses_protx_info() {
        let core = synced_core(|| {
            CoreRpcError::JsonRpc(JsonRpcError::Transport(Box::new(HttpError::HttpErrorCode(
                403,
            ))))
        });
        assert!(matches!(
            wait_for_core_to_sync_v0(&core, CancellationToken::new()),
            Err(Error::CoreRpc(_))
        ));
    }
}
