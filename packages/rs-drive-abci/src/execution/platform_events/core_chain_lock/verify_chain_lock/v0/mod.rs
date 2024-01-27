use crate::error::Error;
use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
use crate::execution::platform_events::core_chain_lock::verify_chain_lock::VerifyChainLockResult;
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;
use std::thread::sleep;
use std::time::Duration;

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;

use crate::rpc::core::CoreRPCLike;

const CORE_ALMOST_SYNCED_RETRIES: u32 = 5;
const CORE_ALMOST_SYNCED_SLEEP_TIME: u64 = 200;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn verify_chain_lock_v0(
        &self,
        round: u32,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        make_sure_core_is_synced: bool,
        platform_version: &PlatformVersion,
    ) -> Result<VerifyChainLockResult, Error> {
        // first we try to verify the chain lock locally
        match self.verify_chain_lock_locally(round, platform_state, chain_lock, platform_version) {
            Ok(Some(valid)) => {
                if valid && make_sure_core_is_synced {
                    match self.make_sure_core_is_synced_to_chain_lock(chain_lock, platform_version)
                    {
                        Ok(sync_status) => {
                            match sync_status {
                                CoreSyncStatus::CoreIsSynced => Ok(VerifyChainLockResult {
                                    chain_lock_signature_is_deserializable: true,
                                    found_valid_locally: Some(true),
                                    found_valid_by_core: None,
                                    core_is_synced: Some(true),
                                }),
                                CoreSyncStatus::CoreAlmostSynced => {
                                    for _i in 0..CORE_ALMOST_SYNCED_RETRIES {
                                        // The chain lock is valid we just need to sleep a bit and retry
                                        sleep(Duration::from_millis(CORE_ALMOST_SYNCED_SLEEP_TIME));
                                        let best_chain_locked =
                                            self.core_rpc.get_best_chain_lock()?;
                                        if best_chain_locked.block_height >= chain_lock.block_height
                                        {
                                            return Ok(VerifyChainLockResult {
                                                chain_lock_signature_is_deserializable: true,
                                                found_valid_locally: Some(valid),
                                                found_valid_by_core: Some(true),
                                                core_is_synced: Some(true),
                                            });
                                        }
                                    }
                                    Ok(VerifyChainLockResult {
                                        chain_lock_signature_is_deserializable: true,
                                        found_valid_locally: Some(true),
                                        found_valid_by_core: Some(true),
                                        core_is_synced: Some(false),
                                    })
                                }
                                CoreSyncStatus::CoreNotSynced => Ok(VerifyChainLockResult {
                                    chain_lock_signature_is_deserializable: true,
                                    found_valid_locally: Some(valid),
                                    found_valid_by_core: Some(true),
                                    core_is_synced: Some(false),
                                }),
                            }
                        }
                        Err(Error::CoreRpc(..)) => {
                            //ToDO (important), separate errors from core, connection Errors -> Err, others should be part of the result
                            Ok(VerifyChainLockResult {
                                chain_lock_signature_is_deserializable: true,
                                found_valid_locally: Some(valid),
                                found_valid_by_core: Some(false),
                                core_is_synced: None, //we do not know
                            })
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Ok(VerifyChainLockResult {
                        chain_lock_signature_is_deserializable: true,
                        found_valid_locally: Some(valid),
                        found_valid_by_core: None,
                        core_is_synced: None,
                    })
                }
            }
            Ok(None) => {
                // we were not able to verify locally
                let (verified, status) = self.verify_chain_lock_through_core(
                    chain_lock,
                    make_sure_core_is_synced,
                    platform_version,
                )?;

                if let Some(sync_status) = status {
                    // if we had make_sure_core_is_synced set to true
                    match sync_status {
                        CoreSyncStatus::CoreIsSynced => Ok(VerifyChainLockResult {
                            chain_lock_signature_is_deserializable: true,
                            found_valid_locally: None,
                            found_valid_by_core: None,
                            core_is_synced: Some(true),
                        }),
                        CoreSyncStatus::CoreAlmostSynced => {
                            for _i in 0..CORE_ALMOST_SYNCED_RETRIES {
                                // The chain lock is valid we just need to sleep a bit and retry
                                sleep(Duration::from_millis(CORE_ALMOST_SYNCED_SLEEP_TIME));
                                let best_chain_locked = self.core_rpc.get_best_chain_lock()?;
                                if best_chain_locked.block_height >= chain_lock.block_height {
                                    return Ok(VerifyChainLockResult {
                                        chain_lock_signature_is_deserializable: true,
                                        found_valid_locally: None,
                                        found_valid_by_core: Some(true),
                                        core_is_synced: Some(true),
                                    });
                                }
                            }
                            Ok(VerifyChainLockResult {
                                chain_lock_signature_is_deserializable: true,
                                found_valid_locally: None,
                                found_valid_by_core: Some(true),
                                core_is_synced: Some(false),
                            })
                        }
                        CoreSyncStatus::CoreNotSynced => Ok(VerifyChainLockResult {
                            chain_lock_signature_is_deserializable: true,
                            found_valid_locally: None,
                            found_valid_by_core: Some(true),
                            core_is_synced: Some(false),
                        }),
                    }
                } else {
                    Ok(VerifyChainLockResult {
                        chain_lock_signature_is_deserializable: true,
                        found_valid_locally: None,
                        found_valid_by_core: Some(verified),
                        core_is_synced: None,
                    })
                }
            }
            Err(Error::BLSError(_)) => Ok(VerifyChainLockResult {
                chain_lock_signature_is_deserializable: false,
                found_valid_locally: None,
                found_valid_by_core: None,
                core_is_synced: None,
            }),
            Err(e) => Err(e),
        }
    }
}
