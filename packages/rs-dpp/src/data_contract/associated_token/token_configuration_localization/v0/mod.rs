mod accessors;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Defines the localized naming format for a token in a specific language.
///
/// `TokenConfigurationLocalizationV0` enables tokens to present user-friendly names
/// across different locales. This information is not used for validation or consensus
/// but enhances UX by allowing consistent display in multilingual interfaces.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationLocalizationV0 {
    /// Indicates whether the token name should be capitalized when displayed.
    ///
    /// This is a stylistic hint for clients (e.g., "Dash" vs. "dash") and is typically
    /// applied to both singular and plural forms unless overridden.
    pub should_capitalize: bool,

    /// The singular form of the token name in the target language.
    ///
    /// Example: "Dash", "Dollar", or "Token".
    pub singular_form: String,

    /// The plural form of the token name in the target language.
    ///
    /// Example: "Dash", "Dollars", or "Tokens".
    pub plural_form: String,
}

impl fmt::Display for TokenConfigurationLocalizationV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Capitalized: {}, Singular: '{}', Plural: '{}'",
            self.should_capitalize, self.singular_form, self.plural_form
        )
    }
}
