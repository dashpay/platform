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
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::{
    validate_token_shielded_pool_enabled, verify_token_pool_bundle,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::FLAGS_OUTPUTS_ONLY;
use dpp::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::TokenMintPastMaxSupplyError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupActionAccessors;
use dpp::shielded::serialized_actions_digest;
use dpp::tokens::token_event::TokenEvent;
use drive::error::drive::DriveError;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_mint_to_pool_transition_action::{
    TokenMintToPoolTransitionAction, TokenMintToPoolTransitionActionAccessorsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenMintToPoolTransitionActionStateValidationV0 {
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

impl TokenMintToPoolTransitionActionStateValidationV0 for TokenMintToPoolTransitionAction {
    /// Runs the transparent mint's authorization (group action included, with the actions digest
    /// as a group parameter) and max supply checks, then verifies the outputs-only bundle whose
    /// value balance is `-amount`. No identity balance is involved.
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
        let rules = token_configuration.manual_minting_rules();
        let main_control_group = token_configuration.main_control_group();
        let validation_result = self.base().validate_group_action(
            rules,
            owner_id,
            contract.owner_id(),
            main_control_group,
            contract.groups(),
            "mintToPool".to_string(),
            token_configuration,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Every signer of a group action must commit to the same amount and the same notes.
        if let Some(original_group_action) = self.base().original_group_action() {
            let changed_internal_fields = match original_group_action.event() {
                GroupActionEvent::TokenEvent(TokenEvent::MintToPool(
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
                GroupActionEvent::TokenEvent(TokenEvent::MintToPool(..))
            );
            if !same_event || !changed_internal_fields.is_empty() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                            ModificationOfGroupActionMainParametersNotPermittedError::new(
                                original_group_action.event().event_name(),
                                "Token: mintToPool".to_string(),
                                changed_internal_fields,
                            ),
                        ),
                    ),
                ));
            }
        }

        if let Some(max_supply) = token_configuration.max_supply() {
            let (token_total_supply, fee) = platform.drive.fetch_token_total_supply_with_cost(
                token_id.to_buffer(),
                block_info,
                transaction,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            let Some(token_total_supply) = token_total_supply else {
                return Err(Error::Drive(drive::error::Error::Drive(
                    DriveError::CorruptedDriveState(format!(
                        "token {} total supply not found",
                        token_id
                    )),
                )));
            };
            match token_total_supply.checked_add(self.amount()) {
                Some(total_supply_after) if total_supply_after <= max_supply => {}
                _ => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(
                            TokenMintPastMaxSupplyError::new(
                                token_id,
                                self.amount(),
                                token_total_supply,
                                max_supply,
                            ),
                        )),
                    ));
                }
            }
        }

        verify_token_pool_bundle(
            validation_mode,
            self.actions(),
            FLAGS_OUTPUTS_ONLY,
            -(self.amount() as i64),
            self.anchor(),
            self.proof(),
            self.binding_signature(),
            &[],
        )
    }
}
