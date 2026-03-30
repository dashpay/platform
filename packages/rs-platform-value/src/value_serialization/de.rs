use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use core::{fmt, slice};
use std::iter::Peekable;

use serde::de::value::SeqDeserializer;
use serde::de::{self, Deserializer as _, IntoDeserializer};

use crate::{Error, Value};

impl<'a> From<&'a Value> for de::Unexpected<'a> {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Bool(x) => Self::Bool(*x),
            Value::Float(x) => Self::Float(*x),
            Value::Bytes(x) => Self::Bytes(x),
            Value::Text(x) => Self::Str(x),
            Value::Array(..) => Self::Seq,
            Value::Map(..) => Self::Map,
            Value::Null => Self::Other("null"),
            Value::U128(_x) => todo!(), // TODO: it seems serde is not happy about u128
            Value::I128(_x) => todo!(), // TODO: ... and for i128 either
            Value::U64(x) => Self::Unsigned(*x),
            Value::I64(x) => Self::Signed(*x),
            Value::U32(x) => Self::Unsigned(*x as u64),
            Value::I32(x) => Self::Signed(*x as i64),
            Value::U16(x) => Self::Unsigned(*x as u64),
            Value::I16(x) => Self::Signed(*x as i64),
            Value::U8(x) => Self::Unsigned(*x as u64),
            Value::I8(x) => Self::Signed(*x as i64),
            Value::Bytes20(x) => Self::Bytes(x),
            Value::Bytes32(x) => Self::Bytes(x),
            Value::Bytes36(x) => Self::Bytes(x),
            Value::EnumU8(_x) => todo!(),
            Value::EnumString(_x) => todo!(),
            Value::Identifier(x) => Self::Bytes(x),
        }
    }
}

macro_rules! mkvisit {
    ($($f:ident($v:ty)),+ $(,)?) => {
        $(
                        fn $f<E: de::Error>(self, v: $v) -> Result<Self::Value, E> {
                Ok(v.into())
            }
        )+
    };
}

struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a valid platform value item")
    }

    mkvisit! {
        visit_bool(bool),
        visit_f32(f32),
        visit_f64(f64),

        visit_i8(i8),
        visit_i16(i16),
        visit_i32(i32),
        visit_i64(i64),
        visit_i128(i128),

        visit_u8(u8),
        visit_u16(u16),
        visit_u32(u32),
        visit_u64(u64),
        visit_u128(u128),

        visit_char(char),
        visit_str(&str),
        visit_borrowed_str(&'de str),
        visit_string(String),

        visit_bytes(&[u8]),
        visit_borrowed_bytes(&'de [u8]),
        visit_byte_buf(Vec<u8>),
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_newtype_struct<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut acc: A) -> Result<Self::Value, A::Error> {
        let mut seq = Vec::new();

        while let Some(elem) = acc.next_element()? {
            seq.push(elem);
        }

        Ok(Value::Array(seq))
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut acc: A) -> Result<Self::Value, A::Error> {
        let mut map = Vec::<(Value, Value)>::new();

        while let Some(kv) = acc.next_entry()? {
            map.push(kv);
        }

        Ok(Value::Map(map))
    }

    fn visit_enum<A: de::EnumAccess<'de>>(self, acc: A) -> Result<Self::Value, A::Error> {
        use serde::de::VariantAccess;
        struct Inner;
        impl<'de> de::Visitor<'de> for Inner {
            type Value = Value;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a valid CBOR item")
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut acc: A) -> Result<Self::Value, A::Error> {
                match acc.size_hint() {
                    Some(1) => {
                        let tag: u8 = acc
                            .next_element()?
                            .ok_or_else(|| de::Error::custom("expected tag"))?;
                        Ok(Value::EnumU8(vec![tag]))
                    }
                    _ => {
                        let val: Vec<String> = de::Deserialize::deserialize(
                            de::value::SeqAccessDeserializer::new(acc),
                        )?;
                        Ok(Value::EnumString(val))
                    }
                }
            }
        }
        let (name, data): (String, _) = acc.variant()?;
        if name == "@@TAGGED@@" {
            data.tuple_variant(2, Inner)
        } else {
            Err(de::Error::custom(format!(
                "Unexpected variant name: {}",
                name
            )))
        }
    }
}

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor)
    }
}

