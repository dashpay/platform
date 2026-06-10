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
pub enum TokenDistributionTypeWithResolvedRecipient {
    /// A scheduled distribution with a known recipient.
    PreProgrammed(Identifier),

    /// A perpetual distribution with a resolved recipient.
    Perpetual(TokenDistributionResolvedRecipient),
}

/// Contains information about a specific token distribution instance.
///
/// - `PreProgrammed(TimestampMillis, Identifier)`: A scheduled distribution with a timestamp and recipient.
/// - `Perpetual(RewardDistributionMoment, RewardDistributionMoment, TokenDistributionResolvedRecipient)`:
///   A perpetual distribution with previous and next distribution moments, along with the resolved recipient.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum TokenDistributionInfo {
    /// A pre-programmed token distribution set for a specific time.
    /// Contains the scheduled timestamp and the recipient’s identifier.
    //
    // `TimestampMillis` is a `u64` in a tuple variant that `#[json_safe_fields]`
    // can't auto-annotate; apply the JS-safe helper directly (string in HR JSON
    // above `MAX_SAFE_INTEGER`). `RewardDistributionMoment` in `Perpetual` is
    // already JS-safe via its own `#[serde(with)]`.
    PreProgrammed(
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json_safe_u64")
        )]
        TimestampMillis,
        Identifier,
    ),

    /// A perpetual token distribution with moment for distribution.
    /// The moment is the beginning of the perpetual distribution cycle
    /// Includes the last and next distribution times and the resolved recipient.
    Perpetual(RewardDistributionMoment, TokenDistributionResolvedRecipient),
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
                "recipient": {"type": "evonodesByParticipation"},
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
                    Value::Text("type".to_string()),
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
        // Externally-tagged tuple variant: `{ "PreProgrammed": [<ts>, <id>] }`.
        // `TimestampMillis` is `u64`; JSON erases the size — see the value-
        // path assertion which uses `1_700_000_000_000u64` to lock in `Value::U64`.
        // `Identifier` is rendered as the base58-encoded string in JSON.
        assert_eq!(
            json,
            json!({
                "PreProgrammed": [
                    1_700_000_000_000u64,
                    "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
                ],
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
        // `Identifier`'s `Serialize` impl emits the typed `Value::Identifier`
        // variant (NOT `Value::Bytes32`). `platform_value!` interpolation goes
        // through Serialize, so a raw `Value::Identifier(...)` literal in the
        // macro would conflict — instead we construct the expected map by hand
        // so the variant is preserved exactly.
        let expected = Value::Map(vec![(
            Value::Text("PreProgrammed".to_string()),
            Value::Array(vec![
                Value::U64(1_700_000_000_000),
                Value::Identifier([0x42; 32]),
            ]),
        )]);
        assert_eq!(value, expected);
        let recovered = TokenDistributionInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
