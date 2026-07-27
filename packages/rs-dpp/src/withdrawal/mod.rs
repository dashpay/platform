pub mod daily_withdrawal_limit;
#[cfg(all(feature = "withdrawals-contract", feature = "system_contracts"))]
mod document_try_into_asset_unlock_base_transaction_info;

use bincode::{Decode, Encode};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;

#[repr(u8)]
#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Encode, Decode, Default,
)]
pub enum Pooling {
    #[default]
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

#[cfg(feature = "json-conversion")]
impl JsonConvertible for Pooling {}

#[cfg(feature = "value-conversion")]
impl ValueConvertible for Pooling {}

/// Transaction index type
pub type WithdrawalTransactionIndex = u64;

/// Simple type alias for withdrawal transaction with it's index
pub type WithdrawalTransactionIndexAndBytes = (WithdrawalTransactionIndex, Vec<u8>);

/// Serde helper for `Pooling` fields exposed through the JS surface.
///
/// `Pooling` is `#[repr(u8)]` with `Serialize_repr` / `Deserialize_repr`, so the
/// default wire shape is the numeric discriminant (`0`/`1`/`2`). That number
/// leaks into JSON / Object output and makes `XxxJSON.pooling: string`
/// declarations false. The helper switches the **human-readable** path to a
/// camelCase string (`"never"`/`"ifAvailable"`/`"standard"`) while keeping the
/// non-HR path at the original `u8` so bincode (consensus binary format) is
/// untouched.
///
/// Apply via `#[serde(with = "crate::withdrawal::pooling_serde")]` on the
/// `pooling` field of any state transition that surfaces it to JS.
#[cfg(feature = "serde-conversion")]
pub mod pooling_serde {
    use super::Pooling;
    use serde::{Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(pooling: &Pooling, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let name = match pooling {
                Pooling::Never => "never",
                Pooling::IfAvailable => "ifAvailable",
                Pooling::Standard => "standard",
            };
            serializer.serialize_str(name)
        } else {
            (*pooling as u8).serialize(serializer)
        }
    }

    /// Deserialize accepts both shapes regardless of the deserializer's
    /// human-readable flag — mirrors the `BinaryData` / `Identifier` pattern.
    /// Necessary because `platform_value::to_value` reports HR=false (emits the
    /// numeric discriminant on the way to `JsValue`), but
    /// `platform_value::from_value` reports HR=true on the way back. Without
    /// dual acceptance, the `fromObject(toObject())` round-trip fails on the
    /// `pooling` field.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Pooling, D::Error> {
        struct PoolingVisitor;

        impl<'de> serde::de::Visitor<'de> for PoolingVisitor {
            type Value = Pooling;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a Pooling variant: 'never'/'ifAvailable'/'standard' or 0/1/2")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Pooling, E> {
                match v {
                    "never" | "Never" => Ok(Pooling::Never),
                    "ifAvailable" | "IfAvailable" | "ifavailable" => Ok(Pooling::IfAvailable),
                    "standard" | "Standard" => Ok(Pooling::Standard),
                    other => Err(E::custom(format!(
                        "unknown pooling variant '{}', expected 'never' | 'ifAvailable' | 'standard'",
                        other
                    ))),
                }
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Pooling, E> {
                self.visit_str(&v)
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Pooling, E> {
                match v {
                    0 => Ok(Pooling::Never),
                    1 => Ok(Pooling::IfAvailable),
                    2 => Ok(Pooling::Standard),
                    other => Err(E::custom(format!("unknown pooling discriminant {}", other))),
                }
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Pooling, E> {
                if v < 0 {
                    return Err(E::custom(format!("negative pooling discriminant {}", v)));
                }
                self.visit_u64(v as u64)
            }

            fn visit_u8<E: serde::de::Error>(self, v: u8) -> Result<Pooling, E> {
                self.visit_u64(v as u64)
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_any(PoolingVisitor)
        } else {
            deserializer.deserialize_u8(PoolingVisitor)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Wrap(#[serde(with = "super")] Pooling);

        #[test]
        fn json_emits_camelcase_string() {
            for (variant, expected) in [
                (Pooling::Never, "\"never\""),
                (Pooling::IfAvailable, "\"ifAvailable\""),
                (Pooling::Standard, "\"standard\""),
            ] {
                let json = serde_json::to_string(&Wrap(variant)).expect("serialize");
                assert_eq!(json, expected);
                let restored: Wrap = serde_json::from_str(expected).expect("deserialize");
                assert_eq!(restored, Wrap(variant));
            }
        }

        #[test]
        fn bincode_keeps_u8_discriminant() {
            for (variant, expected_u8) in [
                (Pooling::Never, 0),
                (Pooling::IfAvailable, 1),
                (Pooling::Standard, 2),
            ] {
                let bytes =
                    bincode::serde::encode_to_vec(Wrap(variant), bincode::config::standard())
                        .expect("bincode encode");
                assert_eq!(bytes.last(), Some(&expected_u8));
                let (restored, _): (Wrap, usize) =
                    bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                        .expect("bincode decode");
                assert_eq!(restored, Wrap(variant));
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
mod json_convertible_tests_pooling {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // `Pooling` is `#[repr(u8)]` with `Serialize_repr` / `Deserialize_repr`, so
    // the wire shape is the raw `u8` discriminant: `0` / `1` / `2`. JSON has
    // only one number type, so `0u8` is erased to `Number(0)`; the value-path
    // assertion uses explicit `0u8` etc. to lock in `Value::U8`.

    #[test]
    fn json_round_trip_never() {
        use crate::serialization::JsonConvertible;
        let original = Pooling::Never;
        let json = original.to_json().expect("to_json");
        // u8 size erased in JSON.
        assert_eq!(json, json!(0));
        let recovered = Pooling::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_if_available() {
        use crate::serialization::JsonConvertible;
        let original = Pooling::IfAvailable;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!(1));
        let recovered = Pooling::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_standard() {
        use crate::serialization::JsonConvertible;
        let original = Pooling::Standard;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!(2));
        let recovered = Pooling::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_never() {
        use crate::serialization::ValueConvertible;
        let original = Pooling::Never;
        let value = original.to_object().expect("to_object");
        // `0u8` locks `Value::U8` (not I32 from a bare `0`).
        assert_eq!(value, platform_value!(0u8));
        let recovered = Pooling::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_if_available() {
        use crate::serialization::ValueConvertible;
        let original = Pooling::IfAvailable;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!(1u8));
        let recovered = Pooling::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_standard() {
        use crate::serialization::ValueConvertible;
        let original = Pooling::Standard;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!(2u8));
        let recovered = Pooling::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
