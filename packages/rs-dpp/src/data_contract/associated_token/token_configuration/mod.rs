use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

pub mod accessors;
mod methods;
pub mod v0;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum TokenConfiguration {
    #[serde(rename = "0")]
    V0(TokenConfigurationV0),
}
impl TokenConfiguration {
    pub fn as_cow_v0(&self) -> Cow<'_, TokenConfigurationV0> {
        match self {
            TokenConfiguration::V0(v0) => Cow::Borrowed(v0),
        }
    }
}

impl fmt::Display for TokenConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfiguration::V0(v0) => write!(f, "{}", v0),
        }
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn token_configuration_large_supply_json_round_trip() {
        let mut config = TokenConfigurationV0::default_most_restrictive();
        config.base_supply = u64::MAX;
        let config = TokenConfiguration::V0(config);

        let json = config.to_json().expect("to_json should succeed");

        // u64::MAX > JS MAX_SAFE_INTEGER, so it should be serialized as a string
        assert!(
            json["baseSupply"].is_string(),
            "baseSupply should be a string for large values, got: {:?}",
            json["baseSupply"]
        );
        assert_eq!(json["baseSupply"].as_str().unwrap(), u64::MAX.to_string());

        let restored = TokenConfiguration::from_json(json).expect("from_json should succeed");
        assert_eq!(config, restored);
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
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;

    /// `default_most_restrictive` already populates ~25 inner fields with
    /// non-default values (decimals=8, base_supply=100_000, etc.) — exactly
    /// what we want for the round-trip structural check below.
    fn fixture() -> TokenConfiguration {
        TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive())
    }

    /// Tier 3: TokenConfiguration embeds ~25 fields, several of which are
    /// themselves versioned enums (TokenConfigurationConvention,
    /// ChangeControlRules x7, TokenKeepsHistoryRules, TokenDistributionRules,
    /// TokenMarketplaceRules). An inline wire-shape literal would be 200+
    /// lines and would re-test the nested types' own assertions. Instead we
    /// assert only the envelope (top-level keys + `$formatVersion`) and trust
    /// the nested types' tests for inner shape correctness.
    #[test]
    fn json_round_trip_with_envelope_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Envelope check: format version + top-level keys present.
        assert_eq!(
            json.get("$formatVersion").and_then(|v| v.as_str()),
            Some("0")
        );
        for key in [
            "conventions",
            "conventionsChangeRules",
            "baseSupply",
            "maxSupply",
            "keepsHistory",
            "startAsPaused",
            "allowTransferToFrozenBalance",
            "maxSupplyChangeRules",
            "distributionRules",
            "marketplaceRules",
            "manualMintingRules",
            "manualBurningRules",
            "freezeRules",
            "unfreezeRules",
            "destroyFrozenFundsRules",
            "emergencyActionRules",
            "mainControlGroup",
            "mainControlGroupCanBeModified",
            "description",
        ] {
            assert!(
                json.get(key).is_some(),
                "expected top-level key {:?} in JSON envelope",
                key
            );
        }
        let recovered = TokenConfiguration::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_envelope_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Same envelope-only check on the platform_value side.
        let map = value.as_map().expect("value is a Map");
        let has_key = |k: &str| {
            map.iter()
                .any(|(key, _)| matches!(key, platform_value::Value::Text(t) if t == k))
        };
        assert!(has_key("$formatVersion"));
        for key in [
            "conventions",
            "conventionsChangeRules",
            "baseSupply",
            "maxSupply",
            "keepsHistory",
            "startAsPaused",
            "allowTransferToFrozenBalance",
            "maxSupplyChangeRules",
            "distributionRules",
            "marketplaceRules",
            "manualMintingRules",
            "manualBurningRules",
            "freezeRules",
            "unfreezeRules",
            "destroyFrozenFundsRules",
            "emergencyActionRules",
            "mainControlGroup",
            "mainControlGroupCanBeModified",
            "description",
        ] {
            assert!(
                has_key(key),
                "expected top-level key {:?} in Value envelope",
                key
            );
        }
        let recovered = TokenConfiguration::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
