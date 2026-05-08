use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::{
    TokenDistributionRulesV0Getters, TokenDistributionRulesV0Setters,
};
use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::{
    TokenMarketplaceRulesV0Getters, TokenMarketplaceRulesV0Setters,
};

impl TokenConfigurationV0 {
    /// Applies a `TokenConfigurationChangeItem` to this token configuration.
    ///
    /// # Parameters
    /// - `change_item`: The change item to be applied.
    ///
    /// This method modifies the current `TokenConfigurationV0` instance in place.
    pub fn apply_token_configuration_item(&mut self, change_item: TokenConfigurationChangeItem) {
        match change_item {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => {
                // No changes are made
            }
            TokenConfigurationChangeItem::Conventions(conventions) => {
                self.conventions = conventions;
            }
            TokenConfigurationChangeItem::ConventionsControlGroup(control_group) => {
                self.conventions_change_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::ConventionsAdminGroup(admin_group) => {
                self.conventions_change_rules
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::MaxSupply(max_supply) => {
                self.max_supply = max_supply;
            }
            TokenConfigurationChangeItem::MaxSupplyControlGroup(control_group) => {
                self.max_supply_change_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(admin_group) => {
                self.max_supply_change_rules
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(identity) => {
                self.distribution_rules
                    .set_new_tokens_destination_identity(identity);
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                control_group,
            ) => {
                self.distribution_rules
                    .new_tokens_destination_identity_rules_mut()
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(admin_group) => {
                self.distribution_rules
                    .new_tokens_destination_identity_rules_mut()
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(allow) => {
                self.distribution_rules
                    .set_minting_allow_choosing_destination(allow);
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                control_group,
            ) => {
                self.distribution_rules
                    .minting_allow_choosing_destination_rules_mut()
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                admin_group,
            ) => {
                self.distribution_rules
                    .minting_allow_choosing_destination_rules_mut()
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::PerpetualDistribution(perpetual_distribution) => {
                self.distribution_rules
                    .set_perpetual_distribution(perpetual_distribution);
            }
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(control_group) => {
                self.distribution_rules
                    .perpetual_distribution_rules_mut()
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(admin_group) => {
                self.distribution_rules
                    .perpetual_distribution_rules_mut()
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::ManualMinting(control_group) => {
                self.manual_minting_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::ManualMintingAdminGroup(admin_group) => {
                self.manual_minting_rules
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::ManualBurning(control_group) => {
                self.manual_burning_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::ManualBurningAdminGroup(admin_group) => {
                self.manual_burning_rules
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::Freeze(control_group) => {
                self.freeze_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::FreezeAdminGroup(admin_group) => {
                self.freeze_rules.set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::Unfreeze(control_group) => {
                self.unfreeze_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::UnfreezeAdminGroup(admin_group) => {
                self.unfreeze_rules.set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::DestroyFrozenFunds(control_group) => {
                self.destroy_frozen_funds_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(admin_group) => {
                self.destroy_frozen_funds_rules
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::EmergencyAction(control_group) => {
                self.emergency_action_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(admin_group) => {
                self.emergency_action_rules
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::MainControlGroup(main_group) => {
                self.main_control_group = main_group;
            }
            TokenConfigurationChangeItem::MarketplaceTradeMode(trade_mode) => {
                self.marketplace_rules.set_trade_mode(trade_mode);
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(control_group) => {
                self.marketplace_rules
                    .trade_mode_change_rules_mut()
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(admin_group) => {
                self.marketplace_rules
                    .trade_mode_change_rules_mut()
                    .set_admin_action_takers(admin_group);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::accessors::v0::{
        TokenConfigurationV0Getters, TokenConfigurationV0Setters,
    };
    use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
    use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Getters;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use platform_value::Identifier;

    fn base() -> TokenConfigurationV0 {
        TokenConfigurationV0::default_most_restrictive()
    }

    // --- no-op ---

    #[test]
    fn no_change_leaves_config_untouched() {
        let mut c = base();
        let before = c.clone();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::TokenConfigurationNoChange);
        assert_eq!(c, before);
    }

    // --- conventions ---

    #[test]
    fn conventions_item_replaces_conventions() {
        let mut c = base();
        let new_conv = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 3,
        });
        c.apply_token_configuration_item(TokenConfigurationChangeItem::Conventions(
            new_conv.clone(),
        ));
        assert_eq!(c.conventions(), &new_conv);
    }

    #[test]
    fn conventions_control_group_sets_authorized_action_takers() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::ConventionsControlGroup(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.conventions_change_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn conventions_admin_group_sets_admin_action_takers() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::ConventionsAdminGroup(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.conventions_change_rules().admin_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    // --- max_supply ---

    #[test]
    fn max_supply_item_sets_value() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::MaxSupply(Some(500)));
        assert_eq!(c.max_supply(), Some(500));
    }

    #[test]
    fn max_supply_item_sets_none() {
        let mut c = base();
        c.set_max_supply(Some(999));
        c.apply_token_configuration_item(TokenConfigurationChangeItem::MaxSupply(None));
        assert_eq!(c.max_supply(), None);
    }

    #[test]
    fn max_supply_control_group_sets_authorized() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::MaxSupplyControlGroup(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.max_supply_change_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn max_supply_admin_group_sets_admin() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::MaxSupplyAdminGroup(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.max_supply_change_rules().admin_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    // --- distribution: new tokens destination identity ---

    #[test]
    fn new_tokens_destination_identity_sets_value() {
        let mut c = base();
        let id = Identifier::from([3u8; 32]);
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(id)),
        );
        assert_eq!(
            c.distribution_rules().new_tokens_destination_identity(),
            Some(&id)
        );
    }

    #[test]
    fn new_tokens_destination_identity_control_group_sets_authorized() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
        );
        assert_eq!(
            c.distribution_rules()
                .new_tokens_destination_identity_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn new_tokens_destination_identity_admin_group_sets_admin() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
        );
        assert_eq!(
            c.distribution_rules()
                .new_tokens_destination_identity_rules()
                .admin_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn minting_allow_choosing_destination_sets_value() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(false),
        );
        assert!(!c.distribution_rules().minting_allow_choosing_destination());
    }

    #[test]
    fn minting_allow_choosing_destination_control_group_sets_authorized() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
        );
        assert_eq!(
            c.distribution_rules()
                .minting_allow_choosing_destination_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn minting_allow_choosing_destination_admin_group_sets_admin() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
        );
        assert_eq!(
            c.distribution_rules()
                .minting_allow_choosing_destination_rules()
                .admin_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn perpetual_distribution_none_clears_it() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::PerpetualDistribution(None));
        assert!(c.distribution_rules().perpetual_distribution().is_none());
    }

    #[test]
    fn perpetual_distribution_control_group_sets_authorized() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
        );
        assert_eq!(
            c.distribution_rules()
                .perpetual_distribution_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn perpetual_distribution_admin_group_sets_admin() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
        );
        assert_eq!(
            c.distribution_rules()
                .perpetual_distribution_rules()
                .admin_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    // --- manual minting / burning ---

    #[test]
    fn manual_minting_sets_authorized() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::ManualMinting(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.manual_minting_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn manual_minting_admin_group_sets_admin() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::ManualMintingAdminGroup(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.manual_minting_rules().admin_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn manual_burning_sets_authorized() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::ManualBurning(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.manual_burning_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    #[test]
    fn manual_burning_admin_group_sets_admin() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::ManualBurningAdminGroup(
            AuthorizedActionTakers::ContractOwner,
        ));
        assert_eq!(
            c.manual_burning_rules().admin_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
    }

    // --- freeze / unfreeze / destroy ---

    #[test]
    fn freeze_and_admin_set_expected_fields() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::Freeze(
            AuthorizedActionTakers::ContractOwner,
        ));
        c.apply_token_configuration_item(TokenConfigurationChangeItem::FreezeAdminGroup(
            AuthorizedActionTakers::MainGroup,
        ));
        assert_eq!(
            c.freeze_rules().authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
        assert_eq!(
            c.freeze_rules().admin_action_takers(),
            &AuthorizedActionTakers::MainGroup
        );
    }

    #[test]
    fn unfreeze_and_admin_set_expected_fields() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::Unfreeze(
            AuthorizedActionTakers::ContractOwner,
        ));
        c.apply_token_configuration_item(TokenConfigurationChangeItem::UnfreezeAdminGroup(
            AuthorizedActionTakers::MainGroup,
        ));
        assert_eq!(
            c.unfreeze_rules().authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
        assert_eq!(
            c.unfreeze_rules().admin_action_takers(),
            &AuthorizedActionTakers::MainGroup
        );
    }

    #[test]
    fn destroy_frozen_funds_and_admin_set_expected_fields() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::DestroyFrozenFunds(
            AuthorizedActionTakers::ContractOwner,
        ));
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(
                AuthorizedActionTakers::MainGroup,
            ),
        );
        assert_eq!(
            c.destroy_frozen_funds_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
        assert_eq!(
            c.destroy_frozen_funds_rules().admin_action_takers(),
            &AuthorizedActionTakers::MainGroup
        );
    }

    // --- emergency action ---

    #[test]
    fn emergency_action_and_admin_set_expected_fields() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::EmergencyAction(
            AuthorizedActionTakers::ContractOwner,
        ));
        c.apply_token_configuration_item(TokenConfigurationChangeItem::EmergencyActionAdminGroup(
            AuthorizedActionTakers::MainGroup,
        ));
        assert_eq!(
            c.emergency_action_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
        assert_eq!(
            c.emergency_action_rules().admin_action_takers(),
            &AuthorizedActionTakers::MainGroup
        );
    }

    // --- main control group ---

    #[test]
    fn main_control_group_sets_value_then_clears() {
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::MainControlGroup(Some(5)));
        assert_eq!(c.main_control_group(), Some(5));
        c.apply_token_configuration_item(TokenConfigurationChangeItem::MainControlGroup(None));
        assert_eq!(c.main_control_group(), None);
    }

    // --- marketplace ---

    #[test]
    fn marketplace_trade_mode_sets_trade_mode() {
        use crate::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
        let mut c = base();
        c.apply_token_configuration_item(TokenConfigurationChangeItem::MarketplaceTradeMode(
            TokenTradeMode::NotTradeable,
        ));
        // Only one variant currently, but this exercises the dispatch arm.
        assert_eq!(
            c.distribution_rules_mut()
                .change_direct_purchase_pricing_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::NoOne
        );
        // Primary assertion: confirm trade mode has been set
        // (we route via the marketplace rule accessor)
        use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Getters as _;
        let _ = c.distribution_rules(); // avoid unused warning if helpers change
                                        // Read trade mode indirectly via reflection: marketplace_rules is pub on V0
        match c.marketplace_rules {
            crate::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules::V0(ref mp) => {
                assert_eq!(mp.trade_mode(), &TokenTradeMode::NotTradeable);
            }
        }
    }

    #[test]
    fn marketplace_trade_mode_control_and_admin_set_correctly() {
        let mut c = base();
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
        );
        c.apply_token_configuration_item(
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(
                AuthorizedActionTakers::MainGroup,
            ),
        );
        assert_eq!(
            c.marketplace_rules
                .trade_mode_change_rules()
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::ContractOwner
        );
        assert_eq!(
            c.marketplace_rules
                .trade_mode_change_rules()
                .admin_action_takers(),
            &AuthorizedActionTakers::MainGroup
        );
    }
}
