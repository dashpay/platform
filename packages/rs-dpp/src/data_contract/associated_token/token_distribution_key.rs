use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::{TokenDistributionRecipient, TokenDistributionResolvedRecipient};
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::prelude::TimestampMillis;

/// Represents the type of token distribution.
///
/// - `PreProgrammed`: A scheduled distribution with predefined rules.
/// - `Perpetual`: A continuous or recurring distribution.
#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Default,
)]
pub enum TokenDistributionType {
    /// A pre-programmed distribution scheduled for a specific time.
    #[default]
    PreProgrammed = 0,

    /// A perpetual distribution that occurs at regular intervals.
    Perpetual = 1,
}

/// Represents a token distribution with a resolved recipient.
///
/// - `PreProgrammed(Identifier)`: A predefined recipient for a scheduled distribution.
/// - `Perpetual(TokenDistributionResolvedRecipient)`: A resolved recipient for an ongoing distribution.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(
    into = "TokenDistributionTypeWithResolvedRecipientRepr",
    from = "TokenDistributionTypeWithResolvedRecipientRepr"
)]
pub enum TokenDistributionTypeWithResolvedRecipient {
    /// A scheduled distribution with a known recipient.
    PreProgrammed(Identifier),

    /// A perpetual distribution with a resolved recipient.
    Perpetual(TokenDistributionResolvedRecipient),
}

// Internal-`$type` serde shape with a uniform `value` payload (single-payload
// variants). Bincode `Encode`/`Decode` on the outer enum are untouched.
#[derive(Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "camelCase")]
enum TokenDistributionTypeWithResolvedRecipientRepr {
    PreProgrammed {
        value: Identifier,
    },
    Perpetual {
        value: TokenDistributionResolvedRecipient,
    },
}

impl From<TokenDistributionTypeWithResolvedRecipient>
    for TokenDistributionTypeWithResolvedRecipientRepr
{
    fn from(m: TokenDistributionTypeWithResolvedRecipient) -> Self {
        match m {
            TokenDistributionTypeWithResolvedRecipient::PreProgrammed(value) => {
                Self::PreProgrammed { value }
            }
            TokenDistributionTypeWithResolvedRecipient::Perpetual(value) => {
                Self::Perpetual { value }
            }
        }
    }
}

impl From<TokenDistributionTypeWithResolvedRecipientRepr>
    for TokenDistributionTypeWithResolvedRecipient
{
    fn from(r: TokenDistributionTypeWithResolvedRecipientRepr) -> Self {
        match r {
            TokenDistributionTypeWithResolvedRecipientRepr::PreProgrammed { value } => {
                Self::PreProgrammed(value)
            }
            TokenDistributionTypeWithResolvedRecipientRepr::Perpetual { value } => {
                Self::Perpetual(value)
            }
        }
    }
}

/// Contains information about a specific token distribution instance.
///
/// - `PreProgrammed(TimestampMillis, Identifier)`: A scheduled distribution with a timestamp and recipient.
/// - `Perpetual(RewardDistributionMoment, RewardDistributionMoment, TokenDistributionResolvedRecipient)`:
///   A perpetual distribution with previous and next distribution moments, along with the resolved recipient.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(into = "TokenDistributionInfoRepr", from = "TokenDistributionInfoRepr")]
pub enum TokenDistributionInfo {
    /// A pre-programmed token distribution set for a specific time.
    /// Contains the scheduled timestamp and the recipient’s identifier.
    PreProgrammed(TimestampMillis, Identifier),

    /// A perpetual token distribution with moment for distribution.
    /// The moment is the beginning of the perpetual distribution cycle
    /// Includes the last and next distribution times and the resolved recipient.
    Perpetual(RewardDistributionMoment, TokenDistributionResolvedRecipient),
}

