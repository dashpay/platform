use crate::balances::credits::TokenAmount;
use crate::prelude::TimestampMillis;
use bincode::Encode;
use platform_serialization::de::Decode;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenPreProgrammedDistributionV0 {
    #[serde(deserialize_with = "deserialize_ts_to_id_amount_map")]
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

use serde::de::Deserializer;

// Custom deserializer for `distributions` that tolerates two JSON shapes:
// - a map keyed by timestamp millis as numbers (u64)
// - a map keyed by timestamp millis as strings (e.g. "1735689600000")
// It normalizes both into `BTreeMap<u64, V>` where V is the inner value map
// (`BTreeMap<Identifier, TokenAmount>` here). If a string key cannot be parsed
// as `u64`, deserialization fails with a serde error. Using `BTreeMap` keeps
// keys ordered by timestamp.
fn deserialize_ts_to_id_amount_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>, D::Error>
where
    D: Deserializer<'de>,
{
    // Untagged enum attempts the variants in order: first try a map with u64
    // keys; if that doesn't match, try a map with string keys.
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum U64OrStrTs<V> {
        // JSON: { 1735689600000: { <id>: <amount>, ... }, ... }
        U64(BTreeMap<TimestampMillis, V>),
        // JSON: { "1735689600000": { <id>: <amount>, ... }, ... }
        Str(BTreeMap<String, V>),
    }

    let helper: U64OrStrTs<BTreeMap<Identifier, TokenAmount>> =
        U64OrStrTs::deserialize(deserializer)?;
    match helper {
        // Already has numeric timestamp keys; return as-is
        U64OrStrTs::U64(m) => Ok(m),
        // Convert string timestamp keys into u64, preserving values unchanged
        U64OrStrTs::Str(sm) => sm
            .into_iter()
            .map(|(k, v)| {
                k.parse::<u64>()
                    .map_err(serde::de::Error::custom)
                    .map(|ts| (ts, v))
            })
            .collect(),
    }
}
