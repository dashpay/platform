use crate::consensus::basic::UnsupportedVersionError;
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;

impl PlatformVersion {
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