pub(crate) struct Deserializer<T>(pub(crate) T);

impl<'de> de::Deserializer<'de> for Deserializer<Value> {
    type Error = Error;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let human_readable = self.is_human_readable();
        match self.0 {
            Value::Bytes(x) => {
                if human_readable {
                    visitor.visit_str(BASE64_STANDARD.encode(x).as_str())
                } else {
                    visitor.visit_bytes(&x)
                }
            }
            Value::Text(x) => visitor.visit_str(&x),
            Value::Array(x) => visitor.visit_seq(ArrayDeserializer(x.iter())),
            Value::Map(x) => visitor.visit_map(ValueMapDeserializer(x.iter().peekable())),
            Value::Bool(x) => visitor.visit_bool(x),
            Value::Null => visitor.visit_none(),
            Value::Float(x) => visitor.visit_f64(x),
            Value::U128(x) => visitor.visit_u128(x),
            Value::I128(x) => visitor.visit_i128(x),
            Value::U64(x) => visitor.visit_u64(x),
            Value::I64(x) => visitor.visit_i64(x),
            Value::U32(x) => visitor.visit_u32(x),
            Value::I32(x) => visitor.visit_i32(x),
            Value::U16(x) => visitor.visit_u16(x),
            Value::I16(x) => visitor.visit_i16(x),
            Value::U8(x) => visitor.visit_u8(x),
            Value::I8(x) => visitor.visit_i8(x),
            Value::Bytes20(x) => {
                if human_readable {
                    visitor.visit_str(BASE64_STANDARD.encode(x).as_str())
                } else {
                    visitor.visit_bytes(&x)
                }
            }
            Value::Bytes32(x) => {
                if human_readable {
                    visitor.visit_str(BASE64_STANDARD.encode(x).as_str())
                } else {
                    visitor.visit_bytes(&x)
                }
            }
            Value::Bytes36(x) => {
                if human_readable {
                    visitor.visit_str(BASE64_STANDARD.encode(x).as_str())
                } else {
                    visitor.visit_bytes(&x)
                }
            }
            Value::EnumU8(x) => visitor.visit_seq(SeqDeserializer::new(x.into_iter())),
            Value::EnumString(x) => visitor.visit_seq(SeqDeserializer::new(x.into_iter())),
            Value::Identifier(x) => {
                if human_readable {
                    visitor.visit_str(bs58::encode(x).into_string().as_str())
                } else {
                    visitor.visit_bytes(&x)
                }
            }
        }
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;

