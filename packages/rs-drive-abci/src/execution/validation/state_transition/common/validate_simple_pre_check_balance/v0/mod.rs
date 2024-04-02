use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::identity::PartialIdentity;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub trait ValidateSimplePreCheckBalanceV0 {
    fn validate_simple_pre_check_minimum_balance_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl ValidateSimplePreCheckBalanceV0 for StateTransition {
    fn validate_simple_pre_check_minimum_balance_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let amount = match self {
            StateTransition::DataContractCreate(_) => {
                platform_version
                    .fee_version
                    .state_transition_min_fees
                    .contract_create
            }
            StateTransition::DataContractUpdate(_) => {
                platform_version
                    .fee_version
                    .state_transition_min_fees
                    .contract_update
            }
            StateTransition::DocumentsBatch(_) => {
                platform_version
                    .fee_version
                    .state_transition_min_fees
                    .document_batch_transition
            }
            StateTransition::IdentityCreate(_) | StateTransition::IdentityTopUp(_) => 0,
            StateTransition::IdentityCreditWithdrawal(_) => {
                platform_version
                    .fee_version
                    .state_transition_min_fees
                    .credit_withdrawal
            }
            StateTransition::IdentityUpdate(_) => {
                platform_version
                    .fee_version
                    .state_transition_min_fees
                    .identity_update
            }
            StateTransition::IdentityCreditTransfer(_) => {
                platform_version
                    .fee_version
                    .state_transition_min_fees
                    .credit_transfer
            }
        };

        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for identity based operations",
                )))?;

        if balance < amount {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(identity.id, balance, amount).into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
