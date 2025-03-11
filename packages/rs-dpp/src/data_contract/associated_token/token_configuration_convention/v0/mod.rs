use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationConventionV0 {
    #[serde(default)]
    pub localizations: BTreeMap<String, TokenConfigurationLocalization>,
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