// Internal-`$type` serde shape with named fields (multi-field variants).
// `TimestampMillis` (u64) carries `json_safe_u64` on the Repr field — JS-safe
// (string above MAX_SAFE_INTEGER in HR JSON), Content-safe (never u128).
// `RewardDistributionMoment` is itself internally tagged; bincode untouched.
#[derive(Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "camelCase")]
enum TokenDistributionInfoRepr {
    PreProgrammed {
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json_safe_u64")
        )]
        timestamp: TimestampMillis,
        identity: Identifier,
    },
    Perpetual {
        moment: RewardDistributionMoment,
        recipient: TokenDistributionResolvedRecipient,
    },
}

impl From<TokenDistributionInfo> for TokenDistributionInfoRepr {
    fn from(m: TokenDistributionInfo) -> Self {
        match m {
            TokenDistributionInfo::PreProgrammed(timestamp, identity) => Self::PreProgrammed {
                timestamp,
                identity,
            },
            TokenDistributionInfo::Perpetual(moment, recipient) => {
                Self::Perpetual { moment, recipient }
            }
        }
    }
}

impl From<TokenDistributionInfoRepr> for TokenDistributionInfo {
    fn from(r: TokenDistributionInfoRepr) -> Self {
        match r {
            TokenDistributionInfoRepr::PreProgrammed {
                timestamp,
                identity,
            } => Self::PreProgrammed(timestamp, identity),
            TokenDistributionInfoRepr::Perpetual { moment, recipient } => {
                Self::Perpetual(moment, recipient)
            }
        }
    }
}

impl From<TokenDistributionInfo> for TokenDistributionTypeWithResolvedRecipient {
    fn from(info: TokenDistributionInfo) -> Self {
        match info {
            TokenDistributionInfo::PreProgrammed(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(recipient)
            }
            TokenDistributionInfo::Perpetual(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::Perpetual(recipient)
            }
        }
    }
}

impl From<&TokenDistributionInfo> for TokenDistributionTypeWithResolvedRecipient {
    fn from(info: &TokenDistributionInfo) -> Self {
        match info {
            TokenDistributionInfo::PreProgrammed(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(*recipient)
            }
            TokenDistributionInfo::Perpetual(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::Perpetual(recipient.clone())
            }
        }
    }
}

impl fmt::Display for TokenDistributionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenDistributionType::PreProgrammed => write!(f, "PreProgrammed"),
            TokenDistributionType::Perpetual => write!(f, "Perpetual"),
        }
    }
}

