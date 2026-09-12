use super::v0::IdentityUpdateStateTransitionStateValidationV0;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;

use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_identity_public_key_contract_bounds::validate_identity_public_keys_contract_bounds;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_dont_exist_in_state::validate_identity_public_key_ids_dont_exist_in_state;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_exist_in_state::validate_identity_public_key_ids_exist_in_state;
use crate::execution::validation::state_transition::common::validate_not_disabling_last_master_key::validate_master_key_uniqueness;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_not_in_state;

pub(in crate::execution::validation::state_transition::state_transitions::identity_update) trait IdentityUpdateStateTransitionStateValidationV1
{
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        state_transition_execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityUpdateStateTransitionStateValidationV1 for IdentityUpdateTransition {
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        state_transition_execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        // Now we should check the state of added keys to make sure there aren't any that already exist
        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_not_in_state(
                self.public_keys_to_add(),
                drive,
                state_transition_execution_context,
                tx,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        validation_result.add_errors(
            validate_identity_public_key_ids_dont_exist_in_state(
                self.identity_id(),
                self.public_keys_to_add(),
                drive,
                tx,
                state_transition_execution_context,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        // Now we should check to make sure any keys that are added are valid for the contract
        // bounds they refer to
        validation_result.add_errors(
            validate_identity_public_keys_contract_bounds(
                self.identity_id(),
                self.public_keys_to_add(),
                drive,
                &block_info.epoch,
                block_info.time_ms,
                tx,
                state_transition_execution_context,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        if !self.public_key_ids_to_disable().is_empty() {
            let validation_result_and_keys_to_disable =
                validate_identity_public_key_ids_exist_in_state(
                    self.identity_id(),
                    self.public_key_ids_to_disable(),
                    drive,
                    state_transition_execution_context,
                    tx,
                    platform_version,
                )?;
            // We need to validate that all keys removed existed
            if !validation_result_and_keys_to_disable.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result_and_keys_to_disable.errors,
                ));
            }

            let keys_to_disable = validation_result_and_keys_to_disable.into_data()?;

            let validation_result = validate_master_key_uniqueness(
                self.public_keys_to_add(),
                keys_to_disable.as_slice(),
                platform_version,
            )?;
            if !validation_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }
        }
        self.transform_into_action_v0()
    }
}
