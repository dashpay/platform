use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::ProtocolError;

use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::StateTransitionIdentityIdFromInputs;
use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNoncesAction;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_not_in_state;
use crate::execution::validation::state_transition::ValidationMode;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create_from_addresses) trait IdentityCreateFromAddressesStateTransitionStateValidationV0
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        action: IdentityCreateFromAddressesTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateFromAddressesStateTransitionStateValidationV0
    for IdentityCreateFromAddressesTransition
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        action: IdentityCreateFromAddressesTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;

        let identity_id = self.identity_id_from_inputs()?;
        let balance =
            drive.fetch_identity_balance(identity_id.to_buffer(), transaction, platform_version)?;

        // Balance is here to check if the identity does already exist
        if balance.is_some() {
            // Since the id comes from the state transition this should never be reachable
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityAlreadyExistsError::new(identity_id.to_owned()).into(),
            ));
        }

        // Now we should check the state of added keys to make sure there aren't any that already exist
        let unique_public_key_validation_result =
            validate_unique_identity_public_key_hashes_not_in_state(
                self.public_keys(),
                drive,
                execution_context,
                transaction,
                platform_version,
            )?;

        if unique_public_key_validation_result.is_valid() {
            // We just pass the action that was given to us
            Ok(ConsensusValidationResult::new_with_data(
                StateTransitionAction::IdentityCreateFromAddressesAction(action),
            ))
        } else {
            // It's not valid, we need to give back the action that partially uses the asset lock

            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .unique_key_already_present;

            let used_credits = penalty
                .checked_add(execution_context.fee_cost(platform_version)?.processing_fee)
                .ok_or(ProtocolError::Overflow("processing fee overflow error"))?;

            let bump_action =
                BumpAddressInputNoncesAction::from_identity_create_from_addresses_transition_action(
                    action,
                    used_credits,
                );
            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action.into(),
                unique_public_key_validation_result.errors,
            ))
        }
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match IdentityCreateFromAddressesTransitionAction::try_from_transition(self, input_balances)
        {
            Ok(action) => Ok(ConsensusValidationResult::new_with_data(action.into())),
            Err(error) => Ok(ConsensusValidationResult::new_with_error(error)),
        }
    }
}
