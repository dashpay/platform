use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::identity::RecipientIdentityDoesNotExistError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::UnauthorizedTokenActionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::multi_identity_events::ActionTaker;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(super) trait TokenMintTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenMintTransitionActionStateValidationV0 for TokenMintTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
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

        // Let's first check to see if we are authorized to perform this action
        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;
        let rules = token_configuration.manual_minting_rules();
        let main_control_group = token_configuration
            .main_control_group()
            .map(|position| contract.expected_group(position))
            .transpose()?;

        if let Some(resolved_group_info) = self.base().store_in_group() {
            // We are trying to do a group action
            // We have already checked when converting into an action that we are a member of the Group
            // Now we need to just check that the group is the actual group set by the contract
            match rules.authorized_to_make_change_action_takers() {
                AuthorizedActionTakers::NoOne | AuthorizedActionTakers::ContractOwner => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                            UnauthorizedTokenActionError::new(
                                self.token_id(),
                                owner_id,
                                "mint".to_string(),
                                rules.authorized_to_make_change_action_takers().clone(),
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
                                            "mint".to_string(),
                                            rules.authorized_to_make_change_action_takers().clone(),
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
                                    "mint".to_string(),
                                    rules.authorized_to_make_change_action_takers().clone(),
                                ),
                            )),
                        ));
                    }
                }
            }
        } else {
            if !rules.can_make_change(
                &contract.owner_id(),
                main_control_group,
                contract.groups(),
                &ActionTaker::SingleIdentity(owner_id),
            ) {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                        UnauthorizedTokenActionError::new(
                            self.token_id(),
                            owner_id,
                            "mint".to_string(),
                            rules.authorized_to_make_change_action_takers().clone(),
                        ),
                    )),
                ));
            }
        }

        // todo verify that minting would not break max supply

        // We need to verify that the receiver is a valid identity

        let recipient = self.identity_balance_holder_id();
        if recipient != owner_id {
            // We have already checked that this user exists if the recipient is the owner id
            let balance = platform.drive.fetch_identity_balance(
                recipient.to_buffer(),
                transaction,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                RetrieveIdentityInfo::only_balance(),
            ));
            if balance.is_none() {
                // The identity does not exist
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::RecipientIdentityDoesNotExistError(
                        RecipientIdentityDoesNotExistError::new(recipient),
                    )),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
