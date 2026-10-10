use crate::data_contract::associated_token::token_configuration::v1::TokenConfigurationV1;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_configuration_item::{
    token_configuration_v0_change_items, TokenConfigurationChangeItem,
};

mod v0;

impl TokenConfiguration {
    /// Applies a `TokenConfigurationChangeItem` to this token configuration.
    ///
    /// # Parameters
    /// - `change_item`: The change item to be applied.
    ///
    /// This method modifies the current `TokenConfigurationV0` instance in place.
    pub fn apply_token_configuration_item(&mut self, change_item: TokenConfigurationChangeItem) {
        match self {
            TokenConfiguration::V0(v0) => v0.apply_token_configuration_item(change_item),
            TokenConfiguration::V1(v1) => v1.apply_token_configuration_item(change_item),
        }
    }
}

impl TokenConfigurationV1 {
    /// Applies a `TokenConfigurationChangeItem`: the shielded pool's threshold items to this
    /// configuration, every other item to the nested V0 configuration.
    pub fn apply_token_configuration_item(&mut self, change_item: TokenConfigurationChangeItem) {
        match change_item {
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(minimum_pool_notes) => {
                self.minimum_pool_notes_for_outgoing = Some(minimum_pool_notes);
            }
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(
                control_group,
            ) => {
                self.minimum_pool_notes_for_outgoing_change_rules
                    .set_authorized_to_make_change_action_takers(control_group);
            }
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(admin_group) => {
                self.minimum_pool_notes_for_outgoing_change_rules
                    .set_admin_action_takers(admin_group);
            }
            token_configuration_v0_change_items!() => {
                self.base.apply_token_configuration_item(change_item)
            }
        }
    }
}
