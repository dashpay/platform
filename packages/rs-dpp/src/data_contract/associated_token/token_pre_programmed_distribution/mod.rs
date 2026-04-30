use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;

pub mod v0;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum TokenPreProgrammedDistribution {
    #[serde(rename = "0")]
    V0(TokenPreProgrammedDistributionV0),
}

impl fmt::Display for TokenPreProgrammedDistribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenPreProgrammedDistribution::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    fn fixture() -> TokenPreProgrammedDistribution {
        let mut inner = BTreeMap::new();
        inner.insert(Identifier::new([0xab; 32]), 1000u64);
        let mut distributions = BTreeMap::new();
        distributions.insert(1_700_000_000_000u64, inner);
        TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 { distributions })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = TokenPreProgrammedDistribution::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenPreProgrammedDistribution::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
