use std::collections::BTreeMap;

use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::UnauthorizedTokenActionError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::change_control_rules::ChangeControlRules;
use dpp::data_contract::group::Group;
use dpp::data_contract::{GroupContractPosition, TokenContractPosition};
use dpp::group::group_action::GroupActionAccessors;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::state_v0::TokenBaseTransitionActionStateValidationV0;
use crate::platform_types::platform::PlatformStateRef;

fn changed_group_action_base_fields(
    original_contract_id: Identifier,
    original_token_position: TokenContractPosition,
    current_contract_id: Identifier,
    current_token_position: TokenContractPosition,
) -> Vec<String> {
    let mut changed_fields = Vec::new();

    if original_contract_id != current_contract_id {
        changed_fields.push("data_contract_id".to_string());
    }

    if original_token_position != current_token_position {
        changed_fields.push("token_contract_position".to_string());
    }

    changed_fields
}

fn main_group_authorizes_selected_group(
    main_control_group: Option<GroupContractPosition>,
    selected_group: GroupContractPosition,
) -> bool {
    main_control_group == Some(selected_group)
}

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenBaseTransitionActionStateValidationV1 {
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    #[allow(clippy::too_many_arguments)]
    fn validate_group_action_v1(
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

impl TokenBaseTransitionActionStateValidationV1 for TokenBaseTransitionAction {
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if let Some(original_group_action) = self.original_group_action() {
            let changed_fields = changed_group_action_base_fields(
                original_group_action.contract_id(),
                original_group_action.token_contract_position(),
                self.data_contract_id(),
                self.token_position(),
            );

            if !changed_fields.is_empty() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                            ModificationOfGroupActionMainParametersNotPermittedError::new(
                                original_group_action.event().event_name(),
                                format!(
                                    "{} at contract {} token position {}",
                                    original_group_action.event().event_name(),
                                    self.data_contract_id(),
                                    self.token_position()
                                ),
                                changed_fields,
                            ),
                        ),
                    ),
                ));
            }
        }

        self.validate_state_v0(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )
    }

    fn validate_group_action_v1(
        &self,
        rules: &ChangeControlRules,
        owner_id: Identifier,
        contract_owner_id: Identifier,
        main_control_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_type_string: String,
        token_configuration: &TokenConfiguration,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if let Some(resolved_group_info) = self.store_in_group() {
            if rules.authorized_to_make_change_action_takers() == &AuthorizedActionTakers::MainGroup
                && !main_group_authorizes_selected_group(
                    token_configuration.main_control_group(),
                    resolved_group_info.group_contract_position,
                )
            {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                        UnauthorizedTokenActionError::new(
                            self.token_id(),
                            owner_id,
                            action_type_string,
                            AuthorizedActionTakers::MainGroup,
                        ),
                    )),
                ));
            }
        }

        self.validate_group_action_v0(
            rules,
            owner_id,
            contract_owner_id,
            main_control_group,
            groups,
            action_type_string,
            token_configuration,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_action_base_fields_bind_contract_and_token_position() {
        let original_contract_id = Identifier::from([1; 32]);
        let other_contract_id = Identifier::from([2; 32]);

        assert!(
            changed_group_action_base_fields(original_contract_id, 7, original_contract_id, 7,)
                .is_empty()
        );
        assert_eq!(
            changed_group_action_base_fields(original_contract_id, 7, other_contract_id, 8,),
            vec![
                "data_contract_id".to_string(),
                "token_contract_position".to_string(),
            ]
        );
    }

    #[test]
    fn main_group_authorization_fails_closed() {
        assert!(main_group_authorizes_selected_group(Some(3), 3));
        assert!(!main_group_authorizes_selected_group(Some(3), 4));
        assert!(!main_group_authorizes_selected_group(None, 3));
    }
}
