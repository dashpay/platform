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

    /// Non-default fixture with two distinct timestamps and two recipients per
    /// timestamp so per-property assertions can catch silent map-flatten /
    /// key-swap on round-trip.
    fn fixture() -> TokenPreProgrammedDistribution {
        let mut early = BTreeMap::new();
        early.insert(Identifier::new([0xab; 32]), 1000u64);
        early.insert(Identifier::new([0xcd; 32]), 2000u64);

        let mut late = BTreeMap::new();
        late.insert(Identifier::new([0xef; 32]), 3000u64);

        let mut distributions = BTreeMap::new();
        distributions.insert(1_700_000_000_000u64, early);
        distributions.insert(1_800_000_000_000u64, late);
        TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 { distributions })
    }

    fn assert_v0_fields(d: &TokenPreProgrammedDistribution) {
        let TokenPreProgrammedDistribution::V0(rec) = d;
        assert_eq!(rec.distributions.len(), 2, "distributions.len");
        let early = rec
            .distributions
            .get(&1_700_000_000_000u64)
            .expect("early ts present");
        assert_eq!(early.len(), 2, "early.recipients.len");
        assert_eq!(
            early.get(&Identifier::new([0xab; 32])).copied(),
            Some(1000u64),
            "early[0xab..]"
        );
        assert_eq!(
            early.get(&Identifier::new([0xcd; 32])).copied(),
            Some(2000u64),
            "early[0xcd..]"
        );
        let late = rec
            .distributions
            .get(&1_800_000_000_000u64)
            .expect("late ts present");
        assert_eq!(late.len(), 1, "late.recipients.len");
        assert_eq!(
            late.get(&Identifier::new([0xef; 32])).copied(),
            Some(3000u64),
            "late[0xef..]"
        );
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = TokenPreProgrammedDistribution::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenPreProgrammedDistribution::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
