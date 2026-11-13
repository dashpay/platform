//! `sys.random`: system functions that draw a value nobody can know when the document is
//! sent (protocol version 15).
//!
//! A property one of them generates is written with a placeholder when the document is
//! created, and the platform replaces the placeholder before the block commits, with a value
//! derived from the block's random beacon: the threshold signature the validators' prevotes
//! for the block combine into, which no one can compute before the block is accepted. The
//! value is kept after: a replace leaves it as it is. A client cannot know it, so it leaves
//! the property out, like a property generated from state.

use crate::data_contract::document_type::DocumentPropertyType;
use platform_value::Value;
use serde::{Deserialize, Serialize};

/// A `sys.random` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomFunction {
    /// `sys.random.uniform`: an integer drawn uniformly from the property's `minimum` to its
    /// `maximum`, both included ([`RandomRange`]).
    Uniform,
}

impl RandomFunction {
    /// Every random function, in wire-name order.
    pub const ALL: [RandomFunction; 1] = [RandomFunction::Uniform];

    /// The wire name, the value of `generatedFrom.function`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            RandomFunction::Uniform => "sys.random.uniform",
        }
    }
}

/// The integers a `sys.random.uniform` property is drawn from: the property's `minimum` to its
/// `maximum`, both included. Registration reads both from the property's schema and requires
/// them to fit the property's integer type, at most 64 bits wide. [`Self::new`] holds every
/// range to `minimum <= maximum` and at most 2^64 values, which a draw maps 128 random bits
/// onto, and so does the deserialization of one: the fields are private, so no range breaks
/// either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "UncheckedRandomRange")]
pub struct RandomRange {
    /// The smallest value drawn, and the placeholder the document holds until the draw.
    minimum: i128,
    /// The largest value drawn.
    maximum: i128,
}

/// The fields of a [`RandomRange`] as sent, checked by its `TryFrom`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedRandomRange {
    minimum: i128,
    maximum: i128,
}

impl TryFrom<UncheckedRandomRange> for RandomRange {
    type Error = String;

    fn try_from(unchecked: UncheckedRandomRange) -> Result<Self, Self::Error> {
        let UncheckedRandomRange { minimum, maximum } = unchecked;
        RandomRange::new(minimum, maximum).ok_or_else(|| {
            format!(
                "a random range holds its minimum, {minimum}, to its maximum, {maximum}: the \
                 minimum may not be above the maximum, and the range may hold at most 2^64 values"
            )
        })
    }
}

impl RandomRange {
    /// The most values a range holds, 2^64: a draw maps 128 random bits onto the range, which
    /// favours no value of a range this size or smaller by more than 2^-64.
    pub const MAX_SIZE: u128 = 1 << 64;

    /// The range from `minimum` to `maximum`, both included; `None` when `minimum` is above
    /// `maximum`, or the range would hold more than [`Self::MAX_SIZE`] values.
    pub fn new(minimum: i128, maximum: i128) -> Option<Self> {
        // The values past the minimum: not negative, and at most 2^64 - 1, so a u64 holds them
        let span = maximum.checked_sub(minimum)?;
        (0..=i128::from(u64::MAX))
            .contains(&span)
            .then_some(RandomRange { minimum, maximum })
    }

    /// The smallest value drawn, and the placeholder the document holds until the draw.
    pub fn minimum(&self) -> i128 {
        self.minimum
    }

    /// The largest value drawn.
    pub fn maximum(&self) -> i128 {
        self.maximum
    }

    /// The value at `offset` from `minimum`, for an `offset` below [`Self::size`].
    pub fn value_at(&self, offset: u128) -> Option<i128> {
        let offset = i128::try_from(offset).ok()?;
        let value = self.minimum.checked_add(offset)?;
        (value <= self.maximum).then_some(value)
    }

    /// How many values the range holds: at least one and at most [`Self::MAX_SIZE`]. One more
    /// than a u64, so the sum cannot overflow.
    pub fn size(&self) -> u128 {
        u128::from(self.span()) + 1
    }

    /// How many values the range holds past its minimum.
    fn span(&self) -> u64 {
        // `new` admits only a `maximum` at most `u64::MAX` past `minimum`, so neither the
        // difference nor the conversion fails
        self.maximum
            .checked_sub(self.minimum)
            .and_then(|span| u64::try_from(span).ok())
            .unwrap_or(u64::MAX)
    }
}

