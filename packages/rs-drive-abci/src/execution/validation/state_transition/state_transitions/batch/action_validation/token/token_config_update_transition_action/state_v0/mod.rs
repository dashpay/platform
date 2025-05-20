use dpp::block::block_info::BlockInfo;
use dpp::errors::consensus::ConsensusError;
use dpp::errors::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::errors::consensus::state::state_error::StateError;
use dpp::errors::consensus::state::token::{InvalidGroupPositionError, NewAuthorizedActionTakerGroupDoesNotExistError, NewAuthorizedActionTakerIdentityDoesNotExistError, NewAuthorizedActionTakerMainGroupNotSetError, NewTokensDestinationIdentityDoesNotExistError, TokenSettingMaxSupplyToLessThanCurrentSupplyError, UnauthorizedTokenActionError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::action_taker::{ActionGoal, ActionTaker};
use dpp::group::group_action::GroupActionAccessors;
use dpp::prelude::Identifier;
use dpp::tokens::token_event::TokenEvent;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenConfigUpdateTransitionActionStateValidationV0 {
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
impl TokenConfigUpdateTransitionActionStateValidationV0 for TokenConfigUpdateTransitionAction {
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
        let main_control_group = token_configuration.main_control_group();

        if let Some(original_group_action) = self.base().original_group_action() {
            if let GroupActionEvent::TokenEvent(TokenEvent::ConfigUpdate(
                old_config_update_change_item,
                _,
            )) = original_group_action.event()
            {
                let mut changed_internal_fields = vec![];
                if old_config_update_change_item != self.update_token_configuration_item() {
                    changed_internal_fields.push("update_token_configuration_item".to_string());
                }
                if !changed_internal_fields.is_empty() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                                ModificationOfGroupActionMainParametersNotPermittedError::new(
                                    original_group_action.event().event_name(),
                                    "Token: configUpdate".to_string(),
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
                                "Token: configUpdate".to_string(),
                                vec![],
                            ),
                        ),
                    ),
                ));
            }
        }

        let goal = if self.base().store_in_group().is_some() {
            // We are using a group
            // We just need to be able to participate
            ActionGoal::ActionParticipation
        } else {
            // We are not using a group
            // We need to make sure that for the change we are doing that we can finish it
            ActionGoal::ActionCompletion
        };

        if !token_configuration.can_apply_token_configuration_item(
            self.update_token_configuration_item(),
            &contract.owner_id(),
            main_control_group,
            contract.groups(),
            &ActionTaker::SingleIdentity(owner_id),
            goal,
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                    UnauthorizedTokenActionError::new(
                        self.token_id(),
                        owner_id,
                        "config_update".to_string(),
                        token_configuration.authorized_action_takers_for_configuration_item(
                            self.update_token_configuration_item(),
                        ),
                    ),
                )),
            ));
        }

        match self.update_token_configuration_item() {
            TokenConfigurationChangeItem::MaxSupply(Some(max_supply)) => {
                // If we are setting a max supply we need to make sure it isn't less than the
                // current supply of the token
                let (token_total_supply, fee) = platform.drive.fetch_token_total_supply_with_cost(
                    self.token_id().to_buffer(),
                    block_info,
                    transaction,
                    platform_version,
                )?;
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
                if let Some(token_total_supply) = token_total_supply {
                    if token_total_supply > *max_supply {
                        // We are trying to set a max supply smaller than the token total supply
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(
                                StateError::TokenSettingMaxSupplyToLessThanCurrentSupplyError(
                                    TokenSettingMaxSupplyToLessThanCurrentSupplyError::new(
                                        self.token_id(),
                                        *max_supply,
                                        token_total_supply,
                                    ),
                                ),
                            ),
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
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(identity_id)) => {
                // We need to make sure the identity exists
                let (identity_balance, fee) = platform.drive.fetch_identity_balance_with_costs(
                    identity_id.to_buffer(),
                    block_info,
                    true,
                    transaction,
                    platform_version,
                )?;
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
                if identity_balance.is_none() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::NewTokensDestinationIdentityDoesNotExistError(
                                NewTokensDestinationIdentityDoesNotExistError::new(*identity_id),
                            ),
                        ),
                    ));
                }
            }
            TokenConfigurationChangeItem::MainControlGroup(Some(control_group)) => {
                if !self
                    .data_contract_fetch_info()
                    .contract
                    .groups()
                    .contains_key(control_group)
                {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::InvalidGroupPositionError(
                            InvalidGroupPositionError::new(
                                self.data_contract_fetch_info()
                                    .contract
                                    .groups()
                                    .keys()
                                    .last()
                                    .copied(),
                                *control_group,
                            ),
                        )),
                    ));
                }
            }
            TokenConfigurationChangeItem::ConventionsControlGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::ConventionsAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::MaxSupplyControlGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::MaxSupplyAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::Identity(
                identity_id,
            ))
            | TokenConfigurationChangeItem::ManualMintingAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::ManualBurning(AuthorizedActionTakers::Identity(
                identity_id,
            ))
            | TokenConfigurationChangeItem::ManualBurningAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::Freeze(AuthorizedActionTakers::Identity(identity_id))
            | TokenConfigurationChangeItem::FreezeAdminGroup(AuthorizedActionTakers::Identity(
                identity_id,
            ))
            | TokenConfigurationChangeItem::Unfreeze(AuthorizedActionTakers::Identity(
                identity_id,
            ))
            | TokenConfigurationChangeItem::UnfreezeAdminGroup(AuthorizedActionTakers::Identity(
                identity_id,
            ))
            | TokenConfigurationChangeItem::DestroyFrozenFunds(AuthorizedActionTakers::Identity(
                identity_id,
            ))
            | TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            )
            | TokenConfigurationChangeItem::EmergencyAction(AuthorizedActionTakers::Identity(
                identity_id,
            ))
            | TokenConfigurationChangeItem::EmergencyActionAdminGroup(
                AuthorizedActionTakers::Identity(identity_id),
            ) => {
                let (identity_balance, fee) = platform.drive.fetch_identity_balance_with_costs(
                    identity_id.to_buffer(),
                    block_info,
                    true,
                    transaction,
                    platform_version,
                )?;
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
                if identity_balance.is_none() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::NewAuthorizedActionTakerIdentityDoesNotExistError(
                                NewAuthorizedActionTakerIdentityDoesNotExistError::new(
                                    *identity_id,
                                ),
                            ),
                        ),
                    ));
                }
            }
            TokenConfigurationChangeItem::ConventionsControlGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::MaxSupplyControlGroup(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::MaxSupplyAdminGroup(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::ManualMintingAdminGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::ManualBurning(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::ManualBurningAdminGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::Freeze(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::FreezeAdminGroup(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::Unfreeze(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::UnfreezeAdminGroup(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::DestroyFrozenFunds(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            )
            | TokenConfigurationChangeItem::EmergencyAction(AuthorizedActionTakers::Group(
                group_contract_position,
            ))
            | TokenConfigurationChangeItem::EmergencyActionAdminGroup(
                AuthorizedActionTakers::Group(group_contract_position),
            ) => {
                if !self
                    .data_contract_fetch_info()
                    .contract
                    .groups()
                    .contains_key(group_contract_position)
                {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::NewAuthorizedActionTakerGroupDoesNotExistError(
                                NewAuthorizedActionTakerGroupDoesNotExistError::new(
                                    *group_contract_position,
                                ),
                            ),
                        ),
                    ));
                }
            }
            TokenConfigurationChangeItem::ConventionsControlGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::ConventionsAdminGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::MaxSupplyControlGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::MaxSupplyAdminGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::ManualMintingAdminGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::ManualBurning(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::ManualBurningAdminGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::Freeze(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::FreezeAdminGroup(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::Unfreeze(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::UnfreezeAdminGroup(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::DestroyFrozenFunds(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(
                AuthorizedActionTakers::MainGroup,
            )
            | TokenConfigurationChangeItem::EmergencyAction(AuthorizedActionTakers::MainGroup)
            | TokenConfigurationChangeItem::EmergencyActionAdminGroup(
                AuthorizedActionTakers::MainGroup,
            ) => {
                if token_configuration.main_control_group().is_none() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::NewAuthorizedActionTakerMainGroupNotSetError(
                                NewAuthorizedActionTakerMainGroupNotSetError::new(),
                            ),
                        ),
                    ));
                }
            }
            _ => {}
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
