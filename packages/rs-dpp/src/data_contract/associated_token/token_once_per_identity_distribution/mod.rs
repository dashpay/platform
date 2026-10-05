use crate::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;
pub mod v0;

/// A distribution that every identity may claim exactly once.
///
/// Any identity can submit a claim of type `OncePerIdentity` against the token. The first claim
/// mints the configured amount to the claimant and records the claim under the token's
/// once-per-identity subtree in Drive; every later claim by the same identity is rejected. The
/// total paid out is bounded only by the token's max supply, so a bounded airdrop is configured by
/// setting `maxSupply`.
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From, DecodeUntrusted,
)]
#[serde(tag = "$formatVersion")]
pub enum TokenOncePerIdentityDistribution {
    #[serde(rename = "0")]
    V0(TokenOncePerIdentityDistributionV0),
}

impl fmt::Display for TokenOncePerIdentityDistribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenOncePerIdentityDistribution::V0(v0) => {
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
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> TokenOncePerIdentityDistribution {
        TokenOncePerIdentityDistribution::V0(TokenOncePerIdentityDistributionV0 { amount: 2500 })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "amount": 2500,
            })
        );
        let recovered = TokenOncePerIdentityDistribution::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_amount_above_max_safe_integer_is_a_string() {
        let original = TokenOncePerIdentityDistribution::V0(TokenOncePerIdentityDistributionV0 {
            amount: u64::MAX,
        });
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "amount": u64::MAX.to_string(),
            })
        );
        let recovered = TokenOncePerIdentityDistribution::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "amount": 2500u64,
            })
        );
        let recovered = TokenOncePerIdentityDistribution::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
