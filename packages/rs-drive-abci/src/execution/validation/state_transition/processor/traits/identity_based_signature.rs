use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::ValidateStateTransitionIdentitySignature;
use crate::execution::validation::state_transition::identity_credit_withdrawal::signature_purpose_matches_requirements::IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidation;
use crate::execution::validation::state_transition::identity_top_up::identity_retrieval::v0::IdentityTopUpStateTransitionIdentityRetrievalV0;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIdentityBasedSignatureValidationV0 {
    /// Validates the identity and signatures of a transaction to ensure its authenticity.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the drive containing the transaction data.
    /// * `tx` - The transaction argument to be authenticated.
    /// * `execution_context` - A mutable reference to the StateTransitionExecutionContext that provides the context for validation.
    /// * `platform_version` - A reference to the PlatformVersion to be used for validation.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(ConsensusValidationResult<Option<PartialIdentity>>)`: Indicates that the transaction has passed authentication, and the result contains an optional `PartialIdentity`.
    /// - `Err(Error)`: Indicates that the transaction failed authentication, and the result contains an `Error` indicating the reason for failure.
    ///
    fn validate_identity_signed_state_transition(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;

    /// fetches identity info
    fn retrieve_identity_info(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;

    /// Is the state transition supposed to have an identity in the state to succeed
    fn uses_identity_in_state(&self) -> bool;

    /// Do we validate the signature based on identity info?
    fn validates_signature_based_on_identity_info(&self) -> bool;
}

impl StateTransitionIdentityBasedSignatureValidationV0 for StateTransition {
    fn validate_identity_signed_state_transition(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        match self {
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreditTransferToAddresses(_) => {
                //Basic signature verification
                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    true,
                    false,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
            StateTransition::IdentityCreditWithdrawal(credit_withdrawal) => {
                let mut consensus_validation_result = self
                    .validate_state_transition_identity_signed(
                        drive,
                        true,
                        false,
                        tx,
                        execution_context,
                        platform_version,
                    )?;

                if consensus_validation_result.is_valid_with_data() {
                    let validation_result = credit_withdrawal
                        .validate_signature_purpose_matches_requirements(
                            consensus_validation_result.data.as_ref().unwrap(),
                            platform_version,
                        )?;
                    if !validation_result.is_valid() {
                        consensus_validation_result.add_errors(validation_result.errors);
                    }
                }
                Ok(consensus_validation_result)
            }
            StateTransition::IdentityUpdate(_) => {
                //Basic signature verification
                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    true,
                    true,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
            StateTransition::MasternodeVote(_) => {
                //Basic signature verification

                // We do not request the balance because masternodes do not pay for their voting
                //  themselves

                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    false,
                    false,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
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
            | StateTransition::ShieldedWithdrawal(_) => Ok(ConsensusValidationResult::new()),
        }
    }

    fn retrieve_identity_info(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        match self {
            StateTransition::IdentityTopUp(st) => Ok(st.retrieve_topped_up_identity(
                drive,
                tx,
                execution_context,
                platform_version,
            )?),
            StateTransition::IdentityTopUpFromAddresses(st) => Ok(st.retrieve_topped_up_identity(
                drive,
                tx,
                execution_context,
                platform_version,
            )?),
            _ => Ok(ConsensusValidationResult::new()),
        }
    }

    /// Is the state transition supposed to have an identity in the state to succeed
    fn uses_identity_in_state(&self) -> bool {
        match self {
            StateTransition::IdentityCreate(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_) => false,
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_) => true,
        }
    }

    /// Do we validate the signature based on identity info?
    fn validates_signature_based_on_identity_info(&self) -> bool {
        match self {
            StateTransition::IdentityCreate(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_) => false,
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_) => true,
        }
    }
}
