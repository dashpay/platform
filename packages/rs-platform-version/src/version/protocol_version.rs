use crate::error::PlatformVersionError;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::DriveVersion;
use crate::version::fee::FeeVersion;
#[cfg(feature = "mock-versions")]
use crate::version::mocks::v2_test::TEST_PLATFORM_V2;
#[cfg(feature = "mock-versions")]
use crate::version::mocks::v3_test::TEST_PLATFORM_V3;
#[cfg(feature = "mock-versions")]
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
use crate::version::system_data_contract_versions::SystemDataContractVersions;
#[cfg(feature = "mock-versions")]
use std::sync::OnceLock;

use crate::version::consensus_versions::ConsensusVersions;
use crate::version::system_limits::SystemLimits;

use crate::version::v1::PLATFORM_V1;
use crate::version::v10::PLATFORM_V10;
use crate::version::v11::PLATFORM_V11;
use crate::version::v12::PLATFORM_V12;
use crate::version::v13::PLATFORM_V13;
use crate::version::v2::PLATFORM_V2;
use crate::version::v3::PLATFORM_V3;
use crate::version::v4::PLATFORM_V4;
use crate::version::v5::PLATFORM_V5;
use crate::version::v6::PLATFORM_V6;
use crate::version::v7::PLATFORM_V7;
use crate::version::v8::PLATFORM_V8;
use crate::version::v9::PLATFORM_V9;

use crate::version::ProtocolVersion;
pub use versioned_feature_core::*;

#[derive(Clone, Debug)]
pub struct PlatformVersion {
    pub protocol_version: ProtocolVersion,
    pub dpp: DPPVersion,
    pub drive: DriveVersion,
    pub drive_abci: DriveAbciVersion,
    pub consensus: ConsensusVersions,
    pub fee_version: FeeVersion,
    pub system_data_contracts: SystemDataContractVersions,
    pub system_limits: SystemLimits,
}

pub const PLATFORM_VERSIONS: &[PlatformVersion] = &[
    PLATFORM_V1,
    PLATFORM_V2,
    PLATFORM_V3,
    PLATFORM_V4,
    PLATFORM_V5,
    PLATFORM_V6,
    PLATFORM_V7,
    PLATFORM_V8,
    PLATFORM_V9,
    PLATFORM_V10,
    PLATFORM_V11,
    PLATFORM_V12,
    PLATFORM_V13,
];

#[cfg(feature = "mock-versions")]
// We use OnceLock to be able to modify the version mocks
pub static PLATFORM_TEST_VERSIONS: OnceLock<Vec<PlatformVersion>> = OnceLock::new();
#[cfg(feature = "mock-versions")]
const DEFAULT_PLATFORM_TEST_VERSIONS: &[PlatformVersion] = &[TEST_PLATFORM_V2, TEST_PLATFORM_V3];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V13;

pub const DESIRED_PLATFORM_VERSION: &PlatformVersion = LATEST_PLATFORM_VERSION;

impl PlatformVersion {
    pub fn get<'a>(version: ProtocolVersion) -> Result<&'a Self, PlatformVersionError> {
        if version > 0 {
            #[cfg(feature = "mock-versions")]
            {
                if version >> TEST_PROTOCOL_VERSION_SHIFT_BYTES > 0 {
                    let test_version = version - (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES);

                    // Init default set of test versions
                    let versions = PLATFORM_TEST_VERSIONS
                        .get_or_init(|| vec![TEST_PLATFORM_V2, TEST_PLATFORM_V3]);

                    return versions.get(test_version as usize - 2).ok_or(
                        PlatformVersionError::UnknownVersionError(format!(
                            "no test platform version {test_version}"
                        )),
                    );
                }
            }
            PLATFORM_VERSIONS.get(version as usize - 1).ok_or_else(|| {
                PlatformVersionError::UnknownVersionError(format!("no platform version {version}"))
            })
        } else {
            Err(PlatformVersionError::UnknownVersionError(format!(
                "no platform version {version}"
            )))
        }
    }

    pub fn get_optional<'a>(version: ProtocolVersion) -> Option<&'a Self> {
        if version > 0 {
            #[cfg(feature = "mock-versions")]
            {
                if version >> TEST_PROTOCOL_VERSION_SHIFT_BYTES > 0 {
                    let test_version = version - (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES);

                    // Init default set of test versions
                    let versions = PLATFORM_TEST_VERSIONS
                        .get_or_init(|| vec![TEST_PLATFORM_V2, TEST_PLATFORM_V3]);

                    return versions.get(test_version as usize - 2);
                }
            }
            PLATFORM_VERSIONS.get(version as usize - 1)
        } else {
            None
        }
    }

