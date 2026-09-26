use crate::data_contract::associated_token::token_configuration::v1::TokenConfigurationV1;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_configuration_item::{
    token_configuration_v0_change_items, TokenConfigurationChangeItem,
};
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;

mod v0;

impl TokenConfiguration {
    /// Returns the current authority that controls a configuration item.
    pub fn controlling_action_takers_for_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
    ) -> AuthorizedActionTakers {
        match self {
            TokenConfiguration::V0(v0) => {
                v0.controlling_action_takers_for_configuration_item(change_item)
            }
            TokenConfiguration::V1(v1) => {
                v1.controlling_action_takers_for_configuration_item(change_item)
            }
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
        match self {
            TokenConfiguration::V0(v0) => {
                v0.authorized_action_takers_for_configuration_item(change_item)
            }
            TokenConfiguration::V1(v1) => {
                v1.authorized_action_takers_for_configuration_item(change_item)
            }
        }
    }
}

impl TokenConfigurationV1 {
    /// Returns the current authority that controls a configuration item: the shielded pool's
    /// threshold items are controlled like any other rule-governed item, every other item as
    /// the nested V0 configuration says.
    pub fn controlling_action_takers_for_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
    ) -> AuthorizedActionTakers {
        match change_item {
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(_)
            | TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(_)
            | TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(_) => {
                self.authorized_action_takers_for_configuration_item(change_item)
            }
            token_configuration_v0_change_items!() => self
                .base
                .controlling_action_takers_for_configuration_item(change_item),
        }
    }

    /// Returns the authorized action takers for a specific `TokenConfigurationChangeItem`: the
    /// shielded pool's threshold items from `minimum_pool_notes_for_outgoing_change_rules`,
    /// every other item from the nested V0 configuration.
    pub fn authorized_action_takers_for_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
    ) -> AuthorizedActionTakers {
        let rules = &self.minimum_pool_notes_for_outgoing_change_rules;
        match change_item {
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(_) => {
                *rules.authorized_to_make_change_action_takers()
            }
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(_)
            | TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(_) => {
                *rules.admin_action_takers()
            }
            token_configuration_v0_change_items!() => self
                .base
                .authorized_action_takers_for_configuration_item(change_item),
        }
    }
}
