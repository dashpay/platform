use crate::balances::credits::TokenAmount;
use crate::prelude::TimestampMillis;
use crate::serialization::json_safe_fields;
use bincode::Encode;
use platform_serialization::de::Decode;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[json_safe_fields]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenPreProgrammedDistributionV0 {
    #[serde(with = "crate::serialization::json::safe_integer_map::json_safe_u64_nested_identifier_u64_map")]
    pub distributions: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
}

/// Implementing `Display` for `TokenPreProgrammedDistributionV0`
impl fmt::Display for TokenPreProgrammedDistributionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "TokenPreProgrammedDistributionV0 {{")?;
        for (timestamp, distributions) in &self.distributions {
            writeln!(f, "  Timestamp: {}", timestamp)?;
            for (identity, amount) in distributions {
                writeln!(f, "    Identity: {}, Amount: {}", identity, amount)?;
            }
        }
        write!(f, "}}")
    }
}

