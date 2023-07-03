use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
use crate::validation::SimpleConsensusValidationResult;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::DriveVersion;
use crate::version::v0::PLATFORM_V1;
use crate::ProtocolError;
use std::collections::BTreeMap;

pub type FeatureVersion = u16;
pub type OptionalFeatureVersion = Option<u16>; //This is a feature that didn't always exist

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

pub struct DocumentFeatureVersionBounds {
    pub bounds: FeatureVersionBounds,
    pub base_version_mapping: BTreeMap<FeatureVersion, FeatureVersion>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StateTransitionVersion {
    pub identity_create_state_transition: FeatureVersionBounds,
    pub identity_update_state_transition: FeatureVersionBounds,
    pub identity_top_up_state_transition: FeatureVersionBounds,
    pub identity_credit_withdrawal_state_transition: FeatureVersionBounds,
    pub identity_credit_transfer_state_transition: FeatureVersionBounds,
    pub contract_create_state_transition: FeatureVersionBounds,
    pub contract_update_state_transition: FeatureVersionBounds,
    pub documents_batch_state_transition: FeatureVersionBounds,
    pub document_base_state_transition: FeatureVersionBounds,
    pub document_create_state_transition: DocumentFeatureVersionBounds,
    pub document_replace_state_transition: DocumentFeatureVersionBounds,
    pub document_delete_state_transition: DocumentFeatureVersionBounds,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StateTransitionSigningVersion {
    pub sign_external: FeatureVersion,
    pub sign: FeatureVersion,
    pub verify_public_key_is_enabled: FeatureVersion,
    pub verify_public_key_level_and_purpose: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AbciStructureVersion {
    pub extended_block_info: FeatureVersionBounds,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DataContractFactoryVersion {
    /// The bounds that the protocol version supports
    pub bounds: FeatureVersionBounds,
    /// This is to example say that the factory version 0 supports data contracts version 0 to 3
    /// Then version 1 supports data contracts 4 to 7
    pub allowed_contract_bounds_mapping: BTreeMap<FeatureVersion, FeatureVersionBounds>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PlatformArchitectureVersion {
    pub data_contract_factory: DataContractFactoryVersion,
}

#[derive(Clone, Copy, Debug)]
pub struct PlatformVersion {
    pub protocol_version: u32,
    pub document: FeatureVersionBounds,
    pub extended_document: FeatureVersionBounds,
    pub contract: FeatureVersionBounds,
    pub identity: FeatureVersionBounds,
    pub proofs: FeatureVersionBounds,
    pub costs: FeatureVersionBounds,
    pub state_transition_signing: StateTransitionSigningVersion,
    pub state_transitions: StateTransitionVersion,
    pub drive: DriveVersion,
    pub drive_abci: DriveAbciVersion,
    pub abci_structure: AbciStructureVersion,
    pub platform_architecture: PlatformArchitectureVersion,
}

pub const PLATFORM_VERSIONS: &'static [PlatformVersion] = &[PLATFORM_V1];

pub const LATEST_PLATFORM_VERSION: &'static PlatformVersion = &PLATFORM_V1;

impl PlatformVersion {
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

    pub fn validate_contract_version(&self, version: u16) -> SimpleConsensusValidationResult {
        if self.contract.check_version(version) {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.contract.min_version,
                    self.contract.max_version,
                )
                .into(),
            )
        }
    }

    pub fn validate_identity_create_state_transition_version(
        &self,
        version: u16,
    ) -> SimpleConsensusValidationResult {
        if self
            .state_transitions
            .identity_create_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.state_transitions
                        .identity_create_state_transition
                        .min_version,
                    self.state_transitions
                        .identity_create_state_transition
                        .max_version,
                )
                .into(),
            )
        }
    }

    pub fn validate_identity_top_up_state_transition_version(
        &self,
        version: u16,
    ) -> SimpleConsensusValidationResult {
        if self
            .state_transitions
            .identity_top_up_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.state_transitions
                        .identity_top_up_state_transition
                        .min_version,
                    self.state_transitions
                        .identity_top_up_state_transition
                        .max_version,
                )
                .into(),
            )
        }
    }

    pub fn validate_identity_update_state_transition_version(
        &self,
        version: u16,
    ) -> SimpleConsensusValidationResult {
        if self
            .state_transitions
            .identity_update_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.state_transitions
                        .identity_update_state_transition
                        .min_version,
                    self.state_transitions
                        .identity_update_state_transition
                        .max_version,
                )
                .into(),
            )
        }
    }

    pub fn validate_identity_credit_withdrawal_state_transition_version(
        &self,
        version: u16,
    ) -> SimpleConsensusValidationResult {
        if self
            .state_transitions
            .identity_credit_withdrawal_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.state_transitions
                        .identity_credit_withdrawal_state_transition
                        .min_version,
                    self.state_transitions
                        .identity_credit_withdrawal_state_transition
                        .max_version,
                )
                .into(),
            )
        }
    }

    pub fn validate_contract_create_state_transition_version(
        &self,
        version: u16,
    ) -> SimpleConsensusValidationResult {
        if self
            .state_transitions
            .contract_create_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.state_transitions
                        .contract_create_state_transition
                        .min_version,
                    self.state_transitions
                        .contract_create_state_transition
                        .max_version,
                )
                .into(),
            )
        }
    }

    pub fn validate_contract_update_state_transition_version(
        &self,
        version: u16,
    ) -> SimpleConsensusValidationResult {
        if self
            .state_transitions
            .contract_update_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.state_transitions
                        .contract_update_state_transition
                        .min_version,
                    self.state_transitions
                        .contract_update_state_transition
                        .max_version,
                )
                .into(),
            )
        }
    }
}
