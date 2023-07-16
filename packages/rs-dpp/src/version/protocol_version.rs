use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::DriveVersion;
use crate::version::v0::PLATFORM_V1;
use crate::ProtocolError;
use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::sync::RwLock;

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
pub struct StateTransitionSigningVersion {
    pub sign_external: FeatureVersion,
    pub sign: FeatureVersion,
    pub verify_public_key_is_enabled: FeatureVersion,
    pub verify_public_key_level_and_purpose: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct AbciStructureVersion {
    pub extended_block_info: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct PlatformArchitectureVersion {
    pub data_contract_factory_structure_version: FeatureVersion,
}

#[derive(Clone, Debug)]
pub struct PlatformVersion {
    pub protocol_version: u32,
    pub identity: FeatureVersionBounds,
    pub proofs: FeatureVersionBounds,
    pub costs: FeatureVersionBounds,
    pub state_transition_signing: StateTransitionSigningVersion,
    pub dpp: DPPVersion,
    pub drive: DriveVersion,
    pub drive_abci: DriveAbciVersion,
    pub abci_structure: AbciStructureVersion,
    pub platform_architecture: PlatformArchitectureVersion,
}

pub const PLATFORM_VERSIONS: &[PlatformVersion] = &[PLATFORM_V1];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V1;

lazy_static! {
    static ref CURRENT_PLATFORM_VERSION: RwLock<Option<&'static PlatformVersion>> =
        RwLock::new(None);
}

impl PlatformVersion {
    pub fn set_current(platform_version: &'static PlatformVersion) {
        let mut context = CURRENT_PLATFORM_VERSION.write().unwrap();
        *context = Some(platform_version);
    }

    pub fn get_current<'a>() -> Result<&'a Self, ProtocolError> {
        CURRENT_PLATFORM_VERSION
            .read()
            .unwrap()
            .ok_or(ProtocolError::CurrentProtocolVersionNotInitialized)
    }

    pub fn get<'a>(version: u32) -> Result<&'a Self, ProtocolError> {
        if version > 0 {
            PLATFORM_VERSIONS.get(version as usize - 1).ok_or(
                ProtocolError::UnknownProtocolVersionError(format!(
                    "no platform version {version}"
                )),
            )
        } else {
            Err(ProtocolError::UnknownProtocolVersionError(format!(
                "no platform version {version}"
            )))
        }
    }

    pub fn latest<'a>() -> &'a Self {
        PLATFORM_VERSIONS
            .last()
            .expect("expected to have a platform version")
    }
}
