use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::identity_credit_transfer_to_addresses::balance::v0::IdentityCreditTransferToAddressesTransitionBalanceValidationV0;
use crate::execution::validation::state_transition::processor::identity_balance::StateTransitionIdentityBalanceValidationV0;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(crate) mod v0;
impl StateTransitionIdentityBalanceValidationV0 for IdentityCreditTransferToAddressesTransition {
    fn validate_identity_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_transfer_to_addresses_state_transition
            .advanced_minimum_balance_pre_check
        {
            Some(0) => {
                self.validate_advanced_minimum_balance_pre_check_v0(identity, platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit transfer to addresses transition: validate_balance"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity credit transfer to addresses transition: validate_balance"
                    .to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
