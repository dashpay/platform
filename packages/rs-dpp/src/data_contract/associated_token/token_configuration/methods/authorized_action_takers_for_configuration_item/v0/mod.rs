use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Getters;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
impl TokenConfigurationV0 {
    /// Returns the current authority that controls a configuration item.
    ///
    /// Unlike `authorized_action_takers_for_configuration_item`, this includes
    /// the rule controlling changes to the main control group itself.
    pub fn controlling_action_takers_for_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
    ) -> AuthorizedActionTakers {
        match change_item {
            TokenConfigurationChangeItem::MainControlGroup(_) => {
                self.main_control_group_can_be_modified
            }
            _ => self.authorized_action_takers_for_configuration_item(change_item),
        }
    }

    /// Returns the authorized action takers for a specific `TokenConfigurationChangeItem`.
    ///
    /// # Parameters
    /// - `change_item`: The change item for which to retrieve the authorized action takers.
    ///
    /// # Returns
    /// - `AuthorizedActionTakers`: The authorized action takers for the given change item.
    pub fn authorized_action_takers_for_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
    ) -> AuthorizedActionTakers {
        match change_item {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => {
                AuthorizedActionTakers::NoOne
            }
            TokenConfigurationChangeItem::Conventions(_) => *self
                .conventions_change_rules
                .authorized_to_make_change_action_takers(),
            TokenConfigurationChangeItem::ConventionsControlGroup(_) => {
                *self.conventions_change_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::ConventionsAdminGroup(_) => {
                *self.conventions_change_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::MaxSupply(_) => *self
                .max_supply_change_rules
                .authorized_to_make_change_action_takers(),
            TokenConfigurationChangeItem::MaxSupplyControlGroup(_) => {
                *self.max_supply_change_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(_) => {
                *self.max_supply_change_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::PerpetualDistribution(_) => *self
                .distribution_rules
                .perpetual_distribution_rules()
                .authorized_to_make_change_action_takers(),
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(_) => *self
                .distribution_rules
                .perpetual_distribution_rules()
                .admin_action_takers(),
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(_) => *self
                .distribution_rules
                .perpetual_distribution_rules()
                .admin_action_takers(),
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(_) => *self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .authorized_to_make_change_action_takers(),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(_) => *self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .admin_action_takers(),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(_) => *self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .admin_action_takers(),
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(_) => *self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .authorized_to_make_change_action_takers(),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(_) => *self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .admin_action_takers(),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(_) => *self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .admin_action_takers(),
            TokenConfigurationChangeItem::ManualMinting(_) => {
                *self.manual_minting_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::ManualMintingAdminGroup(_) => {
                *self.manual_minting_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::ManualBurning(_) => {
                *self.manual_burning_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::ManualBurningAdminGroup(_) => {
                *self.manual_burning_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::Freeze(_) => *self.freeze_rules.admin_action_takers(),
            TokenConfigurationChangeItem::FreezeAdminGroup(_) => {
                *self.freeze_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::Unfreeze(_) => *self.unfreeze_rules.admin_action_takers(),
            TokenConfigurationChangeItem::UnfreezeAdminGroup(_) => {
                *self.unfreeze_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::DestroyFrozenFunds(_) => {
                *self.destroy_frozen_funds_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(_) => {
                *self.destroy_frozen_funds_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::EmergencyAction(_) => {
                *self.emergency_action_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(_) => {
                *self.emergency_action_rules.admin_action_takers()
            }
            TokenConfigurationChangeItem::MainControlGroup(_) => AuthorizedActionTakers::NoOne,
            TokenConfigurationChangeItem::MarketplaceTradeMode(_) => *self
                .marketplace_rules
                .trade_mode_change_rules()
                .authorized_to_make_change_action_takers(),
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(_) => *self
                .marketplace_rules
                .trade_mode_change_rules()
                .admin_action_takers(),
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(_) => *self
                .marketplace_rules
                .trade_mode_change_rules()
                .admin_action_takers(),
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
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;
    use platform_value::Identifier;

    fn config_with_all_owner_rules() -> TokenConfigurationV0 {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        // Assign each rule's authorized_to_make_change / admin_action_takers to a
        // distinguishable value so we can verify dispatch.
        let auth = AuthorizedActionTakers::ContractOwner;
        let admin = AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]));
        let rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: auth,
            admin_action_takers: admin,
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true,
        });
        c.set_conventions_change_rules(rules.clone());
        c.set_max_supply_change_rules(rules.clone());
        c.set_manual_minting_rules(rules.clone());
        c.set_manual_burning_rules(rules.clone());
        c.set_freeze_rules(rules.clone());
        c.set_unfreeze_rules(rules.clone());
        c.set_destroy_frozen_funds_rules(rules.clone());
        c.set_emergency_action_rules(rules.clone());
        c
    }

    #[test]
    fn no_change_returns_no_one() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::TokenConfigurationNoChange,
        );
        assert_eq!(result, AuthorizedActionTakers::NoOne);
    }

    #[test]
    fn conventions_item_returns_authorized_to_make_change() {
        let c = config_with_all_owner_rules();
        let conv = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 8,
        });
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::Conventions(conv),
        );
        assert_eq!(result, AuthorizedActionTakers::ContractOwner);
    }

    #[test]
    fn conventions_control_group_returns_admin_action_takers() {
        let c = config_with_all_owner_rules();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::ConventionsControlGroup(AuthorizedActionTakers::NoOne),
        );
        // conventions_change_rules.admin_action_takers = Identity(9)
        assert_eq!(
            result,
            AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]))
        );
    }

    #[test]
    fn conventions_admin_group_returns_admin_action_takers() {
        let c = config_with_all_owner_rules();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::NoOne),
        );
        assert_eq!(
            result,
            AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]))
        );
    }

    #[test]
    fn max_supply_returns_authorized_to_make_change() {
        let c = config_with_all_owner_rules();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::MaxSupply(Some(42)),
        );
        assert_eq!(result, AuthorizedActionTakers::ContractOwner);
    }

    #[test]
    fn max_supply_control_and_admin_return_admin() {
        let c = config_with_all_owner_rules();
        let expected = AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]));
        let r1 = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::MaxSupplyControlGroup(AuthorizedActionTakers::NoOne),
        );
        let r2 = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::MaxSupplyAdminGroup(AuthorizedActionTakers::NoOne),
        );
        assert_eq!(r1, expected);
        assert_eq!(r2, expected);
    }

    #[test]
    fn manual_minting_returns_admin_not_authorized() {
        // Note: ManualMinting in authorized_action_takers_for_configuration_item
        // returns admin_action_takers (not authorized_to_make_change) per the
        // implementation. Ensure that dispatch is correct.
        let c = config_with_all_owner_rules();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::NoOne),
        );
        assert_eq!(
            result,
            AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]))
        );
    }

    #[test]
    fn freeze_returns_admin_action_takers() {
        let c = config_with_all_owner_rules();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::Freeze(AuthorizedActionTakers::NoOne),
        );
        assert_eq!(
            result,
            AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]))
        );
    }

    #[test]
    fn destroy_frozen_funds_returns_admin_action_takers() {
        let c = config_with_all_owner_rules();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::DestroyFrozenFunds(AuthorizedActionTakers::NoOne),
        );
        assert_eq!(
            result,
            AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]))
        );
    }

    #[test]
    fn main_control_group_always_returns_no_one() {
        // Per implementation, MainControlGroup change items always return NoOne,
        // regardless of config. This is important because modifying the main
        // control group is governed by main_control_group_can_be_modified, not
        // by any of the change_control_rules.
        let c = config_with_all_owner_rules();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::MainControlGroup(Some(3)),
        );
        assert_eq!(result, AuthorizedActionTakers::NoOne);
    }

    #[test]
    fn controlling_main_group_change_uses_its_current_rule() {
        let mut c = config_with_all_owner_rules();
        c.set_main_control_group_can_be_modified(AuthorizedActionTakers::Group(5));

        let result = c.controlling_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::MainControlGroup(Some(3)),
        );

        assert_eq!(result, AuthorizedActionTakers::Group(5));
    }

    #[test]
    fn controlling_main_group_change_reports_configured_rule_verbatim() {
        for takers in [
            AuthorizedActionTakers::NoOne,
            AuthorizedActionTakers::ContractOwner,
            AuthorizedActionTakers::MainGroup,
            AuthorizedActionTakers::Group(0),
            AuthorizedActionTakers::Identity(Identifier::from([7u8; 32])),
        ] {
            let mut c = TokenConfigurationV0::default_most_restrictive();
            c.set_main_control_group_can_be_modified(takers);
            let result = c.controlling_action_takers_for_configuration_item(
                &TokenConfigurationChangeItem::MainControlGroup(Some(3)),
            );
            assert_eq!(result, takers);
        }
    }

    #[test]
    fn marketplace_trade_mode_returns_authorized_to_make_change() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let result = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::MarketplaceTradeMode(TokenTradeMode::NotTradeable),
        );
        // default rules are NoOne
        assert_eq!(result, AuthorizedActionTakers::NoOne);
    }

    #[test]
    fn perpetual_distribution_returns_authorized_to_make_change() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let r = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::PerpetualDistribution(None),
        );
        // default distribution rules authorized_to_make_change is NoOne
        assert_eq!(r, AuthorizedActionTakers::NoOne);
    }

    #[test]
    fn new_tokens_destination_identity_returns_authorized_to_make_change() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let r = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::NewTokensDestinationIdentity(None),
        );
        assert_eq!(r, AuthorizedActionTakers::NoOne);
    }

    #[test]
    fn minting_allow_choosing_destination_returns_authorized_to_make_change() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let r = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::MintingAllowChoosingDestination(true),
        );
        assert_eq!(r, AuthorizedActionTakers::NoOne);
    }

    #[test]
    fn unfreeze_and_admin_both_return_admin() {
        let c = config_with_all_owner_rules();
        let r1 = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::Unfreeze(AuthorizedActionTakers::NoOne),
        );
        let r2 = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::UnfreezeAdminGroup(AuthorizedActionTakers::NoOne),
        );
        let expected = AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]));
        assert_eq!(r1, expected);
        assert_eq!(r2, expected);
    }

    #[test]
    fn emergency_action_and_admin_both_return_admin() {
        let c = config_with_all_owner_rules();
        let r1 = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::EmergencyAction(AuthorizedActionTakers::NoOne),
        );
        let r2 = c.authorized_action_takers_for_configuration_item(
            &TokenConfigurationChangeItem::EmergencyActionAdminGroup(AuthorizedActionTakers::NoOne),
        );
        let expected = AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]));
        assert_eq!(r1, expected);
        assert_eq!(r2, expected);
    }
}
