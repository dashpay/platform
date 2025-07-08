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
