use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Getters;
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
            TokenConfigurationChangeItem::MarketplaceTradeMode(_) => self
                .marketplace_rules
                .trade_mode_change_rules()
                .can_make_change(contract_owner_id, main_group, groups, action_taker, goal),
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(control_group) => self
                .marketplace_rules
                .trade_mode_change_rules()
                .can_change_authorized_action_takers(
                    control_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(admin_group) => self
                .marketplace_rules
                .trade_mode_change_rules()
                .can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
    use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
    use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use crate::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;

    fn config_owner_can_change_everything() -> TokenConfigurationV0 {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        let owner_rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::ContractOwner,
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true,
        });
        c.set_conventions_change_rules(owner_rules.clone());
        c.set_max_supply_change_rules(owner_rules.clone());
        c.set_manual_minting_rules(owner_rules.clone());
        c.set_manual_burning_rules(owner_rules.clone());
        c.set_freeze_rules(owner_rules.clone());
        c.set_unfreeze_rules(owner_rules.clone());
        c.set_destroy_frozen_funds_rules(owner_rules.clone());
        c.set_emergency_action_rules(owner_rules.clone());
        c.set_main_control_group_can_be_modified(AuthorizedActionTakers::ContractOwner);
        c
    }

    #[test]
    fn no_change_always_returns_false() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::TokenConfigurationNoChange,
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(!can);
    }

    #[test]
    fn conventions_item_allowed_when_owner_authorized() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let conv = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 4,
        });
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::Conventions(conv),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(can);
    }

    #[test]
    fn conventions_item_denied_for_non_owner() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let non_owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(non_owner);
        let conv = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 4,
        });
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::Conventions(conv),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(!can);
    }

    #[test]
    fn conventions_control_group_allowed_when_owner_admin() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::ConventionsControlGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(can);
    }

    #[test]
    fn conventions_admin_group_allowed_when_owner_admin() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::ConventionsAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(can);
    }

    #[test]
    fn max_supply_denied_for_default_no_one_rules() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::MaxSupply(Some(1)),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(!can);
    }

    #[test]
    fn manual_minting_allowed_when_admin_action_takers_match() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::ContractOwner),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(can);
    }

    #[test]
    fn manual_minting_admin_group_allowed_when_self_change_allowed() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::ManualMintingAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(can);
    }

    #[test]
    fn manual_minting_admin_group_denied_when_self_change_not_allowed() {
        let mut c = config_owner_can_change_everything();
        let rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::ContractOwner,
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: false,
        });
        c.set_manual_minting_rules(rules);
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::ManualMintingAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(!can);
    }

    #[test]
    fn freeze_unfreeze_destroy_emergency_all_allowed_when_owner_admin() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let items = vec![
            TokenConfigurationChangeItem::Freeze(AuthorizedActionTakers::ContractOwner),
            TokenConfigurationChangeItem::Unfreeze(AuthorizedActionTakers::ContractOwner),
            TokenConfigurationChangeItem::DestroyFrozenFunds(AuthorizedActionTakers::ContractOwner),
            TokenConfigurationChangeItem::EmergencyAction(AuthorizedActionTakers::ContractOwner),
        ];
        for item in items {
            assert!(
                c.can_apply_token_configuration_item(
                    &item,
                    &owner,
                    None,
                    &BTreeMap::new(),
                    &taker,
                    ActionGoal::ActionCompletion,
                ),
                "expected can_apply true for {:?}",
                item
            );
        }
    }

    #[test]
    fn main_control_group_allowed_when_owner_can_modify() {
        let c = config_owner_can_change_everything();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::MainControlGroup(Some(5)),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(can);
    }

    #[test]
    fn main_control_group_denied_by_default() {
        // default main_control_group_can_be_modified = NoOne
        let c = TokenConfigurationV0::default_most_restrictive();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::MainControlGroup(Some(5)),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(!can);
    }

    #[test]
    fn marketplace_trade_mode_denied_by_default() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        let can = c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::MarketplaceTradeMode(TokenTradeMode::NotTradeable),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        );
        assert!(!can);
    }

    #[test]
    fn perpetual_distribution_and_its_admin_denied_by_default() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        assert!(!c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::PerpetualDistribution(None),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
        assert!(!c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn new_tokens_destination_and_minting_choose_destination_denied_by_default() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let owner = Identifier::random();
        let taker = ActionTaker::SingleIdentity(owner);
        assert!(!c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::NewTokensDestinationIdentity(None),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
        assert!(!c.can_apply_token_configuration_item(
            &TokenConfigurationChangeItem::MintingAllowChoosingDestination(true),
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }
}
