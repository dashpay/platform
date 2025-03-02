use crate::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationLocalizationsV0 {
    pub should_capitalize: bool,
    pub singular_form: String,
    pub plural_form: String,
}

impl fmt::Display for TokenConfigurationLocalizationsV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Capitalized: {}, Singular: '{}', Plural: '{}'",
            self.should_capitalize, self.singular_form, self.plural_form
        )
    }
}

pub const ENGLISH_ISO_639: &str = "en";

#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationConventionV0 {
    /// Localizations for the token name.
    /// The key must be a ISO 639 2-chars language code
    #[serde(default)]
    pub localizations: BTreeMap<String, TokenConfigurationLocalizationsV0>,
    #[serde(default = "default_decimals")]
    pub decimals: u16,
}

// Default function for `decimals`
fn default_decimals() -> u16 {
    8 // Default value for decimals
}

impl fmt::Display for TokenConfigurationConventionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let localizations: Vec<String> = self
            .localizations
            .iter()
            .map(|(key, value)| format!("{}: {}", key, value))
            .collect();

        write!(
            f,
            "Decimals: {}, Localizations: [{}]",
            self.decimals,
            localizations.join(", ")
        )
    }
}

impl TokenConfigurationConventionV0Getters for TokenConfigurationConventionV0 {
    fn singular_form_by_language_code_or_default(&self, language_code: &str) -> &str {
        self.localizations
            .get(language_code)
            .map(|localization| &localization.singular_form)
            .unwrap_or_else(|| &self.localizations[ENGLISH_ISO_639].singular_form)
    }

    fn plural_form_by_language_code_or_default(&self, language_code: &str) -> &str {
        self.localizations
            .get(language_code)
            .map(|localization| &localization.plural_form)
            .unwrap_or_else(|| &self.localizations[ENGLISH_ISO_639].plural_form)
    }
}