        match value {
            Value::Bool(x) => visitor.visit_bool(x),
            _ => Err(de::Error::invalid_type((&value).into(), &"bool")),
        }
    }

    fn deserialize_f32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;

        match value {
            Value::Float(x) => visitor.visit_f64(x),
            _ => Err(de::Error::invalid_type((&value).into(), &"f64")),
        }
    }

    fn deserialize_i8<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_i8(value.to_integer()?)
    }

    fn deserialize_i16<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_i16(value.to_integer()?)
    }

    fn deserialize_i32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_i32(value.to_integer()?)
    }

    fn deserialize_i64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_i64(value.to_integer()?)
    }

    fn deserialize_i128<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_i128(value.to_integer()?)
    }

    fn deserialize_u8<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_u8(value.to_integer()?)
    }

    fn deserialize_u16<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_u16(value.to_integer()?)
    }

    fn deserialize_u32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_u32(value.to_integer()?)
    }

    fn deserialize_u64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_u64(value.to_integer()?)
    }

    fn deserialize_u128<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;
        visitor.visit_u128(value.to_integer()?)
    }

    fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;

        match value {
            Value::Text(ref x) => match x.chars().count() {
                1 => visitor.visit_char(x.chars().next().unwrap()),
                _ => Err(de::Error::invalid_type((&value).into(), &"char")),
            },

            _ => Err(de::Error::invalid_type((&value).into(), &"char")),
        }
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;

        match value {
            Value::Text(x) => visitor.visit_str(&x),
            _ => Err(de::Error::invalid_type((&value).into(), &"str")),
        }
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;

        match value {
            Value::Bytes(x) => visitor.visit_bytes(&x),
            Value::Bytes20(x) => visitor.visit_bytes(x.as_slice()),
            Value::Bytes32(x) => visitor.visit_bytes(x.as_slice()),
            Value::Bytes36(x) => visitor.visit_bytes(x.as_slice()),
            Value::Identifier(x) => visitor.visit_bytes(x.as_slice()),
            _ => Err(de::Error::invalid_type((&value).into(), &"bytes")),
        }
    }

    fn deserialize_byte_buf<V: de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;

        match value {
            Value::Array(x) => visitor.visit_seq(ArrayDeserializer(x.iter())),
            _ => Err(de::Error::invalid_type((&value).into(), &"array")),
        }
    }

    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.0;

        match value {
            Value::Map(x) => visitor.visit_map(ValueMapDeserializer(x.iter().peekable())),
            _ => Err(de::Error::invalid_type((&value).into(), &"map")),
        }
    }

    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_tuple<V: de::Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_identifier<V: de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Null => visitor.visit_none(),
            x => visitor.visit_some(Self(x)),
        }
    }

    fn deserialize_unit<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            _ => Err(de::Error::invalid_type((&self.0).into(), &"null")),
        }
    }

    fn deserialize_unit_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.0 {
            // Existing CBOR enum types — keep for backward compatibility
            Value::EnumU8(x) => {
                let enum_variant = x.first().ok_or_else(|| {
                    de::Error::invalid_length(0, &"at least one variant expected")
                })?;
                let variant_name = format!("Variant{}", enum_variant);
                visitor.visit_enum(variant_name.into_deserializer())
            }
            Value::EnumString(x) => {
                let variant_name = x
                    .first()
                    .ok_or_else(|| de::Error::invalid_length(0, &"at least one variant expected"))?
                    .clone();
                visitor.visit_enum(variant_name.into_deserializer())
            }

            // String → unit variant (e.g., "Abstain", "Lock", "NoWinner")
            Value::Text(variant) => visitor.visit_enum(ValueEnumDeserializer {
                variant,
                value: None,
            }),

            // Single-key map → externally tagged variant
            // e.g., {"TowardsIdentity": <data>} or {"ResourceVote": {...}}
            Value::Map(entries) if entries.len() == 1 => {
                let (key, value) = entries
                    .into_iter()
                    .next()
                    .expect("guard guarantees len == 1");
                let variant = match key {
                    Value::Text(s) => s,
                    other => {
                        return Err(de::Error::invalid_type(
                            (&other).into(),
                            &"string variant name",
                        ))
                    }
                };
                visitor.visit_enum(ValueEnumDeserializer {
                    variant,
                    value: Some(value),
                })
            }

            other => Err(de::Error::invalid_type((&other).into(), &"enum")),
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct ArrayDeserializer<'a>(slice::Iter<'a, Value>);

impl<'de> de::SeqAccess<'de> for ArrayDeserializer<'_> {
    type Error = Error;

    fn next_element_seed<U: de::DeserializeSeed<'de>>(
        &mut self,
        seed: U,
    ) -> Result<Option<U::Value>, Self::Error> {
        self.0
            .next()
            .map(|x| seed.deserialize(Deserializer(x.clone())))
            .transpose() // TODO
    }
}

struct ValueMapDeserializer<'a>(Peekable<slice::Iter<'a, (Value, Value)>>);

impl<'de> de::MapAccess<'de> for ValueMapDeserializer<'_> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        // Serde expect `key` call to go first, thus it should not move iterator
        // as `value` call should follow
        self.0
            .peek()
            .map(|x| seed.deserialize(Deserializer(x.0.clone()))) // TODO
            .transpose()
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let map_value = self
            .0
            .next()
            .expect("`next_key_seed` must be called first")
            .1
            .clone(); // TODO
        seed.deserialize(Deserializer(map_value))
    }
}

/// EnumAccess for deserializing externally-tagged enums from Value.
struct ValueEnumDeserializer {
    variant: String,
    value: Option<Value>,
}

impl<'de> de::EnumAccess<'de> for ValueEnumDeserializer {
    type Error = Error;
    type Variant = ValueVariantDeserializer;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((variant, ValueVariantDeserializer { value: self.value }))
    }
}

/// VariantAccess that delegates to Deserializer<Value> for variant data.
struct ValueVariantDeserializer {
    value: Option<Value>,
}

