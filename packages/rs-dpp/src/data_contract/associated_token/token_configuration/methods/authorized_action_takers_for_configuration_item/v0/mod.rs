use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Getters;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
impl TokenConfigurationV0 {
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
