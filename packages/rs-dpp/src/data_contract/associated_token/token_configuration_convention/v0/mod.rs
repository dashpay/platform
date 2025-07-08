use crate::data_contract::associated_token::token_configuration_convention::accessors::v0::{
    TokenConfigurationConventionV0Getters, TokenConfigurationConventionV0Setters,
};
use crate::data_contract::associated_token::token_configuration_localization::accessors::v0::TokenConfigurationLocalizationV0Getters;
use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub const ENGLISH_ISO_639: &str = "en";

/// Defines display conventions for a token, including name localization and decimal precision.
///
/// `TokenConfigurationConventionV0` provides human-readable metadata to guide client applications
/// in rendering token names and formatting token values. This structure is purely informative
/// and does not affect consensus-critical logic or supply calculations.
#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationConventionV0 {
    /// A mapping of ISO 639-1 language codes (2-letter lowercase strings) to localized
    /// token names and metadata.
    ///
    /// These localizations enable wallets and dApps to display token information in the
    /// user's preferred language. At least one localization (e.g., English) is strongly recommended.
    #[serde(default)]
    pub localizations: BTreeMap<String, TokenConfigurationLocalization>,

    /// The number of decimal places used to represent the token.
    ///
    /// For example, a value of `8` means that one full token is represented as `10^8` base units
    /// (similar to Bitcoin's satoshis or Dash's duffs).
    ///
    /// This value is used by clients to determine formatting and user interface display.
    #[serde(default = "default_decimals")]
    pub decimals: u8,
}

// Default function for `decimals`
fn default_decimals() -> u8 {
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
            .map(|localization| localization.singular_form())
            .unwrap_or_else(|| self.localizations[ENGLISH_ISO_639].singular_form())
    }

    fn plural_form_by_language_code_or_default(&self, language_code: &str) -> &str {
        self.localizations
            .get(language_code)
            .map(|localization| localization.plural_form())
            .unwrap_or_else(|| self.localizations[ENGLISH_ISO_639].plural_form())
    }

    fn localizations(&self) -> &BTreeMap<String, TokenConfigurationLocalization> {
        &self.localizations
    }

    fn localizations_mut(&mut self) -> &mut BTreeMap<String, TokenConfigurationLocalization> {
        &mut self.localizations
    }

    fn decimals(&self) -> u8 {
        self.decimals
    }
}

impl TokenConfigurationConventionV0Setters for TokenConfigurationConventionV0 {
    fn set_localizations(
        &mut self,
        localizations: BTreeMap<String, TokenConfigurationLocalization>,
    ) {
        self.localizations = localizations;
    }

    fn set_decimals(&mut self, decimals: u8) {
        self.decimals = decimals;
    }
}
