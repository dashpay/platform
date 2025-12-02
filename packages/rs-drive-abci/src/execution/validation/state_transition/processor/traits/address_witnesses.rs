use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::serialization::Signable;
use dpp::state_transition::{StateTransition, StateTransitionWitnessValidation};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// A trait for validating address witnesses within a state transition.
pub(crate) trait StateTransitionAddressWitnessValidationV0 {
    /// Validates the address witnesses for this state transition.
    ///
    /// # Arguments
    ///
    /// * `platform_version` – The active platform version.
    ///
    /// # Returns
    ///
    /// Returns a [`SimpleConsensusValidationResult`] on success
    /// or an [`Error`] if validation fails.
    fn validate_address_witnesses(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

pub(crate) trait StateTransitionHasAddressWitnessValidationV0 {
    /// True if the state transition has address witness validation.
    fn has_address_witness_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;
}

impl StateTransitionAddressWitnessValidationV0 for StateTransition {
    fn validate_address_witnesses(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .validate_address_witnesses
        {
            0 => {
                let signable_bytes = self
                    .signable_bytes()
                    .map_err(|e| Error::Protocol(e.into()))?;

                let result = match self {
                    StateTransition::AddressFundsTransfer(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::IdentityCreateFromAddresses(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::IdentityTopUpFromAddresses(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::AddressCreditWithdrawal(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::AddressFundingFromAssetLock(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    // These state transitions don't have address witness validation
                    StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::Batch(_)
                    | StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_)
                    | StateTransition::AddressFundingFromAssetLock(_) => {
                        SimpleConsensusValidationResult::new()
                    }
                };
                Ok(result)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_address_witnesses".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionHasAddressWitnessValidationV0 for StateTransition {
    fn has_address_witness_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .has_address_witness_validation
        {
            0 => {
                // Preferably use match without wildcard arm to avoid missing cases
                // in the future when new state transitions are added
                let has_address_witness_validation = match self {
                    StateTransition::AddressFundsTransfer(_)
                    | StateTransition::IdentityCreateFromAddresses(_)
                    | StateTransition::IdentityTopUpFromAddresses(_)
                    | StateTransition::AddressCreditWithdrawal(_) => true,
                    StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::Batch(_)
                    | StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_)
                    | StateTransition::AddressFundingFromAssetLock(_) => false,
                };

                Ok(has_address_witness_validation)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::has_address_witness_validation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