#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    PlatformDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[platform_serialize(unversioned)]
pub struct TokenDistributionKey {
    pub token_id: Identifier,
    pub recipient: TokenDistributionRecipient,
    pub distribution_type: TokenDistributionType,
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenDistributionTypeWithResolvedRecipient {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenDistributionTypeWithResolvedRecipient {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenDistributionInfo {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenDistributionInfo {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenDistributionType {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenDistributionType {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenDistributionKey {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenDistributionKey {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_token_distribution_type_and_key {
    use super::*;
    use crate::serialization::{JsonConvertible, ValueConvertible};
    use platform_value::{platform_value, Value};
    use serde_json::json;

    #[test]
    fn token_distribution_type_round_trips_all_variants() {
        // Unit-only enum: serde default emits bare PascalCase strings on both
        // wire formats.
        let cases = [
            (TokenDistributionType::PreProgrammed, "PreProgrammed"),
            (TokenDistributionType::Perpetual, "Perpetual"),
        ];
        for (original, expected) in cases {
            let json_v = original.to_json().expect("to_json");
            assert_eq!(json_v, json!(expected));
            assert_eq!(
                TokenDistributionType::from_json(json_v).expect("from_json"),
                original
            );
            let value = original.to_object().expect("to_object");
            assert_eq!(value, platform_value!(expected));
            assert_eq!(
                TokenDistributionType::from_object(value).expect("from_object"),
                original
            );
        }
    }

    fn key_fixture() -> TokenDistributionKey {
        TokenDistributionKey {
            token_id: Identifier::new([0x42; 32]),
            recipient: TokenDistributionRecipient::EvonodesByParticipation,
            distribution_type: TokenDistributionType::Perpetual,
        }
    }

    #[test]
    fn token_distribution_key_json_round_trip_with_full_wire_shape() {
        let original = key_fixture();
        let json = original.to_json().expect("to_json");
        // `recipient` uses TokenDistributionRecipient's custom internally-tagged
        // shape; `token_id` renders as base58. Field names are snake_case (no
        // rename_all on this struct — internal key type, not user-authored JSON).
        assert_eq!(
            json,
            json!({
                "token_id": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
                "recipient": {"$type": "evonodesByParticipation"},
                "distribution_type": "Perpetual",
            })
        );
        let recovered = TokenDistributionKey::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn token_distribution_key_value_round_trip_with_full_wire_shape() {
        let original = key_fixture();
        let value = original.to_object().expect("to_object");
        let expected = Value::Map(vec![
            (
                Value::Text("token_id".to_string()),
                Value::Identifier([0x42; 32]),
            ),
            (
                Value::Text("recipient".to_string()),
                Value::Map(vec![(
                    Value::Text("$type".to_string()),
                    Value::Text("evonodesByParticipation".to_string()),
                )]),
            ),
            (
                Value::Text("distribution_type".to_string()),
                Value::Text("Perpetual".to_string()),
            ),
        ]);
        assert_eq!(value, expected);
        let recovered = TokenDistributionKey::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_token_distribution_info {
    use super::*;
    use platform_value::{Identifier, Value};
    use serde_json::json;

    /// Non-default `PreProgrammed` variant with distinct timestamp + identifier
    /// so the wire-shape assertion catches a silent variant flip or inner-zero
    /// on round-trip.
    fn fixture() -> TokenDistributionInfo {
        TokenDistributionInfo::PreProgrammed(1_700_000_000_000, Identifier::new([0x42; 32]))
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Internally tagged with named fields:
        // `{ "$type":"preProgrammed", "timestamp":<ts>, "identity":<id> }`.
        // `TimestampMillis` is `u64`; JSON erases the size — see the value-
        // path assertion which uses `Value::U64` to lock it in.
        // `Identifier` is rendered as the base58-encoded string in JSON.
        assert_eq!(
            json,
            json!({
                "$type": "preProgrammed",
                "timestamp": 1_700_000_000_000u64,
                "identity": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
            })
        );
        let recovered = TokenDistributionInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Internally tagged with named fields. `Identifier`'s Serialize emits
        // the typed `Value::Identifier` variant (NOT `Value::Bytes32`), which
        // survives serde's internal-tag Content buffer. Built by hand so the
        // typed-bytes variant is preserved exactly.
        let expected = Value::Map(vec![
            (
                Value::Text("$type".to_string()),
                Value::Text("preProgrammed".to_string()),
            ),
            (
                Value::Text("timestamp".to_string()),
                Value::U64(1_700_000_000_000),
            ),
            (
                Value::Text("identity".to_string()),
                Value::Identifier([0x42; 32]),
            ),
        ]);
        assert_eq!(value, expected);
        let recovered = TokenDistributionInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_perpetual_variant() {
        use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionResolvedRecipient;
        use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
        use crate::serialization::JsonConvertible;
        // The Perpetual variant (moment + resolved recipient) complements the
        // PreProgrammed wire-shape test above; pin its `$type` discriminator and
        // full round-trip so a silent variant flip is caught.
        let original = TokenDistributionInfo::Perpetual(
            RewardDistributionMoment::BlockBasedMoment(500),
            TokenDistributionResolvedRecipient::Identity(Identifier::new([0x77; 32])),
        );
        let json = original.to_json().expect("to_json");
        assert_eq!(json["$type"], json!("perpetual"));
        let recovered = TokenDistributionInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }
}
