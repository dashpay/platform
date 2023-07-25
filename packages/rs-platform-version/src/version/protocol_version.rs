use crate::error::PlatformVersionError;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::DriveVersion;
use crate::version::v0::PLATFORM_V1;
use std::collections::BTreeMap;

pub type FeatureVersion = u16;
pub type OptionalFeatureVersion = Option<u16>; //This is a feature that didn't always exist

#[derive(Clone, Debug, Default)]
pub struct FeatureVersionBounds {
    pub min_version: FeatureVersion,
    pub max_version: FeatureVersion,
    pub default_current_version: FeatureVersion,
}

impl FeatureVersionBounds {
    /// Will get a protocol error if the version is unknown
    pub fn check_version(&self, version: FeatureVersion) -> bool {
        version >= self.min_version && version <= self.max_version
    }
}

#[derive(Clone, Debug, Default)]
pub struct AbciStructureVersion {
    pub extended_block_info: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct PlatformArchitectureVersion {
    pub data_contract_factory_structure_version: FeatureVersion,
    pub document_factory_structure_version: FeatureVersion,
}

#[derive(Clone, Debug)]
pub struct PlatformVersion {
    pub protocol_version: u32,
    pub identity: FeatureVersionBounds,
    pub proofs: FeatureVersionBounds,
    pub costs: FeatureVersionBounds,
    pub dpp: DPPVersion,
    pub drive: DriveVersion,
    pub drive_abci: DriveAbciVersion,
    pub abci_structure: AbciStructureVersion,
    pub platform_architecture: PlatformArchitectureVersion,
}

pub const PLATFORM_VERSIONS: &[PlatformVersion] = &[PLATFORM_V1];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V1;

impl PlatformVersion {
    pub fn get<'a>(version: u32) -> Result<&'a Self, PlatformVersionError> {
        if version > 0 {
            PLATFORM_VERSIONS.get(version as usize - 1).ok_or(
                PlatformVersionError::UnknownVersionError(format!("no platform version {version}")),
            )
        } else {
            Err(PlatformVersionError::UnknownVersionError(format!(
                "no platform version {version}"
            )))
        }
    }

    pub fn get_version_or_latest<'a>(
        version: Option<u32>,
    ) -> Result<&'a Self, PlatformVersionError> {
        if let Some(version) = version {
            if version > 0 {
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
}
