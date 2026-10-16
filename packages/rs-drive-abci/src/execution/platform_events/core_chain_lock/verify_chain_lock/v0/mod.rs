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
                                CoreSyncStatus::Done => Ok(VerifyChainLockResult {
                                    chain_lock_signature_is_deserializable: true,
                                    found_valid_locally: Some(true),
                                    found_valid_by_core: None,
                                    core_is_synced: Some(true),
                                }),
                                CoreSyncStatus::Almost => {
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
                                CoreSyncStatus::Not => Ok(VerifyChainLockResult {
                                    chain_lock_signature_is_deserializable: true,
                                    found_valid_locally: Some(valid),
                                    found_valid_by_core: Some(true),
                                    core_is_synced: Some(false),
                                }),
                            }
                        }
                        Err(Error::CoreRpc(error)) => {
                            tracing::error!(
                                ?chain_lock,
                                ?error,
                                "Failed to submit chain lock: {}",
                                error
                            );

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
                        CoreSyncStatus::Done => Ok(VerifyChainLockResult {
                            chain_lock_signature_is_deserializable: true,
                            found_valid_locally: None,
                            found_valid_by_core: None,
                            core_is_synced: Some(true),
                        }),
                        CoreSyncStatus::Almost => {
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
                        CoreSyncStatus::Not => Ok(VerifyChainLockResult {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::chain_lock_quorums::{
        chain_lock_test_platform, sign_chain_lock, QuorumMinedAfterCoreHeight, TestQuorum,
    };
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::dashcore_rpc::Error as DashCoreRpcError;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_accept_through_core_chain_lock_signed_by_quorum_mined_after_last_committed_core_height(
    ) {
        // Core mined quorum n at 1114 and signed the proposed chain lock with it. Platform does
        // not hold n, so it hands the chain lock to Core, which verifies its signature and
        // already has it.
        let mut platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());
        let chain_lock = scenario.chain_lock_signed_by_new_quorum();
        let chain_lock_height = chain_lock.block_height;

        let mut core_rpc = MockCoreRPCLike::new();
        let asked = chain_lock.clone();
        core_rpc
            .expect_verify_chain_lock()
            .withf(move |chain_lock| *chain_lock == asked)
            .times(1)
            .returning(|_| Ok(true));
        core_rpc
            .expect_submit_chain_lock()
            .times(1)
            .returning(move |_| Ok(chain_lock_height));
        platform.core_rpc = core_rpc;

        let result = platform
            .platform
            .verify_chain_lock_v0(0, &state, &chain_lock, true, PlatformVersion::latest())
            .expect("verification runs");

        assert!(result.chain_lock_signature_is_deserializable);
        assert_eq!(result.found_valid_locally, None);
        assert_eq!(result.found_valid_by_core, None);
        assert_eq!(result.core_is_synced, Some(true));
    }

    #[test]
    fn should_reject_forged_chain_lock_without_asking_core_when_signing_height_quorums_are_known() {
        // The signing height is within what Platform holds the exact quorums for, so a chain
        // lock none of them signed is rejected locally; Core's answer, which skips the
        // signature check for heights it already has a chain lock for, is never asked.
        let mut platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());
        let chain_lock = sign_chain_lock(&TestQuorum::new(9, 0), 1113 + 8);

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc.expect_verify_chain_lock().times(0);
        core_rpc.expect_submit_chain_lock().times(0);
        platform.core_rpc = core_rpc;

        let result = platform
            .platform
            .verify_chain_lock_v0(0, &state, &chain_lock, true, PlatformVersion::latest())
            .expect("verification runs");

        assert!(result.chain_lock_signature_is_deserializable);
        assert_eq!(result.found_valid_locally, Some(false));
        assert_eq!(result.found_valid_by_core, None);
        assert_eq!(result.core_is_synced, None);
    }

    #[test]
    fn should_reject_forged_chain_lock_left_to_core_when_core_already_has_a_chain_lock_at_its_height(
    ) {
        // Platform cannot check these chain locks itself: at 1122 a quorum mined at 1114 may
        // have signed it, at 1136 the signing height is past the DKG interval of Platform's core
        // height 1106. Core already has a chain lock at 1140, so submitchainlock returns that
        // height without checking the signature; verifychainlock does check it and fails it.
        const CORE_BEST_CHAIN_LOCK_HEIGHT: u32 = 1140;
        let mut platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());

        for chain_lock_height in [1114 + 8, 1128 + 8] {
            let forged = sign_chain_lock(&TestQuorum::new(9, 0), chain_lock_height);

            let mut core_rpc = MockCoreRPCLike::new();
            let asked = forged.clone();
            core_rpc
                .expect_verify_chain_lock()
                .withf(move |chain_lock| *chain_lock == asked)
                .times(1)
                .returning(|_| Ok(false));
            core_rpc
                .expect_submit_chain_lock()
                .times(0)
                .returning(|_| Ok(CORE_BEST_CHAIN_LOCK_HEIGHT));
            platform.core_rpc = core_rpc;

            let result = platform
                .platform
                .verify_chain_lock_v0(0, &state, &forged, true, PlatformVersion::latest())
                .expect("verification runs");

            assert!(result.chain_lock_signature_is_deserializable);
            assert_eq!(
                result.found_valid_locally,
                Some(false),
                "a forged chain lock at {chain_lock_height} must be rejected, not accepted \
                 through Core's early return (core is synced: {:?})",
                result.core_is_synced
            );
            assert_eq!(result.found_valid_by_core, None);
            assert_eq!(
                result.core_is_synced, None,
                "submitchainlock must not be asked"
            );
        }
    }

    #[test]
    fn should_fail_verification_of_chain_lock_left_to_core_when_verifychainlock_fails() {
        // The block proposal turns this error into an invalid chain lock, as it does when
        // submitchainlock fails.
        let mut platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());
        let chain_lock = scenario.chain_lock_signed_by_new_quorum();

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc.expect_verify_chain_lock().times(1).returning(|_| {
            Err(DashCoreRpcError::UnexpectedStructure(
                "simulated rpc failure".to_string(),
            ))
        });
        core_rpc.expect_submit_chain_lock().times(0);
        platform.core_rpc = core_rpc;

        let result = platform.platform.verify_chain_lock_v0(
            0,
            &state,
            &chain_lock,
            true,
            PlatformVersion::latest(),
        );

        assert!(
            matches!(result, Err(Error::CoreRpc(_))),
            "the verifychainlock failure must reach the block proposal"
        );
    }

    /// Exercises the `Err(Error::BLSError(_))` branch of `verify_chain_lock_v0`:
    /// when the inner `verify_chain_lock_locally` returns a BLS deserialization
    /// error, `verify_chain_lock_v0` must swallow it and return a
    /// `VerifyChainLockResult` with `chain_lock_signature_is_deserializable: false`
    /// and all other fields `None`.
    #[test]
    fn verify_chain_lock_v0_invalid_signature_yields_not_deserializable_result() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.platform.state.load();
        let platform_version = PlatformVersion::latest();

        let bad_chain_lock = ChainLock {
            block_height: 1,
            block_hash: BlockHash::from_byte_array([9u8; 32]),
            signature: [0u8; 96].into(),
        };

        // make_sure_core_is_synced=false avoids hitting core_rpc paths.
        let result = platform
            .platform
            .verify_chain_lock_v0(0, &platform_state, &bad_chain_lock, false, platform_version)
            .expect("BLS error must be wrapped into Ok(VerifyChainLockResult)");

        assert!(!result.chain_lock_signature_is_deserializable);
        assert!(result.found_valid_locally.is_none());
        assert!(result.found_valid_by_core.is_none());
        assert!(result.core_is_synced.is_none());
    }
}
