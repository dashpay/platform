use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::engine::consensus_params_update::v0::consensus_params_update_v0;
use crate::platform_types::epoch_info::EpochInfo;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::types::ConsensusParams;

mod v0;
mod v1;

pub(crate) fn consensus_params_update(
    network: Network,
    original_platform_version: &PlatformVersion,
    new_platform_version: &PlatformVersion,
    epoch_info: &EpochInfo,
) -> Result<Option<ConsensusParams>, Error> {
    match new_platform_version
        .drive_abci
        .methods
        .engine
        .consensus_params_update
    {
        0 => Ok(consensus_params_update_v0(
            network,
            original_platform_version,
            new_platform_version,
            epoch_info,
        )),
        1 => Ok(v1::consensus_params_update_v1(
            network,
            original_platform_version,
            new_platform_version,
            epoch_info,
        )),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "consensus_params_update".to_string(),
            known_versions: vec![0, 1],
            received: version,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use dpp::dashcore::Network;
    use dpp::version::PlatformVersion;

    /// Helper: build an EpochInfo that represents the first block of a given epoch.
    fn epoch_change_to(epoch_index: u16) -> EpochInfo {
        EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: epoch_index,
            previous_epoch_index: if epoch_index > 0 {
                Some(epoch_index - 1)
            } else {
                None
            },
            is_epoch_change: true,
        })
    }

    /// Helper: build an EpochInfo for a non-epoch-change block in a given epoch.
    fn mid_epoch(epoch_index: u16) -> EpochInfo {
        EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: epoch_index,
            previous_epoch_index: None,
            is_epoch_change: false,
        })
    }

    // -----------------------------------------------------------------------
    // Tests for the top-level version dispatch
    // -----------------------------------------------------------------------

    mod version_dispatch {
        use super::*;

        #[test]
        fn dispatches_to_v0_when_version_is_0() {
            // PlatformVersion v1 has consensus_params_update = 0
            let platform_v1 = PlatformVersion::first();
            assert_eq!(
                platform_v1
                    .drive_abci
                    .methods
                    .engine
                    .consensus_params_update,
                0
            );
            let epoch_info = mid_epoch(5);
            // Same version for original and new => None
            let result =
                consensus_params_update(Network::Mainnet, platform_v1, platform_v1, &epoch_info);
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn dispatches_to_v1_when_version_is_1() {
            // PlatformVersion v3+ has consensus_params_update = 1
            let platform_v3 = PlatformVersion::get(3).unwrap();
            assert_eq!(
                platform_v3
                    .drive_abci
                    .methods
                    .engine
                    .consensus_params_update,
                1
            );
            let epoch_info = mid_epoch(5);
            // Same version for original and new => None
            let result =
                consensus_params_update(Network::Mainnet, platform_v3, platform_v3, &epoch_info);
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn returns_error_for_unknown_version() {
            // Clone a real PlatformVersion and set consensus_params_update to an
            // invalid value so we exercise the actual dispatch error path.
            let mut bad_version = PlatformVersion::first().clone();
            bad_version
                .drive_abci
                .methods
                .engine
                .consensus_params_update = 99;

            let epoch_info = mid_epoch(5);
            let result = consensus_params_update(
                Network::Mainnet,
                PlatformVersion::first(),
                &bad_version,
                &epoch_info,
            );

            match result {
                Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                    method,
                    known_versions,
                    received,
                })) => {
                    assert_eq!(method, "consensus_params_update");
                    assert_eq!(known_versions, vec![0, 1]);
                    assert_eq!(received, 99);
                }
                other => panic!("expected UnknownVersionMismatch error, got: {:?}", other),
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tests for consensus_params_update_v0
    // -----------------------------------------------------------------------

    mod v0_tests {
        use super::*;
        use crate::execution::engine::consensus_params_update::v0::consensus_params_update_v0;

        #[test]
        fn dash_mainnet_epoch_3_returns_emergency_update() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v0(Network::Mainnet, platform_v1, platform_v1, &epoch_info);

            let params = result.expect("should return Some for Dash epoch 3");
            let version = params.version.expect("should have version params");
            assert_eq!(version.app_version, platform_v1.protocol_version as u64);
            assert_eq!(version.consensus_version, 1);
            // All other fields should be None
            assert!(params.block.is_none());
            assert!(params.evidence.is_none());
            assert!(params.validator.is_none());
            assert!(params.synchrony.is_none());
            assert!(params.timeout.is_none());
            assert!(params.abci.is_none());
        }

        #[test]
        fn dash_mainnet_epoch_3_uses_new_version_app_version() {
            let platform_v1 = PlatformVersion::first();
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v0(Network::Mainnet, platform_v1, platform_v3, &epoch_info);

            let params = result.expect("should return Some for Dash epoch 3");
            let version = params.version.unwrap();
            // The app_version should come from new_platform_version, not original
            assert_eq!(version.app_version, platform_v3.protocol_version as u64);
        }

        #[test]
        fn dash_mainnet_not_epoch_3_falls_through_to_normal_logic() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = epoch_change_to(4); // Not epoch 3

            // Same versions => None
            let result =
                consensus_params_update_v0(Network::Mainnet, platform_v1, platform_v1, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn dash_mainnet_mid_epoch_3_does_not_trigger_emergency() {
            let platform_v1 = PlatformVersion::first();
            // Not the first block of epoch 3 (is_epoch_change = false)
            let epoch_info = mid_epoch(3);

            let result =
                consensus_params_update_v0(Network::Mainnet, platform_v1, platform_v1, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn testnet_epoch_1480_returns_emergency_update() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = epoch_change_to(1480);

            let result =
                consensus_params_update_v0(Network::Testnet, platform_v1, platform_v1, &epoch_info);

            let params = result.expect("should return Some for Testnet epoch 1480");
            let version = params.version.expect("should have version params");
            assert_eq!(version.app_version, platform_v1.protocol_version as u64);
            assert_eq!(version.consensus_version, 1);
        }

        #[test]
        fn testnet_not_epoch_1480_falls_through_to_normal_logic() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = epoch_change_to(1479);

            let result =
                consensus_params_update_v0(Network::Testnet, platform_v1, platform_v1, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn testnet_mid_epoch_1480_does_not_trigger_emergency() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = mid_epoch(1480);

            let result =
                consensus_params_update_v0(Network::Testnet, platform_v1, platform_v1, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn regtest_never_triggers_emergency_update() {
            let platform_v1 = PlatformVersion::first();
            // Even on epoch 3, Regtest should not trigger emergency
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v0(Network::Regtest, platform_v1, platform_v1, &epoch_info);
            // Same tenderdash_consensus_version => None
            assert!(result.is_none());
        }

        #[test]
        fn devnet_never_triggers_emergency_update() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v0(Network::Devnet, platform_v1, platform_v1, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn same_consensus_version_returns_none() {
            // v1 and v2 both have tenderdash_consensus_version = 0
            let platform_v1 = PlatformVersion::first();
            let platform_v2 = PlatformVersion::get(2).unwrap();
            let epoch_info = mid_epoch(10);

            let result =
                consensus_params_update_v0(Network::Regtest, platform_v1, platform_v2, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn different_consensus_version_returns_update() {
            // v2 has tenderdash_consensus_version = 0, v3 has tenderdash_consensus_version = 1
            let platform_v2 = PlatformVersion::get(2).unwrap();
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = mid_epoch(10);

            let result =
                consensus_params_update_v0(Network::Regtest, platform_v2, platform_v3, &epoch_info);

            let params = result.expect("should return Some when consensus versions differ");
            let version = params.version.expect("should have version params");
            assert_eq!(version.app_version, platform_v3.protocol_version as u64);
            assert_eq!(
                version.consensus_version,
                platform_v3.consensus.tenderdash_consensus_version as i32
            );
        }

        #[test]
        fn different_consensus_version_on_devnet_returns_update() {
            let platform_v2 = PlatformVersion::get(2).unwrap();
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(5);

            let result =
                consensus_params_update_v0(Network::Devnet, platform_v2, platform_v3, &epoch_info);

            let params = result.expect("should return Some when consensus versions differ");
            let version = params.version.unwrap();
            assert_eq!(version.app_version, platform_v3.protocol_version as u64);
            assert_eq!(
                version.consensus_version,
                platform_v3.consensus.tenderdash_consensus_version as i32
            );
        }

        #[test]
        fn emergency_update_takes_priority_over_normal_version_change() {
            // When on Dash mainnet epoch 3, emergency update fires even if
            // consensus versions also differ
            let platform_v2 = PlatformVersion::get(2).unwrap();
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v0(Network::Mainnet, platform_v2, platform_v3, &epoch_info);

            let params = result.expect("should return Some for emergency update");
            let version = params.version.unwrap();
            // Emergency update always sets consensus_version to 1
            assert_eq!(version.consensus_version, 1);
            assert_eq!(version.app_version, platform_v3.protocol_version as u64);
        }
    }

    // -----------------------------------------------------------------------
    // Tests for consensus_params_update_v1
    // -----------------------------------------------------------------------

    mod v1_tests {
        use super::*;
        use crate::execution::engine::consensus_params_update::v1::consensus_params_update_v1;

        #[test]
        fn dash_mainnet_epoch_3_returns_emergency_update() {
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v1(Network::Mainnet, platform_v3, platform_v3, &epoch_info);

            let params = result.expect("should return Some for Dash epoch 3");
            let version = params.version.expect("should have version params");
            assert_eq!(version.app_version, platform_v3.protocol_version as u64);
            assert_eq!(version.consensus_version, 1);
        }

        #[test]
        fn testnet_epoch_1480_returns_emergency_update() {
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(1480);

            let result =
                consensus_params_update_v1(Network::Testnet, platform_v3, platform_v3, &epoch_info);

            let params = result.expect("should return Some for Testnet epoch 1480");
            let version = params.version.unwrap();
            assert_eq!(version.consensus_version, 1);
        }

        #[test]
        fn testnet_not_epoch_1480_falls_through() {
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(100);

            let result =
                consensus_params_update_v1(Network::Testnet, platform_v3, platform_v3, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn regtest_skips_emergency_update() {
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v1(Network::Regtest, platform_v3, platform_v3, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn same_consensus_and_protocol_version_returns_none() {
            // v1 differs from v0 here: both consensus version AND protocol version
            // must be the same to return None
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = mid_epoch(10);

            let result =
                consensus_params_update_v1(Network::Regtest, platform_v3, platform_v3, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn different_consensus_version_returns_update() {
            // v2 has tenderdash_consensus_version=0, v3 has tenderdash_consensus_version=1
            let platform_v2 = PlatformVersion::get(2).unwrap();
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = mid_epoch(10);

            let result =
                consensus_params_update_v1(Network::Regtest, platform_v2, platform_v3, &epoch_info);

            let params = result.expect("should return Some when consensus versions differ");
            let version = params.version.unwrap();
            assert_eq!(version.app_version, platform_v3.protocol_version as u64);
            assert_eq!(
                version.consensus_version,
                platform_v3.consensus.tenderdash_consensus_version as i32
            );
        }

        #[test]
        fn different_protocol_version_same_consensus_version_returns_update() {
            // This is the KEY difference between v0 and v1:
            // v1 also returns an update when protocol_version changes even if
            // tenderdash_consensus_version stays the same.
            // v3 and v4 both have tenderdash_consensus_version=1 but protocol_versions 3 and 4.
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let platform_v4 = PlatformVersion::get(4).unwrap();
            let epoch_info = mid_epoch(10);

            // Confirm they have the same tenderdash_consensus_version
            assert_eq!(
                platform_v3.consensus.tenderdash_consensus_version,
                platform_v4.consensus.tenderdash_consensus_version
            );
            // But different protocol versions
            assert_ne!(platform_v3.protocol_version, platform_v4.protocol_version);

            let result =
                consensus_params_update_v1(Network::Regtest, platform_v3, platform_v4, &epoch_info);

            let params = result
                .expect("v1 should return Some when protocol versions differ even if consensus version is same");
            let version = params.version.unwrap();
            assert_eq!(version.app_version, platform_v4.protocol_version as u64);
        }

        #[test]
        fn v0_would_return_none_where_v1_returns_some_for_protocol_version_change() {
            // This test demonstrates the behavioral difference between v0 and v1.
            // When only protocol_version changes (not tenderdash_consensus_version),
            // v0 returns None but v1 returns Some.
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let platform_v4 = PlatformVersion::get(4).unwrap();
            let epoch_info = mid_epoch(10);

            let v0_result =
                consensus_params_update_v0(Network::Regtest, platform_v3, platform_v4, &epoch_info);
            let v1_result =
                consensus_params_update_v1(Network::Regtest, platform_v3, platform_v4, &epoch_info);

            assert!(
                v0_result.is_none(),
                "v0 should return None when only protocol version changes"
            );
            assert!(
                v1_result.is_some(),
                "v1 should return Some when protocol version changes"
            );
        }

        #[test]
        fn emergency_update_takes_priority_over_normal_check() {
            let platform_v2 = PlatformVersion::get(2).unwrap();
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update_v1(Network::Mainnet, platform_v2, platform_v3, &epoch_info);

            let params = result.expect("emergency update should fire");
            let version = params.version.unwrap();
            assert_eq!(version.consensus_version, 1);
        }

        #[test]
        fn dash_mid_epoch_3_does_not_trigger_emergency() {
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = mid_epoch(3);

            let result =
                consensus_params_update_v1(Network::Mainnet, platform_v3, platform_v3, &epoch_info);
            assert!(result.is_none());
        }

        #[test]
        fn testnet_mid_epoch_1480_does_not_trigger_emergency() {
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = mid_epoch(1480);

            let result =
                consensus_params_update_v1(Network::Testnet, platform_v3, platform_v3, &epoch_info);
            assert!(result.is_none());
        }
    }

    // -----------------------------------------------------------------------
    // Integration-style tests through the dispatch function
    // -----------------------------------------------------------------------

    mod integration {
        use super::*;

        #[test]
        fn v0_dispatch_dash_emergency_epoch_3() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = epoch_change_to(3);

            let result =
                consensus_params_update(Network::Mainnet, platform_v1, platform_v1, &epoch_info)
                    .expect("should not error");

            assert!(result.is_some());
            let version = result.unwrap().version.unwrap();
            assert_eq!(version.consensus_version, 1);
        }

        #[test]
        fn v1_dispatch_protocol_version_change_triggers_update() {
            // Use v3 as original and v4 as new (both have consensus_params_update=1)
            // They have same tenderdash_consensus_version but different protocol_version
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let platform_v4 = PlatformVersion::get(4).unwrap();
            let epoch_info = mid_epoch(10);

            let result =
                consensus_params_update(Network::Regtest, platform_v3, platform_v4, &epoch_info)
                    .expect("should not error");

            assert!(
                result.is_some(),
                "v1 dispatch should detect protocol version change"
            );
        }

        #[test]
        fn v0_dispatch_no_change_returns_none() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = mid_epoch(50);

            let result =
                consensus_params_update(Network::Regtest, platform_v1, platform_v1, &epoch_info)
                    .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn v1_dispatch_no_change_returns_none() {
            let platform_v3 = PlatformVersion::get(3).unwrap();
            let epoch_info = mid_epoch(50);

            let result =
                consensus_params_update(Network::Regtest, platform_v3, platform_v3, &epoch_info)
                    .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn v0_dispatch_same_version_returns_none() {
            let platform_v1 = PlatformVersion::first();
            let epoch_info = mid_epoch(10);

            let result =
                consensus_params_update(Network::Devnet, platform_v1, platform_v1, &epoch_info)
                    .expect("should not error");
            assert!(result.is_none());
        }
    }
}
