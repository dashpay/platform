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
            _ => Err(de::Error::invalid_type((&self.0).into(), &"enum")),
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct ArrayDeserializer<'a>(slice::Iter<'a, Value>);

impl<'a, 'de> de::SeqAccess<'de> for ArrayDeserializer<'a> {
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

impl<'a, 'de> de::MapAccess<'de> for ValueMapDeserializer<'a> {
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
