use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::address_credit_withdrawal::StateTransitionAddressCreditWithdrawalTransitionActionTransformer;
use crate::execution::validation::state_transition::address_funding_from_asset_lock::StateTransitionAddressFundingFromAssetLockTransitionActionTransformer;
use crate::execution::validation::state_transition::address_funds_transfer::StateTransitionAddressFundsTransferTransitionActionTransformer;
use crate::execution::validation::state_transition::identity_create::StateTransitionActionTransformerForIdentityCreateTransitionV0;
use crate::execution::validation::state_transition::identity_create_from_addresses::StateTransitionActionTransformerForIdentityCreateFromAddressesTransitionV0;
use crate::execution::validation::state_transition::identity_top_up::StateTransitionIdentityTopUpTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::serialization::Signable;
use dpp::state_transition::StateTransition;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

/// A trait for validating state transitions within a blockchain.
pub trait StateTransitionActionTransformer {
    /// A trait for transforming a raw `TransactionArg` into a typed `StateTransitionAction`.
    ///
    /// This is primarily intended for **testing**, internal tooling, and controlled validation flows.
    /// In production, the transformation is normally performed within `validate_state`, and this
    /// method should not be invoked directly.
    ///
    /// ## Versioning Note
    /// This trait is intentionally **not versioned**.
    /// If the structure of `transform_into_action` ever needs to change, a new trait should be
    /// introduced rather than modifying this one to preserve API stability.
    ///
    /// # Type Parameters
    /// * `C` – A type implementing the [`CoreRPCLike`] trait.
    ///
    /// # Arguments
    /// * `platform` – A platform reference implementing `CoreRPCLike`.
    /// * `block_info` – Information about the current block.
    /// * `remaining_address_input_balances` – A map of input addresses to their (nonce, balance)
    ///   tuples used during partial validation.
    /// * `validation_mode` – The current validation mode controlling the strictness of checks.
    /// * `execution_context` – The execution context for the state transition.
    /// * `tx` – The raw transaction argument to be transformed.
    ///
    /// # Returns
    /// A `Result` containing:
    /// * `ConsensusValidationResult<StateTransitionAction>` on success, or
    /// * `Error` if the action could not be created or validated.
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionActionTransformer for StateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::DataContractUpdate(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreate(st) => {
                let signable_bytes = self.signable_bytes()?;
                st.transform_into_action_for_identity_create_transition(
                    platform,
                    signable_bytes,
                    validation_mode,
                    execution_context,
                    tx,
                )
            }
            StateTransition::IdentityUpdate(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityTopUp(st) => {
                let signable_bytes = self.signable_bytes()?;
                st.transform_into_action_for_identity_top_up_transition(
                    platform,
                    signable_bytes,
                    validation_mode,
                    execution_context,
                    tx,
                )
            }
            StateTransition::IdentityCreditWithdrawal(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::Batch(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::MasternodeVote(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.transform_into_action(
                platform,
                block_info,
                remaining_address_input_balances,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreateFromAddresses(st) => {
                let Some(remaining_address_input_balances) = remaining_address_input_balances
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we must have remaining address input balances",
                    )));
                };
                st.transform_into_action_for_identity_create_from_addresses_transition(
                    platform,
                    remaining_address_input_balances.clone(),
                )
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                let Some(remaining_address_input_balances) = remaining_address_input_balances
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we must have remaining address input balances",
                    )));
                };
                Ok(
                    IdentityTopUpFromAddressesTransitionAction::try_from_transition(
                        st,
                        remaining_address_input_balances.clone(),
                    )
                    .map(|action| action.into()),
                )
            }
            StateTransition::AddressFundsTransfer(st) => {
                let Some(remaining_address_input_balances) = remaining_address_input_balances
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we must have remaining address input balances",
                    )));
                };
                st.transform_into_action_for_address_funds_transfer_transition(
                    platform,
                    remaining_address_input_balances.clone(),
                )
            }
            StateTransition::AddressFundingFromAssetLock(st) => {
                let Some(remaining_address_input_balances) = remaining_address_input_balances
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we must have remaining address input balances",
                    )));
                };
                let signable_bytes = self.signable_bytes()?;
                st.transform_into_action_for_address_funding_from_asset_lock_transition(
                    platform,
                    signable_bytes,
                    remaining_address_input_balances.clone(),
                    validation_mode,
                    execution_context,
                    tx,
                )
            }
            StateTransition::AddressCreditWithdrawal(st) => {
                let Some(remaining_address_input_balances) = remaining_address_input_balances
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we must have remaining address input balances",
                    )));
                };
                st.transform_into_action_for_address_credit_withdrawal_transition(
                    platform,
                    block_info,
                    remaining_address_input_balances.clone(),
                )
            }
        }
    }
}
