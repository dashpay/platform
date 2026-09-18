use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::{
    charge_drive_operations, validate_minimum_token_pool_notes, validate_token_not_paused,
    validate_token_pool_anchor_exists, validate_token_pool_nullifiers,
    validate_token_shielded_pool_enabled, verify_token_pool_bundle,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::FLAGS_SPENDS_AND_OUTPUTS;
use dpp::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupActionAccessors;
use dpp::shielded::{serialized_actions_digest, token_burn_from_pool_extra_sighash_data};
use dpp::tokens::token_event::TokenEvent;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_burn_from_pool_transition_action::{
    TokenBurnFromPoolTransitionAction, TokenBurnFromPoolTransitionActionAccessorsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenBurnFromPoolTransitionActionStateValidationV0 {
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenBurnFromPoolTransitionActionStateValidationV0 for TokenBurnFromPoolTransitionAction {
    /// Runs the transparent burn's authorization (group action included, with the actions digest
    /// as a group parameter), then the pool spend checks (paused, anchor, nullifiers, balance)
    /// and verifies the spend bundle whose value balance is `+amount`, bound to the token, the
    /// burner (the batch owner, or the proposer of a group action) and the amount.
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.base().validate_state(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let validation_result = validate_token_shielded_pool_enabled(self.base())?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let token_id = self.token_id();

        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;
        let rules = token_configuration.manual_burning_rules();
        let main_control_group = token_configuration.main_control_group();
        let validation_result = self.base().validate_group_action(
            rules,
            owner_id,
            contract.owner_id(),
            main_control_group,
            contract.groups(),
            "burnFromPool".to_string(),
            token_configuration,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Every signer of a group action must commit to the same amount and the same notes.
        if let Some(original_group_action) = self.base().original_group_action() {
            let changed_internal_fields = match original_group_action.event() {
                GroupActionEvent::TokenEvent(TokenEvent::BurnFromPool(
                    old_group_action_amount,
                    old_actions_digest,
                    _,
                )) => {
                    let mut changed = vec![];
                    if old_group_action_amount != &self.amount() {
                        changed.push("amount".to_string());
                    }
                    if old_actions_digest.as_bytes() != &serialized_actions_digest(self.actions()) {
                        changed.push("actions".to_string());
                    }
                    changed
                }
                _ => vec![],
            };
            let same_event = matches!(
                original_group_action.event(),
                GroupActionEvent::TokenEvent(TokenEvent::BurnFromPool(..))
            );
            if !same_event || !changed_internal_fields.is_empty() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                            ModificationOfGroupActionMainParametersNotPermittedError::new(
                                original_group_action.event().event_name(),
                                "Token: burnFromPool".to_string(),
                                changed_internal_fields,
                            ),
                        ),
                    ),
                ));
            }
        }

        let validation_result = validate_token_not_paused(
            platform,
            token_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let token_id_bytes = token_id.to_buffer();
        let mut drive_operations = vec![];

        let validation_result = validate_minimum_token_pool_notes(
            platform.drive,
            &token_id_bytes,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            charge_drive_operations(
                platform.drive,
                block_info,
                execution_context,
                drive_operations,
                platform_version,
            )?;
            return Ok(validation_result);
        }

        let validation_result = validate_token_pool_anchor_exists(
            platform.drive,
            &token_id_bytes,
            self.anchor(),
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            charge_drive_operations(
                platform.drive,
                block_info,
                execution_context,
                drive_operations,
                platform_version,
            )?;
            return Ok(validation_result);
        }

        let validation_result = validate_token_pool_nullifiers(
            platform.drive,
            &token_id_bytes,
            &self.nullifiers(),
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            charge_drive_operations(
                platform.drive,
                block_info,
                execution_context,
                drive_operations,
                platform_version,
            )?;
            return Ok(validation_result);
        }

        let pool_balance = platform.drive.read_token_shielded_pool_total_balance(
            &token_id_bytes,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        charge_drive_operations(
            platform.drive,
            block_info,
            execution_context,
            drive_operations,
            platform_version,
        )?;

        if pool_balance < self.amount() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                    "token shielded pool has insufficient balance: pool has {} but burn requires {}",
                    pool_balance,
                    self.amount()
                )))
                .into(),
            ));
        }

        // A group action's bundle is proven once by the proposer and submitted unchanged by every
        // other signer (the digest check above pins it), so the sighash binds the proposer rather
        // than the batch owner. A direct burn's proposer is the batch owner.
        let burner_id = match self.base().original_group_action() {
            Some(original_group_action) => original_group_action.proposer_id(),
            None => owner_id,
        };
        let extra_sighash_data = token_burn_from_pool_extra_sighash_data(
            &token_id_bytes,
            &burner_id.to_buffer(),
            self.amount(),
            platform_version,
        )?;

        verify_token_pool_bundle(
            validation_mode,
            self.actions(),
            FLAGS_SPENDS_AND_OUTPUTS,
            self.amount() as i64,
            self.anchor(),
            self.proof(),
            self.binding_signature(),
            &extra_sighash_data,
        )
    }
}
