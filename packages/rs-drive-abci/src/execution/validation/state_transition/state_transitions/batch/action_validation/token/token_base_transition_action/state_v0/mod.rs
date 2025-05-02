use std::collections::BTreeMap;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::group::GroupActionAlreadySignedByIdentityError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::UnauthorizedTokenActionError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::change_control_rules::ChangeControlRules;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::group::action_taker::{ActionGoal, ActionTaker};
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenBaseTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

#[allow(clippy::too_many_arguments)]
    fn validate_group_action_v0(
        &self,
        rules: &ChangeControlRules,
        owner_id: Identifier,
        contract_owner_id: Identifier,
        main_control_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_type_string: String,
        token_configuration: &TokenConfiguration,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenBaseTransitionActionStateValidationV0 for TokenBaseTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // We should start by validating that if we did not yet sign
        if let Some(group_state_transition_resolved_info) = self.store_in_group() {
            let (already_signed, cost) = platform.drive.fetch_action_id_has_signer_with_costs(
                self.data_contract_id(),
                group_state_transition_resolved_info.group_contract_position,
                group_state_transition_resolved_info.action_id,
                owner_id,
                block_info,
                transaction,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(cost));
            if already_signed {
                // We already have signed this state transition group action
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::GroupActionAlreadySignedByIdentityError(
                            GroupActionAlreadySignedByIdentityError::new(
                                owner_id,
                                self.data_contract_id(),
                                group_state_transition_resolved_info.group_contract_position,
                                group_state_transition_resolved_info.action_id,
                            ),
                        ),
                    ),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }

    fn validate_group_action_v0(
        &self,
        rules: &ChangeControlRules,
        owner_id: Identifier,
        contract_owner_id: Identifier,
        main_control_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_type_string: String,
        token_configuration: &TokenConfiguration,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // We should start by validating that if we did not yet sign
        if let Some(resolved_group_info) = self.store_in_group() {
            // We are trying to do a group action
            // We have already checked when converting into an action that we are a member of the Group
            // Now we need to just check that the group is the actual group set by the contract
            match rules.authorized_to_make_change_action_takers() {
                AuthorizedActionTakers::NoOne
                | AuthorizedActionTakers::ContractOwner
                | AuthorizedActionTakers::Identity(_) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                            UnauthorizedTokenActionError::new(
                                self.token_id(),
                                owner_id,
                                action_type_string,
                                *rules.authorized_to_make_change_action_takers(),
                            ),
                        )),
                    ))
                }
                AuthorizedActionTakers::MainGroup => {
                    if let Some(main_control_group_contract_position) =
                        token_configuration.main_control_group()
                    {
                        if main_control_group_contract_position
                            != resolved_group_info.group_contract_position
                        {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                ConsensusError::StateError(
                                    StateError::UnauthorizedTokenActionError(
                                        UnauthorizedTokenActionError::new(
                                            self.token_id(),
                                            owner_id,
                                            action_type_string,
                                            *rules.authorized_to_make_change_action_takers(),
                                        ),
                                    ),
                                ),
                            ));
                        }
                    }
                }
                AuthorizedActionTakers::Group(group_contract_position) => {
                    if *group_contract_position != resolved_group_info.group_contract_position {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                                UnauthorizedTokenActionError::new(
                                    self.token_id(),
                                    owner_id,
                                    action_type_string,
                                    *rules.authorized_to_make_change_action_takers(),
                                ),
                            )),
                        ));
                    }
                }
            }
        } else if !rules.can_make_change(
            &contract_owner_id,
            main_control_group,
            groups,
            &ActionTaker::SingleIdentity(owner_id),
            ActionGoal::ActionCompletion,
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                    UnauthorizedTokenActionError::new(
                        self.token_id(),
                        owner_id,
                        action_type_string,
                        *rules.authorized_to_make_change_action_takers(),
                    ),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
