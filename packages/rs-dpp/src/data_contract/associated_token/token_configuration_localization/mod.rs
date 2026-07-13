use crate::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::Encode;
use derive_more::From;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;
pub mod v0;

/// Versioned wrapper for token name localization data.
///
/// `TokenConfigurationLocalization` allows extensibility for future schema upgrades
/// while preserving backward compatibility. Each variant represents a specific format
/// version for localization information.
///
/// This structure is used to map language codes to localized token names in a flexible,
/// forward-compatible manner.
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, From)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[serde(tag = "$formatVersion")]
pub enum TokenConfigurationLocalization {
    /// Version 0 of the token localization schema.
    ///
    /// Defines basic capitalization preference, singular form, and plural form
    /// for displaying token names.
    #[serde(rename = "0")]
    V0(TokenConfigurationLocalizationV0),
}

impl fmt::Display for TokenConfigurationLocalization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfigurationLocalization::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;

    fn fixture() -> TokenConfigurationLocalization {
        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
            should_capitalize: true,
            singular_form: "Token".to_string(),
            plural_form: "Tokens".to_string(),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "shouldCapitalize": true,
                "singularForm": "Token",
                "pluralForm": "Tokens",
            })
        );
        let recovered = TokenConfigurationLocalization::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "shouldCapitalize": true,
                "singularForm": "Token",
                "pluralForm": "Tokens",
            })
        );
        let recovered = TokenConfigurationLocalization::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
