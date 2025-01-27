use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
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
            TokenConfigurationChangeItem::Conventions(_) => self
                .conventions_change_rules
                .authorized_to_make_change_action_takers()
                .clone(),
            TokenConfigurationChangeItem::ConventionsControlGroup(_) => {
                self.conventions_change_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::ConventionsAdminGroup(_) => {
                self.conventions_change_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::MaxSupply(_) => self
                .max_supply_change_rules
                .authorized_to_make_change_action_takers()
                .clone(),
            TokenConfigurationChangeItem::MaxSupplyControlGroup(_) => {
                self.max_supply_change_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(_) => {
                self.max_supply_change_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::PerpetualDistribution(_) => self
                .distribution_rules
                .perpetual_distribution_rules()
                .authorized_to_make_change_action_takers()
                .clone(),
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(_) => self
                .distribution_rules
                .perpetual_distribution_rules()
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(_) => self
                .distribution_rules
                .perpetual_distribution_rules()
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(_) => self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .authorized_to_make_change_action_takers()
                .clone(),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(_) => self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(_) => self
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(_) => self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .authorized_to_make_change_action_takers()
                .clone(),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(_) => self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(_) => self
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::ManualMinting(_) => {
                self.manual_minting_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::ManualMintingAdminGroup(_) => {
                self.manual_minting_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::ManualBurning(_) => {
                self.manual_burning_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::ManualBurningAdminGroup(_) => {
                self.manual_burning_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::Freeze(_) => {
                self.freeze_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::FreezeAdminGroup(_) => {
                self.freeze_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::Unfreeze(_) => {
                self.unfreeze_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::UnfreezeAdminGroup(_) => {
                self.unfreeze_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::DestroyFrozenFunds(_) => self
                .destroy_frozen_funds_rules
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(_) => self
                .destroy_frozen_funds_rules
                .admin_action_takers()
                .clone(),
            TokenConfigurationChangeItem::EmergencyAction(_) => {
                self.emergency_action_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(_) => {
                self.emergency_action_rules.admin_action_takers().clone()
            }
            TokenConfigurationChangeItem::MainControlGroup(_) => AuthorizedActionTakers::NoOne,
        }
    }
}