    pub fn get_version_or_latest<'a>(
        version: Option<ProtocolVersion>,
    ) -> Result<&'a Self, PlatformVersionError> {
        if let Some(version) = version {
            if version > 0 {
                #[cfg(feature = "mock-versions")]
                {
                    if version >> TEST_PROTOCOL_VERSION_SHIFT_BYTES > 0 {
                        let test_version = version - (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES);

                        // Init default set of test versions
                        let versions = PLATFORM_TEST_VERSIONS
                            .get_or_init(|| Vec::from(DEFAULT_PLATFORM_TEST_VERSIONS));

                        return versions.get(test_version as usize - 2).ok_or(
                            PlatformVersionError::UnknownVersionError(format!(
                                "no test platform version {test_version}"
                            )),
                        );
                    }
                }
                PLATFORM_VERSIONS.get(version as usize - 1).ok_or(
                    PlatformVersionError::UnknownVersionError(format!(
                        "no platform version {version}"
                    )),
                )
            } else {
                Err(PlatformVersionError::UnknownVersionError(format!(
                    "no platform version {version}"
                )))
            }
        } else {
            Ok(Self::latest())
        }
    }

    pub fn first<'a>() -> &'a Self {
        PLATFORM_VERSIONS
            .first()
            .expect("expected to have a platform version")
    }

    pub fn latest<'a>() -> &'a Self {
        PLATFORM_VERSIONS
            .last()
            .expect("expected to have a platform version")
    }

    pub fn desired<'a>() -> &'a Self {
        DESIRED_PLATFORM_VERSION
    }

    #[cfg(feature = "mock-versions")]
    /// Set mock versions for testing
    pub fn replace_test_versions(versions: Vec<PlatformVersion>) {
        PLATFORM_TEST_VERSIONS
            .set(versions)
            .expect("failed to set test versions")
    }
}

#[cfg(test)]
mod shielded_pool_gating_tests {
    use super::PlatformVersion;

    // The shielded credit pool lives in GroveDB at
    // `[RootTree::ShieldedBalances (52), MAIN_SHIELDED_CREDIT_POOL_KEY ("M" = 0x4d)]`.
    // That subtree is created ONLY at the protocol v12 upgrade migration
    // (`transition_to_version_12`) or on a v12 genesis (`create_initial_state_structure_v3`).
    // It does not exist on a protocol-v11 state (v11 uses init structure v2).
    //
    // The block-end / recent-block-storage shielded methods below read that
    // subtree unconditionally every block. If they are active before v12 they
    // open `[52, "M"]` on a state where it was never created, producing the
    // consensus-breaking GroveDB error
    //   "path parent layer not found: could not get key 4d for parent [52]".
    //
    // These methods were inactive (`None`) through protocol v11 in the released
    // 3.0.1 line, then accidentally turned on for v11 when the four fields were
    // added as `Some(0)` to the SHARED `DRIVE_ABCI_METHOD_VERSIONS_V7` struct in
    // the Medusa shielded-pool PR (#3177) — V7 is referenced by both v11.rs and
    // v12.rs. These tests pin the invariant: shielded reads are gated to v12+.

    #[test]
    fn shielded_block_processing_methods_inactive_before_v12() {
        let v11 = PlatformVersion::get(11).expect("protocol version 11 must exist");
        let block_end = &v11.drive_abci.methods.block_end;

        assert_eq!(
            block_end.record_shielded_pool_anchor, None,
            "v11 must NOT record shielded pool anchors: the [52, \"M\"] subtree does not exist before v12"
        );
        assert_eq!(
            block_end.prune_shielded_pool_anchors, None,
            "v11 must NOT prune shielded pool anchors: the [52, \"M\"] subtree does not exist before v12"
        );
    }

    #[test]
    fn shielded_block_processing_methods_active_at_v12() {
        let v12 = PlatformVersion::get(12).expect("protocol version 12 must exist");
        let block_end = &v12.drive_abci.methods.block_end;

        // At v12 the shielded pool subtree is created by the upgrade migration,
        // so the same methods must be active.
        assert_eq!(block_end.record_shielded_pool_anchor, Some(0));
        assert_eq!(block_end.prune_shielded_pool_anchors, Some(0));
    }
}
