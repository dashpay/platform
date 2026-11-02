use crate::balances::credits::TokenAmount;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{DecodeUntrusted, Encode};
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Version 0 of the once-per-identity distribution: a single fixed amount.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub struct TokenOncePerIdentityDistributionV0 {
    /// The amount minted to an identity on its single claim. Must be at least 1 and at most
    /// `i64::MAX`, the largest amount a token balance can hold.
    pub amount: TokenAmount,
}

impl fmt::Display for TokenOncePerIdentityDistributionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenOncePerIdentityDistributionV0 {{ amount: {} }}",
            self.amount
        )
    }
}
