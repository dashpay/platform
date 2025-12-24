use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionIdentityEstimatedFeeValidation};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIdentityBalanceValidationV0 {
    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<StateTransitionAction>, Error>` - A result with either a ConsensusValidationResult containing a StateTransitionAction or an Error.
    fn validate_identity_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has a balance validation.
    /// This balance validation is not for the operations of the state transition, but more as a
    /// quick early verification that the user has the balance they want to transfer or withdraw.
    fn has_identity_minimum_balance_pre_check_validation(&self) -> bool {
        true
    }
}

impl StateTransitionIdentityBalanceValidationV0 for StateTransition {
    fn validate_identity_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for credit transfer transition",
                )))?;
        match self {
            StateTransition::IdentityCreditTransfer(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityCreditWithdrawal(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::Batch(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityCreditTransferToAddresses(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::DataContractCreate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::DataContractUpdate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityUpdate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_) => {
                Ok(SimpleConsensusValidationResult::new())
            }
        }
    }

    fn has_identity_minimum_balance_pre_check_validation(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityCreditTransfer(_)
                | StateTransition::IdentityCreditWithdrawal(_)
                | StateTransition::DataContractCreate(_)
                | StateTransition::DataContractUpdate(_)
                | StateTransition::Batch(_)
                | StateTransition::IdentityUpdate(_)
                | StateTransition::IdentityCreditTransferToAddresses(_)
        )
    }
}
