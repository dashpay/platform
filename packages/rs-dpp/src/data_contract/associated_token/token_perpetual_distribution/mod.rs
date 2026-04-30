use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
use crate::errors::ProtocolError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod distribution_function;
pub mod distribution_recipient;
pub mod methods;
pub mod reward_distribution_moment;
pub mod reward_distribution_type;
pub mod v0;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    From,
)]
#[serde(tag = "$formatVersion")]
#[platform_serialize(unversioned)]
pub enum TokenPerpetualDistribution {
    #[serde(rename = "0")]
    V0(TokenPerpetualDistributionV0),
}

impl fmt::Display for TokenPerpetualDistribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenPerpetualDistribution::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;

    fn fixture() -> TokenPerpetualDistribution {
        TokenPerpetualDistribution::V0(TokenPerpetualDistributionV0 {
            distribution_type: RewardDistributionType::BlockBasedDistribution {
                interval: 1000,
                function: DistributionFunction::FixedAmount { amount: 100 },
            },
            distribution_recipient: TokenDistributionRecipient::ContractOwner,
        })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = TokenPerpetualDistribution::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenPerpetualDistribution::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
