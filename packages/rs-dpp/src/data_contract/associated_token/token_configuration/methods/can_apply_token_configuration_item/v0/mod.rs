use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
use platform_value::Identifier;
use std::collections::BTreeMap;

impl TokenConfigurationV0 {
    /// Determines whether a `TokenConfigurationChangeItem` can be applied to this token configuration.
    ///
    /// # Parameters
    /// - `change_item`: The change item to evaluate.
    /// - `contract_owner_id`: The ID of the contract owner.
    /// - `main_group`: The main control group position, if any.
    /// - `groups`: A map of group positions to their respective `Group` instances.
    /// - `action_taker`: The entity attempting the action.
    /// - `goal`: The goal of the action being attempted.
    ///
    /// Returns `true` if the change item can be applied, `false` otherwise.
    pub fn can_apply_token_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match change_item {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => false,
            TokenConfigurationChangeItem::Conventions(_) => self
                .conventions_change_rules
                .can_make_change(contract_owner_id, main_group, groups, action_taker, goal),
            TokenConfigurationChangeItem::ConventionsControlGroup(control_group) => self
                .conventions_change_rules
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::ConventionsAdminGroup(admin_group) => self
                .conventions_change_rules
                .can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::MaxSupply(_) => self
                .max_supply_change_rules
                .can_make_change(contract_owner_id, main_group, groups, action_taker, goal),
            TokenConfigurationChangeItem::MaxSupplyControlGroup(control_group) => self
                .max_supply_change_rules
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(admin_group) => {
                self.max_supply_change_rules.can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::PerpetualDistribution(_) => self
                .distribution_rules
                .perpetual_distribution_rules()
                .can_make_change(contract_owner_id, main_group, groups, action_taker, goal),
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(control_group) => self
                .distribution_rules
                .perpetual_distribution_rules()
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(admin_group) => self
                .distribution_rules
                .perpetual_distribution_rules()
                .can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(_) => self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .can_make_change(contract_owner_id, main_group, groups, action_taker, goal),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                control_group,
            ) => self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(admin_group) => {
                self.distribution_rules
                    .new_tokens_destination_identity_rules()
                    .can_change_admin_action_takers(
                        admin_group,
                        contract_owner_id,
                        main_group,
                        groups,
                        action_taker,
                        goal,
                    )
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(_) => self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .can_make_change(contract_owner_id, main_group, groups, action_taker, goal),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                control_group,
            ) => self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                admin_group,
            ) => self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::ManualMinting(control_group) => self
                .manual_minting_rules
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::ManualMintingAdminGroup(admin_group) => {
                self.manual_minting_rules.can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::ManualBurning(control_group) => self
                .manual_burning_rules
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::ManualBurningAdminGroup(admin_group) => {
                self.manual_burning_rules.can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::Freeze(control_group) => {
                self.freeze_rules.can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::FreezeAdminGroup(admin_group) => {
                self.freeze_rules.can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::Unfreeze(control_group) => {
                self.unfreeze_rules.can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::UnfreezeAdminGroup(admin_group) => {
                self.unfreeze_rules.can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::DestroyFrozenFunds(control_group) => self
                .destroy_frozen_funds_rules
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(admin_group) => self
                .destroy_frozen_funds_rules
                .can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::EmergencyAction(control_group) => self
                .emergency_action_rules
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(admin_group) => {
                self.emergency_action_rules.can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            TokenConfigurationChangeItem::MainControlGroup(_) => self
                .main_control_group_can_be_modified
                .allowed_for_action_taker(
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
        }
    }
}
