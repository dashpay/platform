use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;

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
        }
    }
}
