use crate::execution::validation::state_transition::common::validate_identity_public_key_contract_bounds::validate_identity_public_keys_contract_bounds;
use crate::execution::validation::state_transition::common::validate_identity_public_keys_limits::validate_identity_public_keys_limits;
use dpp::block::block_info::BlockInfo;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;

use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;

use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::StateTransitionIdentityIdFromInputs;
use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::identity_create_from_addresses::bump_input_nonces_with_penalty;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_not_in_state;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create_from_addresses) trait IdentityCreateFromAddressesStateTransitionStateValidationV1
{
    fn validate_state_v1<C>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        action: IdentityCreateFromAddressesTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateFromAddressesStateTransitionStateValidationV1
    for IdentityCreateFromAddressesTransition
{
    fn validate_state_v1<C>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
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
        let mut key_state_validation_result =
            validate_unique_identity_public_key_hashes_not_in_state(
                self.public_keys(),
                drive,
                execution_context,
                transaction,
                platform_version,
            )?;

        key_state_validation_result.add_errors(
            validate_identity_public_keys_contract_bounds(
                identity_id,
                self.public_keys(),
                drive,
                &block_info.epoch,
                transaction,
                execution_context,
                platform_version,
            )?
            .errors,
        );

        // A key must not already be expired in the block that registers it
        key_state_validation_result.add_errors(
            validate_identity_public_keys_limits(self.public_keys(), block_info, platform_version)?
                .errors,
        );

        if key_state_validation_result.is_valid() {
            // We just pass the action that was given to us
            Ok(ConsensusValidationResult::new_with_data(
                StateTransitionAction::IdentityCreateFromAddressesAction(action),
            ))
        } else {
            // It's not valid: the inputs only bump their nonces and pay the penalty
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .unique_key_already_present;

            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_input_nonces_with_penalty(self, &action, penalty)?,
                key_state_validation_result.errors,
            ))
        }
    }
}
