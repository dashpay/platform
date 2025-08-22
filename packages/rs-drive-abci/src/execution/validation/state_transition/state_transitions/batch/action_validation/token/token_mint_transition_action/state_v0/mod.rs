use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::consensus::state::identity::RecipientIdentityDoesNotExistError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityTokenAccountFrozenError, TokenMintPastMaxSupplyError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupActionAccessors;
use dpp::prelude::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::token_event::TokenEvent;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenMintTransitionActionStateValidationV0 {
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
        let main_control_group = token_configuration.main_control_group();
        let validation_result = self.base().validate_group_action(
            rules,
            owner_id,
            contract.owner_id(),
            main_control_group,
            contract.groups(),
            "mint".to_string(),
            token_configuration,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if let Some(original_group_action) = self.base().original_group_action() {
            if let GroupActionEvent::TokenEvent(TokenEvent::Mint(
                old_group_action_amount,
                old_group_action_recipient,
                _,
            )) = original_group_action.event()
            {
                let mut changed_internal_fields = vec![];
                if old_group_action_amount != &self.mint_amount() {
                    changed_internal_fields.push("mint_amount".to_string());
                }
                if old_group_action_recipient != &self.identity_balance_holder_id() {
                    changed_internal_fields.push("recipient".to_string());
                }
                if !changed_internal_fields.is_empty() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                                ModificationOfGroupActionMainParametersNotPermittedError::new(
                                    original_group_action.event().event_name(),
                                    "Token: mint".to_string(),
                                    changed_internal_fields,
                                ),
                            ),
                        ),
                    ));
                }
            } else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                            ModificationOfGroupActionMainParametersNotPermittedError::new(
                                original_group_action.event().event_name(),
                                "Token: mint".to_string(),
                                vec![],
                            ),
                        ),
                    ),
                ));
            }
        }

        if let Some(max_supply) = token_configuration.max_supply() {
            // We have a max supply, let's get the current supply
            let (token_total_supply, fee) = platform.drive.fetch_token_total_supply_with_cost(
                self.token_id().to_buffer(),
                block_info,
                transaction,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            if let Some(token_total_supply) = token_total_supply {
                if let Some(total_supply_after_mint) =
                    token_total_supply.checked_add(self.mint_amount())
                {
                    if total_supply_after_mint > max_supply {
                        // We are trying to set a max supply smaller than the token total supply
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(
                                TokenMintPastMaxSupplyError::new(
                                    self.token_id(),
                                    self.mint_amount(),
                                    token_total_supply,
                                    max_supply,
                                ),
                            )),
                        ));
                    }
                } else {
                    // if we overflow we would also always go over max supply
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(
                            TokenMintPastMaxSupplyError::new(
                                self.token_id(),
                                self.mint_amount(),
                                token_total_supply,
                                max_supply,
                            ),
                        )),
                    ));
                }
            } else {
                return Err(Error::Drive(drive::error::Error::Drive(
                    DriveError::CorruptedDriveState(format!(
                        "token {} total supply not found",
                        self.token_id()
                    )),
                )));
            }
        }

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

        // We need to verify that account we are transferring to not frozen
        if !self
            .base()
            .token_configuration()?
            .is_allowed_transfer_to_frozen_balance()
        {
            let (info, fee_result) = platform.drive.fetch_identity_token_info_with_costs(
                self.token_id().to_buffer(),
                recipient.to_buffer(),
                block_info,
                true,
                transaction,
                platform_version,
            )?;

            execution_context
                .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

            if let Some(info) = info {
                if info.frozen() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(
                            IdentityTokenAccountFrozenError::new(
                                self.token_id(),
                                recipient,
                                "mint".to_string(),
                            ),
                        )),
                    ));
                }
            };
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
