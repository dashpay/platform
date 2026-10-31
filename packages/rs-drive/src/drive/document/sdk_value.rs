//! Building the plain values `drive::document::layout` and
//! `drive::document::cost` hand to the SDKs, in the shapes JavaScript reads
//! well.

use dpp::platform_value::{Value, ValueMap};

pub(crate) fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

/// A map of `entries`, leaving out a key whose value is absent (`Null`):
/// JavaScript reads a null that crosses into it as undefined anyway.
pub(crate) fn map(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .filter(|(_, value)| !value.is_null())
            .map(|(key, value)| (text(key), value))
            .collect::<ValueMap>(),
    )
}

pub(crate) fn texts(values: &[String]) -> Value {
    Value::Array(values.iter().map(|value| text(value)).collect())
}

/// A count as a value JavaScript reads as a number: a u32 when it fits,
/// else a float (exact up to 2^53), rather than a u64, which becomes a
/// BigInt.
pub(crate) fn number(value: u64) -> Value {
    u32::try_from(value)
        .map(Value::U32)
        .unwrap_or(Value::Float(value as f64))
}
