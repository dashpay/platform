use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;
pub mod methods;
pub mod v0;

/// Versioned wrapper for token display conventions.
///
/// `TokenConfigurationConvention` provides a flexible, forward-compatible structure
/// for representing human-readable metadata about a token, such as localized names
/// and decimal formatting standards.
///
/// This enum enables evolution of the convention schema over time without breaking
/// compatibility with older tokens. Each variant defines a specific format version.
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, From)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[serde(tag = "$formatVersion")]
pub enum TokenConfigurationConvention {
    /// Version 0 of the token convention schema.
    ///
    /// Defines localized names (by ISO 639 language codes) and the number of decimal places
    /// used for displaying token amounts.
    #[serde(rename = "0")]
    V0(TokenConfigurationConventionV0),
}

impl fmt::Display for TokenConfigurationConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfigurationConvention::V0(v0) => {
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
    use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
    use crate::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
    use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
    use std::collections::BTreeMap;

    fn fixture() -> TokenConfigurationConvention {
        let mut localizations = BTreeMap::new();
        localizations.insert(
            "en".to_string(),
            TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                should_capitalize: true,
                singular_form: "Token".to_string(),
                plural_form: "Tokens".to_string(),
            }),
        );
        TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations,
            decimals: 8,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `decimals` is `u8`; JSON erases the size — value-path locks `8u8` below.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "localizations": {
                    "en": {
                        "$formatVersion": "0",
                        "shouldCapitalize": true,
                        "singularForm": "Token",
                        "pluralForm": "Tokens",
                    }
                },
                "decimals": 8,
            })
        );
        let recovered = TokenConfigurationConvention::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `decimals` is u8 → `Value::U8`.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "localizations": {
                    "en": {
                        "$formatVersion": "0",
                        "shouldCapitalize": true,
                        "singularForm": "Token",
                        "pluralForm": "Tokens",
                    }
                },
                "decimals": 8u8,
            })
        );
        let recovered = TokenConfigurationConvention::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
