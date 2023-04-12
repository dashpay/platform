use crate::version::v0::PLATFORM_V0;
use crate::ProtocolError;

pub type FeatureVersion = u16;

#[derive(Clone, Copy, Debug, Default)]
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

#[derive(Clone, Copy, Debug, Default)]
pub struct StateTransitionVersion {
    pub identity_create_state_transition: FeatureVersionBounds,
    pub identity_update_state_transition: FeatureVersionBounds,
    pub identity_top_up_state_transition: FeatureVersionBounds,
    pub identity_credit_withdrawal_state_transition: FeatureVersionBounds,
    pub contract_create_state_transition: FeatureVersionBounds,
    pub contract_update_state_transition: FeatureVersionBounds,
    pub documents_batch_state_transition: FeatureVersionBounds,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveStructureVersion {
    pub document_indexes: FeatureVersionBounds,
    pub identity_indexes: FeatureVersionBounds,
    pub pools: FeatureVersionBounds,
}

#[derive(Clone, Copy, Debug)]
pub struct PlatformVersion {
    pub protocol_version: u32,
    pub contract: FeatureVersionBounds,
    pub proofs: FeatureVersionBounds,
    pub costs: FeatureVersionBounds,
    pub state_transitions: StateTransitionVersion,
    pub drive_structure: DriveStructureVersion,
}

const PLATFORM_VERSIONS: &'static [PlatformVersion] = &[PLATFORM_V0];

const LATEST_VERSION: PlatformVersion = PLATFORM_V0;

impl PlatformVersion {
    pub fn get(version: u32) -> Result<Self, ProtocolError> {
        PLATFORM_VERSIONS.get(version as usize).copied().ok_or(
            ProtocolError::UnknownProtocolVersionError(format!("no platform version {version}")),
        )
    }

    pub fn validate_contract_version(&self, version: u16) -> bool {
        self.contract.check_version(version)
    }

    pub fn validate_identity_create_state_transition_version(&self, version: u16) -> bool {
        self.state_transitions
            .identity_create_state_transition
            .check_version(version)
    }

    pub fn validate_identity_top_up_state_transition_version(&self, version: u16) -> bool {
        self.state_transitions
            .identity_top_up_state_transition
            .check_version(version)
    }

    pub fn validate_identity_update_state_transition_version(&self, version: u16) -> bool {
        self.state_transitions
            .identity_update_state_transition
            .check_version(version)
    }

    pub fn validate_identity_credit_withdrawal_state_transition_version(
        &self,
        version: u16,
    ) -> bool {
        self.state_transitions
            .identity_credit_withdrawal_state_transition
            .check_version(version)
    }

    pub fn validate_contract_create_state_transition_version(&self, version: u16) -> bool {
        self.state_transitions
            .contract_create_state_transition
            .check_version(version)
    }

    pub fn validate_contract_update_state_transition_version(&self, version: u16) -> bool {
        self.state_transitions
            .contract_update_state_transition
            .check_version(version)
    }
}
