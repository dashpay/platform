use crate::consensus::basic::UnsupportedVersionError;
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;

impl PlatformVersion {
    pub fn validate_contract_version(&self, version: u16) -> SimpleConsensusValidationResult {
        if self
            .dpp
            .contract_versions
            .contract_serialization_version
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.dpp
                        .contract_versions
                        .contract_serialization_version
                        .min_version,
                    self.dpp
                        .contract_versions
                        .contract_serialization_version
                        .max_version,
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
            .dpp
            .state_transition_serialization_versions
            .identity_create_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.dpp
                        .state_transition_serialization_versions
                        .identity_create_state_transition
                        .min_version,
                    self.dpp
                        .state_transition_serialization_versions
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
            .dpp
            .state_transition_serialization_versions
            .identity_top_up_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.dpp
                        .state_transition_serialization_versions
                        .identity_top_up_state_transition
                        .min_version,
                    self.dpp
                        .state_transition_serialization_versions
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
            .dpp
            .state_transition_serialization_versions
            .identity_update_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.dpp
                        .state_transition_serialization_versions
                        .identity_update_state_transition
                        .min_version,
                    self.dpp
                        .state_transition_serialization_versions
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
            .dpp
            .state_transition_serialization_versions
            .identity_credit_withdrawal_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.dpp
                        .state_transition_serialization_versions
                        .identity_credit_withdrawal_state_transition
                        .min_version,
                    self.dpp
                        .state_transition_serialization_versions
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
            .dpp
            .state_transition_serialization_versions
            .contract_create_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.dpp
                        .state_transition_serialization_versions
                        .contract_create_state_transition
                        .min_version,
                    self.dpp
                        .state_transition_serialization_versions
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
            .dpp
            .state_transition_serialization_versions
            .contract_update_state_transition
            .check_version(version)
        {
            SimpleConsensusValidationResult::default()
        } else {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    version,
                    self.dpp
                        .state_transition_serialization_versions
                        .contract_update_state_transition
                        .min_version,
                    self.dpp
                        .state_transition_serialization_versions
                        .contract_update_state_transition
                        .max_version,
                )
                .into(),
            )
        }
    }
}