/// `value` as `property_type` stores it, `None` when the type is not an integer one of at most
/// 64 bits or `value` does not fit it.
pub fn integer_value_for_type(property_type: &DocumentPropertyType, value: i128) -> Option<Value> {
    Some(match property_type {
        DocumentPropertyType::U8 => Value::U8(u8::try_from(value).ok()?),
        DocumentPropertyType::U16 => Value::U16(u16::try_from(value).ok()?),
        DocumentPropertyType::U32 => Value::U32(u32::try_from(value).ok()?),
        DocumentPropertyType::U64 => Value::U64(u64::try_from(value).ok()?),
        DocumentPropertyType::I8 => Value::I8(i8::try_from(value).ok()?),
        DocumentPropertyType::I16 => Value::I16(i16::try_from(value).ok()?),
        DocumentPropertyType::I32 => Value::I32(i32::try_from(value).ok()?),
        DocumentPropertyType::I64 => Value::I64(i64::try_from(value).ok()?),
        _ => return None,
    })
}

/// The smallest and largest values `property_type` holds, for the integer types of at most 64
/// bits, the ones a `sys.random.uniform` property may have.
pub fn integer_type_bounds(property_type: &DocumentPropertyType) -> Option<(i128, i128)> {
    Some(match property_type {
        DocumentPropertyType::U8 => (0, u8::MAX.into()),
        DocumentPropertyType::U16 => (0, u16::MAX.into()),
        DocumentPropertyType::U32 => (0, u32::MAX.into()),
        DocumentPropertyType::U64 => (0, u64::MAX.into()),
        DocumentPropertyType::I8 => (i8::MIN.into(), i8::MAX.into()),
        DocumentPropertyType::I16 => (i16::MIN.into(), i16::MAX.into()),
        DocumentPropertyType::I32 => (i32::MIN.into(), i32::MAX.into()),
        DocumentPropertyType::I64 => (i64::MIN.into(), i64::MAX.into()),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_count_and_index_every_value_of_the_range() {
        let range = RandomRange::new(-2, 2).expect("a range");
        assert_eq!(range.size(), 5);
        assert_eq!(range.value_at(0), Some(-2));
        assert_eq!(range.value_at(4), Some(2));
        assert_eq!(range.value_at(5), None);

        let single = RandomRange::new(7, 7).expect("a range of one value");
        assert_eq!(single.size(), 1);
        assert_eq!(single.value_at(0), Some(7));

        let widest = RandomRange::new(0, u64::MAX.into()).expect("2^64 values");
        assert_eq!(widest.size(), RandomRange::MAX_SIZE);
        assert_eq!(widest.value_at((1u128 << 64) - 1), Some(u64::MAX.into()));
        let widest_signed =
            RandomRange::new(i64::MIN.into(), i64::MAX.into()).expect("2^64 values");
        assert_eq!(widest_signed.size(), RandomRange::MAX_SIZE);
    }

    #[test]
    fn should_refuse_a_range_upside_down_or_of_more_than_2_pow_64_values() {
        assert_eq!(RandomRange::new(10, 1), None);
        assert_eq!(RandomRange::new(0, i128::from(u64::MAX) + 1), None);
        assert_eq!(RandomRange::new(-1, u64::MAX.into()), None);
        // The widest pair of bounds, whose difference does not even fit an i128
        assert_eq!(RandomRange::new(i128::MIN, i128::MAX), None);
    }

    #[test]
    fn should_deserialize_only_a_range_new_admits() {
        let range: RandomRange =
            serde_json::from_str(r#"{ "minimum": 1, "maximum": 100 }"#).expect("a range");
        assert_eq!((range.minimum(), range.maximum()), (1, 100));
        assert_eq!(
            serde_json::to_value(range).expect("serializes"),
            serde_json::json!({ "minimum": 1, "maximum": 100 })
        );
        assert!(
            serde_json::from_str::<RandomRange>(r#"{ "minimum": 100, "maximum": 1 }"#).is_err()
        );
        assert!(serde_json::from_str::<RandomRange>(
            r#"{ "minimum": -1, "maximum": 18446744073709551615 }"#
        )
        .is_err());
        assert!(serde_json::from_str::<RandomRange>(
            r#"{ "minimum": 1, "maximum": 100, "extra": 0 }"#
        )
        .is_err());
    }

    #[test]
    fn should_store_a_value_at_the_property_width_only_when_it_fits() {
        assert_eq!(
            integer_value_for_type(&DocumentPropertyType::U8, 200),
            Some(Value::U8(200))
        );
        assert_eq!(integer_value_for_type(&DocumentPropertyType::U8, 256), None);
        assert_eq!(
            integer_value_for_type(&DocumentPropertyType::I16, -5),
            Some(Value::I16(-5))
        );
        assert_eq!(integer_value_for_type(&DocumentPropertyType::U128, 1), None);
        assert_eq!(
            integer_type_bounds(&DocumentPropertyType::I64),
            Some((i64::MIN.into(), i64::MAX.into()))
        );
        assert_eq!(integer_type_bounds(&DocumentPropertyType::I128), None);
    }
}
