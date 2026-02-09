use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_create::StateTransitionStateValidationForIdentityCreateTransitionV0;
use crate::execution::validation::state_transition::identity_create_from_addresses::StateTransitionStateValidationForIdentityCreateFromAddressesTransitionV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionStateValidation: StateTransitionActionTransformer {
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
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    /// Do we perform the validate state call on this transaction?
    /// Some parts of state validation (balances/nonces) happen outside this call.
    fn has_state_validation(&self) -> bool {
        true
    }

    /// Validates full state on check tx, this is only the case for masternode voting.
    /// This is because masternodes do not pay for the voting, so we need to make sure they can
    /// actually vote
    fn validates_full_state_on_check_tx(&self) -> bool {
        false
    }
}

impl StateTransitionStateValidation for StateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            // The replay attack is prevented by checking if a data contract exists with this id first
            StateTransition::DataContractCreate(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                block_info,
                execution_context,
                tx,
            ),
            // The replay attack is prevented by identity data contract nonce
            StateTransition::DataContractUpdate(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                block_info,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreate(st) => {
                let action =
                    action.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "identity create validation should always an action",
                    )))?;
                let StateTransitionAction::IdentityCreateAction(action) = action else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a identity create transition action",
                    )));
                };
                st.validate_state_for_identity_create_transition(
                    action,
                    platform,
                    execution_context,
                    tx,
                )
            }
            StateTransition::IdentityUpdate(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                block_info,
                execution_context,
                tx,
            ),
            StateTransition::IdentityTopUp(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "identity top up should not have state validation",
                )))
            }
            StateTransition::IdentityCreditWithdrawal(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "identity credit withdrawal should not have state validation",
                )))
            }
            // The replay attack is prevented by identity data contract nonce
            StateTransition::Batch(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                block_info,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                block_info,
                execution_context,
                tx,
            ),
            StateTransition::MasternodeVote(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                block_info,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreditTransferToAddresses(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "identity credit transfer to addresses should not have state validation",
                )))
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                let action =
                    action.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "identity create from addresses validation should always an action",
                    )))?;
                let StateTransitionAction::IdentityCreateFromAddressesAction(action) = action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a identity create from addresses transition action",
                    )));
                };
                st.validate_state_for_identity_create_from_addresses_transition(
                    action,
                    platform,
                    execution_context,
                    tx,
                )
            }
            StateTransition::IdentityTopUpFromAddresses(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "identity top up from addresses should not have state validation",
                )))
            }
            StateTransition::AddressFundsTransfer(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "address funds transfer should not have state validation",
                )))
            }
            StateTransition::AddressFundingFromAssetLock(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "address funding from asset lock should not have state validation",
                )))
            }
            StateTransition::AddressCreditWithdrawal(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "address credit withdrawal should not have state validation",
                )))
            }
            StateTransition::Shield(_) => Err(Error::Execution(
                ExecutionError::CorruptedCodeExecution("shield should not have state validation"),
            )),
            StateTransition::ShieldedTransfer(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "shielded transfer should not have state validation",
                )))
            }
            StateTransition::Unshield(_) => Err(Error::Execution(
                ExecutionError::CorruptedCodeExecution("unshield should not have state validation"),
            )),
        }
    }

    fn has_state_validation(&self) -> bool {
        match self {
            StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::DataContractCreate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_) => true,
            StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_) => false,
        }
    }
}
