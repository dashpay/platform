use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

/// A trait for validating identity nonce rules within a state transition.
pub(crate) trait StateTransitionIdentityNonceValidationV0 {
    /// Validates the identity nonce constraints for this state transition.
    ///
    /// # Arguments
    ///
    /// * `platform` – Reference to the platform state.
    /// * `block_info` – Information about the current block.
    /// * `tx` – The raw transaction argument.
    /// * `execution_context` – Execution context for the state transition.
    /// * `platform_version` – The active platform version.
    ///
    /// # Returns
    ///
    /// Returns a [`SimpleConsensusValidationResult`] on success
    /// or an [`Error`] if validation fails.
    fn validate_identity_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

pub(crate) trait StateTransitionHasIdentityNonceValidationV0 {
    /// True if the state transition has identity nonces validation.
    fn has_identity_nonce_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;
}

impl StateTransitionIdentityNonceValidationV0 for StateTransition {
    fn validate_identity_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::Batch(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractCreate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractUpdate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityUpdate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::MasternodeVote(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_) => Ok(SimpleConsensusValidationResult::new()),
        }
    }
}

impl StateTransitionHasIdentityNonceValidationV0 for StateTransition {
    fn has_identity_nonce_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
        {
            0 => {
                let has_nonce_validation = matches!(
                    self,
                    StateTransition::Batch(_)
                        | StateTransition::DataContractCreate(_)
                        | StateTransition::DataContractUpdate(_)
                        | StateTransition::IdentityUpdate(_)
                        | StateTransition::IdentityCreditTransfer(_)
                        | StateTransition::IdentityCreditWithdrawal(_)
                );

                Ok(has_nonce_validation)
            }
            1 => {
                // Preferably to use match without wildcard arm (_) to avoid missing cases
                // in the future when new state transitions are added
                let has_nonce_validation = match self {
                    StateTransition::Batch(_)
                    | StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_) => true,
                    StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreateFromAddresses(_)
                    | StateTransition::IdentityTopUpFromAddresses(_)
                    | StateTransition::AddressFundsTransfer(_)
                    | StateTransition::AddressFundingFromAssetLock(_)
                    | StateTransition::AddressCreditWithdrawal(_)
                    | StateTransition::Shield(_)
                    | StateTransition::ShieldedTransfer(_)
                    | StateTransition::Unshield(_)
                    | StateTransition::ShieldFromAssetLock(_)
                    | StateTransition::ShieldedWithdrawal(_) => false,
                };

                Ok(has_nonce_validation)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::has_nonce_validation".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

// Version dispatch tests for has_identity_nonce_validation were intentionally removed.
// The version-specific routing (v0 vs v1) is covered by strategy tests that exercise
// the processor at the platform version used by the test harness.
