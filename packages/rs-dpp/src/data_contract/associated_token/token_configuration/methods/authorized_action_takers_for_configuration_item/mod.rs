use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;

mod v0;

impl TokenConfiguration {
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
        }
    }
}
