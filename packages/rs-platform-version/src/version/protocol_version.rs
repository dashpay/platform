use crate::error::PlatformVersionError;
use crate::version::contracts::SystemDataContractVersions;
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
use crate::version::v1::PLATFORM_V1;
#[cfg(feature = "mock-versions")]
use std::sync::OnceLock;

use crate::version::limits::SystemLimits;
use crate::version::v2::PLATFORM_V2;
use crate::version::ProtocolVersion;
pub use versioned_feature_core::*;

#[derive(Clone, Debug, Default)]
pub struct PlatformArchitectureVersion {
    pub data_contract_factory_structure_version: FeatureVersion,
    pub document_factory_structure_version: FeatureVersion,
}

#[derive(Clone, Debug)]
pub struct PlatformVersion {
    pub protocol_version: ProtocolVersion,
    pub proofs: FeatureVersionBounds,
    pub dpp: DPPVersion,
    pub drive: DriveVersion,
    pub drive_abci: DriveAbciVersion,
    pub fee_version: FeeVersion,
    pub platform_architecture: PlatformArchitectureVersion,
    pub system_data_contracts: SystemDataContractVersions,
    pub system_limits: SystemLimits,
}

pub const PLATFORM_VERSIONS: &[PlatformVersion] = &[PLATFORM_V1, PLATFORM_V2];

#[cfg(feature = "mock-versions")]
// We use OnceLock to be able to modify the version mocks
pub static PLATFORM_TEST_VERSIONS: OnceLock<Vec<PlatformVersion>> = OnceLock::new();
#[cfg(feature = "mock-versions")]
const DEFAULT_PLATFORM_TEST_VERSIONS: &[PlatformVersion] = &[TEST_PLATFORM_V2, TEST_PLATFORM_V3];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V2;

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

    #[cfg(feature = "mock-versions")]
    /// Set mock versions for testing
    pub fn replace_test_versions(versions: Vec<PlatformVersion>) {
        PLATFORM_TEST_VERSIONS
            .set(versions)
            .expect("failed to set test versions")
    }
}