impl<'de> de::VariantAccess<'de> for ValueVariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            None => Ok(()),
            Some(Value::Null) => Ok(()),
            Some(other) => Err(de::Error::invalid_type((&other).into(), &"unit variant")),
        }
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        match self.value {
            Some(value) => seed.deserialize(Deserializer(value)),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V: de::Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value {
            Some(value) => Deserializer(value).deserialize_seq(visitor),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value {
            Some(value) => Deserializer(value).deserialize_map(visitor),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

impl<'de> de::VariantAccess<'de> for Deserializer<Value> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.0 {
            Value::Null => Ok(()),
            v => Err(de::Error::invalid_type((&v).into(), &"unit")),
        }
    }

    fn newtype_variant_seed<U: de::DeserializeSeed<'de>>(
        self,
        seed: U,
    ) -> Result<U::Value, Self::Error> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: de::Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_value, to_value};
    use serde::{Deserialize, Serialize};

    // ---------------------------------------------------------------
    // Deserialize Value via serde (Value -> Rust types)
    // ---------------------------------------------------------------

    #[test]
    fn deserialize_bool() {
        let val = Value::Bool(true);
        let result: bool = from_value(val).unwrap();
        assert!(result);
    }

    #[test]
    fn deserialize_u8() {
        let val = Value::U8(42);
        let result: u8 = from_value(val).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn deserialize_u16() {
        let val = Value::U16(1000);
        let result: u16 = from_value(val).unwrap();
        assert_eq!(result, 1000);
    }

    #[test]
    fn deserialize_u32() {
        let val = Value::U32(100_000);
        let result: u32 = from_value(val).unwrap();
        assert_eq!(result, 100_000);
    }

    #[test]
    fn deserialize_u64() {
        let val = Value::U64(10_000_000_000);
        let result: u64 = from_value(val).unwrap();
        assert_eq!(result, 10_000_000_000);
    }

    #[test]
    fn deserialize_u128() {
        let val = Value::U128(u128::MAX);
        let result: u128 = from_value(val).unwrap();
        assert_eq!(result, u128::MAX);
    }

    #[test]
    fn deserialize_i8() {
        let val = Value::I8(-42);
        let result: i8 = from_value(val).unwrap();
        assert_eq!(result, -42);
    }

    #[test]
    fn deserialize_i16() {
        let val = Value::I16(-1000);
        let result: i16 = from_value(val).unwrap();
        assert_eq!(result, -1000);
    }

    #[test]
    fn deserialize_i32() {
        let val = Value::I32(-100_000);
        let result: i32 = from_value(val).unwrap();
        assert_eq!(result, -100_000);
    }

    #[test]
    fn deserialize_i64() {
        let val = Value::I64(-10_000_000_000);
        let result: i64 = from_value(val).unwrap();
        assert_eq!(result, -10_000_000_000);
    }

    #[test]
    fn deserialize_i128() {
        let val = Value::I128(i128::MIN);
        let result: i128 = from_value(val).unwrap();
        assert_eq!(result, i128::MIN);
    }

    #[test]
    fn deserialize_f64() {
        let val = Value::Float(3.14);
        let result: f64 = from_value(val).unwrap();
        assert!((result - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn deserialize_string() {
        let val = Value::Text("hello".into());
        let result: String = from_value(val).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn deserialize_null_as_option_none() {
        let val = Value::Null;
        let result: Option<String> = from_value(val).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn deserialize_some_as_option_some() {
        let val = Value::Text("hi".into());
        let result: Option<String> = from_value(val).unwrap();
        assert_eq!(result, Some("hi".to_string()));
    }

    #[test]
    fn deserialize_array_of_integers() {
        let val = Value::Array(vec![Value::U32(1), Value::U32(2), Value::U32(3)]);
        let result: Vec<u32> = from_value(val).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn deserialize_unit_from_null() {
        let val = Value::Null;
        let result: () = from_value(val).unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn deserialize_unit_from_non_null_errors() {
        let val = Value::U8(1);
        let result: Result<(), Error> = from_value(val);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_char_from_single_char_text() {
        let val = Value::Text("A".into());
        let result: char = from_value(val).unwrap();
        assert_eq!(result, 'A');
    }

    #[test]
    fn deserialize_char_from_multi_char_text_errors() {
        let val = Value::Text("AB".into());
        let result: Result<char, Error> = from_value(val);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_struct_from_map() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Point {
            x: i32,
            y: i32,
        }

        let val = Value::Map(vec![
            (Value::Text("x".into()), Value::I32(10)),
            (Value::Text("y".into()), Value::I32(20)),
        ]);
        let result: Point = from_value(val).unwrap();
        assert_eq!(result, Point { x: 10, y: 20 });
    }

    #[test]
    fn deserialize_tuple_from_array() {
        let val = Value::Array(vec![Value::U32(1), Value::U32(2)]);
        let result: (u32, u32) = from_value(val).unwrap();
        assert_eq!(result, (1, 2));
    }

    #[test]
    fn deserialize_bool_type_mismatch_errors() {
        let val = Value::U32(1);
        let result: Result<bool, Error> = from_value(val);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_f64_type_mismatch_errors() {
        let val = Value::Text("not_a_float".into());
        let result: Result<f64, Error> = from_value(val);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_string_type_mismatch_errors() {
        let val = Value::U32(42);
        let result: Result<String, Error> = from_value(val);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_seq_type_mismatch_errors() {
        let val = Value::U32(42);
        let result: Result<Vec<u32>, Error> = from_value(val);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_map_type_mismatch_errors() {
        let val = Value::U32(42);
        let result: Result<std::collections::HashMap<String, u32>, Error> = from_value(val);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Round-trip serialization: Rust -> Value -> Rust
    // ---------------------------------------------------------------

    #[test]
    fn round_trip_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Data {
            name: String,
            count: u64,
            active: bool,
        }

        let original = Data {
            name: "test".into(),
            count: 42,
            active: true,
        };
        let val = to_value(&original).unwrap();
        let recovered: Data = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_vec() {
        let original = vec![1u32, 2, 3, 4, 5];
        let val = to_value(&original).unwrap();
        let recovered: Vec<u32> = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_option_some() {
        let original: Option<String> = Some("hello".into());
        let val = to_value(&original).unwrap();
        let recovered: Option<String> = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_option_none() {
        let original: Option<String> = None;
        let val = to_value(&original).unwrap();
        let recovered: Option<String> = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_nested_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Inner {
            value: i32,
        }
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Outer {
            inner: Inner,
            label: String,
        }

        let original = Outer {
            inner: Inner { value: -5 },
            label: "test".into(),
        };
        let val = to_value(&original).unwrap();
        let recovered: Outer = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_hashmap() {
        let mut original = std::collections::HashMap::new();
        original.insert("a".to_string(), 1u32);
        original.insert("b".to_string(), 2u32);
        let val = to_value(&original).unwrap();
        let recovered: std::collections::HashMap<String, u32> = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    // ---------------------------------------------------------------
    // Value -> Value deserialization (Deserialize for Value)
    // ---------------------------------------------------------------

    #[test]
    fn value_deserialize_preserves_bool() {
        let val = Value::Bool(false);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn value_deserialize_preserves_text() {
        let val = Value::Text("test".into());
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn value_deserialize_preserves_integer_types() {
        for val in [
            Value::U8(1),
            Value::I8(-1),
            Value::U16(100),
            Value::I16(-100),
            Value::U32(1000),
            Value::I32(-1000),
            Value::U64(10000),
            Value::I64(-10000),
            Value::U128(100000),
            Value::I128(-100000),
        ] {
            let cloned = val.clone();
            let result: Value = from_value(cloned).unwrap();
            assert_eq!(result, val);
        }
    }

    #[test]
    fn value_deserialize_preserves_float() {
        let val = Value::Float(2.718);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn value_deserialize_preserves_null() {
        let val = Value::Null;
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn value_deserialize_preserves_array() {
        let val = Value::Array(vec![Value::U8(1), Value::Text("hello".into())]);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn value_deserialize_preserves_map() {
        let val = Value::Map(vec![(Value::Text("key".into()), Value::U64(42))]);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        assert_eq!(result, val);
    }

    // ---------------------------------------------------------------
    // Bytes deserialization (non-human-readable -> visit_bytes)
    // ---------------------------------------------------------------

    #[test]
    fn value_deserialize_bytes_as_value() {
        let val = Value::Bytes(vec![1, 2, 3]);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn value_deserialize_bytes32_as_value() {
        let val = Value::Bytes32([7u8; 32]);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        // Non-human-readable: Bytes32 goes through visit_bytes which produces Value::Bytes
        assert_eq!(result, Value::Bytes(vec![7u8; 32]));
    }

    #[test]
    fn value_deserialize_bytes20_as_value() {
        let val = Value::Bytes20([8u8; 20]);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        // Non-human-readable: Bytes20 goes through visit_bytes which produces Value::Bytes
        assert_eq!(result, Value::Bytes(vec![8u8; 20]));
    }

    #[test]
    fn value_deserialize_bytes36_as_value() {
        let val = Value::Bytes36([9u8; 36]);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        // Non-human-readable: Bytes36 goes through visit_bytes which produces Value::Bytes
        assert_eq!(result, Value::Bytes(vec![9u8; 36]));
    }

    #[test]
    fn value_deserialize_identifier_as_value() {
        let val = Value::Identifier([10u8; 32]);
        let cloned = val.clone();
        let result: Value = from_value(cloned).unwrap();
        // Non-human-readable: Identifier goes through visit_bytes which produces Value::Bytes
        assert_eq!(result, Value::Bytes(vec![10u8; 32]));
    }

    // ---------------------------------------------------------------
    // Newtype struct deserialization
    // ---------------------------------------------------------------

    #[test]
    fn deserialize_newtype_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Wrapper(u32);

        let val = Value::U32(42);
        let result: Wrapper = from_value(val).unwrap();
        assert_eq!(result, Wrapper(42));
    }

    // ---------------------------------------------------------------
    // Unit struct deserialization
    // ---------------------------------------------------------------

    #[test]
    fn deserialize_unit_struct_from_null() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Empty;

        let val = Value::Null;
        let result: Empty = from_value(val).unwrap();
        assert_eq!(result, Empty);
    }

    // ---------------------------------------------------------------
    // Bytes deserialization into byte buffers
    // ---------------------------------------------------------------

    #[test]
    fn deserialize_bytes_from_bytes_value() {
        let val = Value::Bytes(vec![1, 2, 3, 4]);
        let result: Value = from_value(val.clone()).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn deserialize_bytes_type_mismatch_errors() {
        use serde::de::Deserializer as _;
        struct BytesVisitor;
        impl<'de> de::Visitor<'de> for BytesVisitor {
            type Value = Vec<u8>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "bytes")
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(v.to_vec())
            }
        }
        let deser = Deserializer(Value::Bool(true));
        let result = deser.deserialize_bytes(BytesVisitor);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Enum deserialization
    // ---------------------------------------------------------------

    #[test]
    fn deserialize_unit_enum_from_text() {
        #[derive(Debug, PartialEq, Deserialize)]
        enum Color {
            Red,
            Green,
            Blue,
        }
        let val = Value::Text("Red".into());
        let result: Color = from_value(val).unwrap();
        assert_eq!(result, Color::Red);
    }

    #[test]
    fn deserialize_newtype_enum_from_map() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Wrapper {
            Count(u32),
        }
        let val = Value::Map(vec![(Value::Text("Count".into()), Value::U32(42))]);
        let result: Wrapper = from_value(val).unwrap();
        assert_eq!(result, Wrapper::Count(42));
    }

    #[test]
    fn round_trip_unit_enum() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Direction {
            Up,
            Down,
        }
        let original = Direction::Up;
        let val = to_value(&original).unwrap();
        let recovered: Direction = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_newtype_enum() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Payload {
            Message(String),
        }
        let original = Payload::Message("hello".into());
        let val = to_value(&original).unwrap();
        let recovered: Payload = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_struct_enum() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Shape {
            Circle { radius: u32 },
        }
        let original = Shape::Circle { radius: 5 };
        let val = to_value(&original).unwrap();
        let recovered: Shape = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_tuple_enum() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Pair {
            Coords(i32, i32),
        }
        let original = Pair::Coords(10, 20);
        let val = to_value(&original).unwrap();
        let recovered: Pair = from_value(val).unwrap();
        assert_eq!(original, recovered);
    }
}
