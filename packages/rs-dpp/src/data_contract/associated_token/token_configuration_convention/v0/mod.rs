use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
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

#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd, Default,
)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct TokenConfigurationConventionV0 {
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
