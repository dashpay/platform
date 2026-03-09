use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
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
#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From, ValueConvertible,
)]
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
