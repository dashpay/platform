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
/// `maximum`, both included. Registration reads both from the property's schema, requires
/// `minimum <= maximum`, and requires both to fit the property's integer type, at most 64 bits
/// wide, so the range never holds more than 2^64 values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RandomRange {
    /// The smallest value drawn, and the placeholder the document holds until the draw.
    pub minimum: i128,
    /// The largest value drawn.
    pub maximum: i128,
}

impl RandomRange {
    /// The value at `offset` from `minimum`, for an `offset` below [`Self::size`].
    pub fn value_at(&self, offset: u128) -> Option<i128> {
        let offset = i128::try_from(offset).ok()?;
        let value = self.minimum.checked_add(offset)?;
        (value <= self.maximum).then_some(value)
    }

    /// How many values the range holds; at least one.
    pub fn size(&self) -> u128 {
        self.maximum.abs_diff(self.minimum) + 1
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
        let range = RandomRange {
            minimum: -2,
            maximum: 2,
        };
        assert_eq!(range.size(), 5);
        assert_eq!(range.value_at(0), Some(-2));
        assert_eq!(range.value_at(4), Some(2));
        assert_eq!(range.value_at(5), None);

        let widest = RandomRange {
            minimum: 0,
            maximum: u64::MAX.into(),
        };
        assert_eq!(widest.size(), 1u128 << 64);
        assert_eq!(widest.value_at((1u128 << 64) - 1), Some(u64::MAX.into()));
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
