use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
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
                self.new_tokens_destination_identity = identity;
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                control_group,
            ) => {
                self.new_tokens_destination_identity_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(admin_group) => {
                self.new_tokens_destination_identity_rules
                    .set_admin_action_takers(admin_group);
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(allow) => {
                self.minting_allow_choosing_destination = allow;
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                control_group,
            ) => {
                self.minting_allow_choosing_destination_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                admin_group,
            ) => {
                self.minting_allow_choosing_destination_rules
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
        }
    }
}
