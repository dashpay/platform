use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use std::collections::BTreeMap;

/// Accessor trait for getters of `TokenConfigurationConventionV0`
pub trait TokenConfigurationConventionV0Getters {
    /// Returns a reference to the localizations.
    fn localizations(&self) -> &BTreeMap<String, TokenConfigurationLocalization>;

    /// Returns a mutable reference to the localizations.
    fn localizations_mut(&mut self) -> &mut BTreeMap<String, TokenConfigurationLocalization>;

    /// Returns the decimals value.
    fn decimals(&self) -> u16;
}

/// Accessor trait for setters of `TokenConfigurationConventionV0`
pub trait TokenConfigurationConventionV0Setters {
    /// Sets the localizations.
    fn set_localizations(
        &mut self,
        localizations: BTreeMap<String, TokenConfigurationLocalization>,
    );

    /// Sets the decimals value.
    fn set_decimals(&mut self, decimals: u16);
}
