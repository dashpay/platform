use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use std::collections::BTreeMap;

/// Accessor trait for getters of `TokenConfigurationConventionV0`
pub trait TokenConfigurationConventionV0Getters {
    /// Returns the localized token name in singular form
    fn singular_form_by_language_code_or_default(&self, language_code: &str) -> &str;
    /// Returns the localized token name in plural form
    fn plural_form_by_language_code_or_default(&self, language_code: &str) -> &str;
    /// Returns a reference to the localizations.
    fn localizations(&self) -> &BTreeMap<String, TokenConfigurationLocalization>;

    /// Returns a mutable reference to the localizations.
    fn localizations_mut(&mut self) -> &mut BTreeMap<String, TokenConfigurationLocalization>;

    /// Returns the decimals value.
    fn decimals(&self) -> u8;
}

/// Accessor trait for setters of `TokenConfigurationConventionV0`
pub trait TokenConfigurationConventionV0Setters {
    /// Sets the localizations.
    fn set_localizations(
        &mut self,
        localizations: BTreeMap<String, TokenConfigurationLocalization>,
    );

    /// Sets the decimals value.
    fn set_decimals(&mut self, decimals: u8);
}
