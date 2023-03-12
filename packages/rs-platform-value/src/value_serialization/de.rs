use std::iter::Peekable;
use serde::de::{self, Deserializer as _};
use crate::{Error, Value};

impl<'a> From<&'a Value> for de::Unexpected<'a> {
    #[inline]
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Bool(x) => Self::Bool(*x),
            Value::Float(x) => Self::Float(*x),
            Value::Bytes(x) => Self::Bytes(x),
            Value::Text(x) => Self::Str(x),
            Value::Array(..) => Self::Seq,
            Value::Map(..) => Self::Map,
            Value::Null => Self::Other("null"),
        }
    }
}

macro_rules! mkvisit {
    ($($f:ident($v:ty)),+ $(,)?) => {
        $(
            #[inline]
            fn $f<E: de::Error>(self, v: $v) -> Result<Self::Value, E> {
                Ok(v.into())
            }
        )+
    };
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

    #[inline]
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    #[inline]
    fn visit_some<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    #[inline]
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    #[inline]
    fn visit_newtype_struct<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    #[inline]
    fn visit_seq<A: de::SeqAccess<'de>>(self, mut acc: A) -> Result<Self::Value, A::Error> {
        let mut seq = Vec::new();

        while let Some(elem) = acc.next_element()? {
            seq.push(elem);
        }

        Ok(Value::Array(seq))
    }

    #[inline]
    fn visit_map<A: de::MapAccess<'de>>(self, mut acc: A) -> Result<Self::Value, A::Error> {
        let mut map = Vec::<(Value, Value)>::new();

        while let Some(kv) = acc.next_entry()? {
            map.push(kv);
        }

        Ok(Value::Map(map))
    }

    #[inline]
    fn visit_enum<A: de::EnumAccess<'de>>(self, acc: A) -> Result<Self::Value, A::Error> {
        use serde::de::VariantAccess;

        struct Inner;

        impl<'de> serde::de::Visitor<'de> for Inner {
            type Value = Value;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(formatter, "a valid CBOR item")
            }

            #[inline]
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut acc: A) -> Result<Self::Value, A::Error> {
                let tag: u64 = acc
                    .next_element()?
                    .ok_or_else(|| de::Error::custom("expected tag"))?;
                let val = acc
                    .next_element()?
                    .ok_or_else(|| de::Error::custom("expected val"))?;
                Ok(Value::EnumU8(tag, Box::new(val)))
            }
        }

        let (name, data): (String, _) = acc.variant()?;
        assert_eq!("@@TAGGED@@", name);
        data.tuple_variant(2, Inner)
    }
}

impl<'de> de::Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor)
    }
}

struct Deserializer<T>(T);
//
// impl<'a> Deserializer<&'a Value> {
//     fn integer<N>(&self, kind: &'static str) -> Result<N, Error>
//         where
//             N: TryFrom<u128>,
//             N: TryFrom<i128>,
//     {
//         fn raw(value: &Value) -> Result<u128, Error> {
//             let mut buffer = 0u128.to_ne_bytes();
//             let length = buffer.len();
//
//             let bytes = match value {
//                 Value::Bytes(bytes) => {
//                     // Skip leading zeros...
//                     let mut bytes: &[u8] = bytes.as_ref();
//                     while bytes.len() > buffer.len() && bytes[0] == 0 {
//                         bytes = &bytes[1..];
//                     }
//
//                     if bytes.len() > buffer.len() {
//                         return Err(de::Error::custom("bigint too large"));
//                     }
//
//                     bytes
//                 }
//
//                 _ => return Err(de::Error::invalid_type(value.into(), &"bytes")),
//             };
//
//             buffer[length - bytes.len()..].copy_from_slice(bytes);
//             Ok(u128::from_be_bytes(buffer))
//         }
//
//         let err = || de::Error::invalid_type(self.0.into(), &kind);
//
//         Ok(match self {
//             Value::Integer(x) => i128::from(*x).try_into().map_err(|_| err())?,
//             Value::Tag(t, v) if *t == tag::BIGPOS => raw(v)?.try_into().map_err(|_| err())?,
//             Value::Tag(t, v) if *t == tag::BIGNEG => i128::try_from(raw(v)?)
//                 .map(|x| x ^ !0)
//                 .map_err(|_| err())
//                 .and_then(|x| x.try_into().map_err(|_| err()))?,
//             _ => return Err(de::Error::invalid_type(self.0.into(), &"(big)int")),
//         })
//     }
// }

impl<'a, 'de> de::Deserializer<'de> for Deserializer<&'a Value> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Bytes(x) => visitor.visit_bytes(x),
            Value::Text(x) => visitor.visit_str(x),
            Value::Array(x) => visitor.visit_seq(Deserializer(x.iter())),
            Value::Map(x) => visitor.visit_map(Deserializer(x.iter().peekable())),
            Value::Bool(x) => visitor.visit_bool(*x),
            Value::Null => visitor.visit_none(),

            Value::Float(x) => visitor.visit_f64(*x),
            Value::U128(x) => visitor.visit_u128(*x),
            Value::I128(x) => visitor.visit_i128(*x),
            Value::U64(x) => visitor.visit_u64(*x),
            Value::I64(x) => visitor.visit_i64(*x),
            Value::U32(x) => visitor.visit_u32(*x),
            Value::I32(x) => visitor.visit_i32(*x),
            Value::U16(x) => visitor.visit_u16(*x),
            Value::I16(x) => visitor.visit_i16(*x),
            Value::U8(x) => visitor.visit_u8(*x),
            Value::I8(x) => visitor.visit_i8(*x),
            Value::Bytes32(x) => visitor.visit_bytes(x),
            Value::EnumU8(x) => visitor.visit_enum(x),
            Value::EnumString(x) => visitor.visit_enum(x),
            Value::Identifier(x) => visitor.visit_bytes(x),
        }
    }

    #[inline]
    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;

        match value {
            Value::Bool(x) => visitor.visit_bool(*x),
            _ => Err(de::Error::invalid_type(value.into(), &"bool")),
        }
    }

    #[inline]
    fn deserialize_f32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_f64(visitor)
    }

    #[inline]
    fn deserialize_f64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;

        match value {
            Value::Float(x) => visitor.visit_f64(*x),
            _ => Err(de::Error::invalid_type(value.into(), &"f64")),
        }
    }

    fn deserialize_i8<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_i8(value.to_integer()?)
    }

    fn deserialize_i16<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_i16(value.to_integer()?)
    }

    fn deserialize_i32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_i32(value.to_integer()?)
    }

    fn deserialize_i64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_i64(value.to_integer()?)
    }

    fn deserialize_i128<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_i128(value.to_integer()?)
    }

    fn deserialize_u8<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_u8(value.to_integer()?)
    }

    fn deserialize_u16<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_u16(value.to_integer()?)
    }

    fn deserialize_u32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_u32(value.to_integer()?)
    }

    fn deserialize_u64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_u64(value.to_integer()?)
    }

    fn deserialize_u128<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;
        visitor.visit_u128(value.to_integer()?)
    }

    fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;

        match value {
            Value::Text(x) => match x.chars().count() {
                1 => visitor.visit_char(x.chars().next().unwrap()),
                _ => Err(de::Error::invalid_type(value.into(), &"char")),
            },

            _ => Err(de::Error::invalid_type(value.into(), &"char")),
        }
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;

        match value {
            Value::Text(x) => visitor.visit_str(x),
            _ => Err(de::Error::invalid_type(value.into(), &"str")),
        }
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;

        match value {
            Value::Bytes(x) => visitor.visit_bytes(x),
            _ => Err(de::Error::invalid_type(value.into(), &"bytes")),
        }
    }

    fn deserialize_byte_buf<V: de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;

        match value {
            Value::Array(x) => visitor.visit_seq(Deserializer(x.iter())),
            _ => Err(de::Error::invalid_type(value.into(), &"array")),
        }
    }

    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut value = self.0;

        match value {
            Value::Map(x) => visitor.visit_map(Deserializer(x.iter().peekable())),
            _ => Err(de::Error::invalid_type(value.into(), &"map")),
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

    #[inline]
    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Null => visitor.visit_none(),
            x => visitor.visit_some(Self(x)),
        }
    }

    #[inline]
    fn deserialize_unit<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            _ => Err(de::Error::invalid_type(self.0.into(), &"null")),
        }
    }

    #[inline]
    fn deserialize_unit_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Map(x) if x.len() == 1 => visitor.visit_enum(Deserializer(&x[0])),
            x @ Value::Text(..) => visitor.visit_enum(Deserializer(x)),
            _ => Err(de::Error::invalid_type(self.0.into(), &"map")),
        }
    }
}

impl<'a, 'de, T: Iterator<Item = &'a Value>> de::SeqAccess<'de> for Deserializer<T> {
    type Error = Error;

    #[inline]
    fn next_element_seed<U: de::DeserializeSeed<'de>>(
        &mut self,
        seed: U,
    ) -> Result<Option<U::Value>, Self::Error> {
        match self.0.next() {
            None => Ok(None),
            Some(v) => seed.deserialize(Deserializer(v)).map(Some),
        }
    }
}

impl<'a, 'de, T: Iterator<Item = &'a (Value, Value)>> de::MapAccess<'de>
for Deserializer<Peekable<T>>
{
    type Error = Error;

    #[inline]
    fn next_key_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.0.peek() {
            None => Ok(None),
            Some(x) => Ok(Some(seed.deserialize(Deserializer(&x.0))?)),
        }
    }

    #[inline]
    fn next_value_seed<V: de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        seed.deserialize(Deserializer(&self.0.next().unwrap().1))
    }
}

impl<'a, 'de> de::EnumAccess<'de> for Deserializer<&'a (Value, Value)> {
    type Error = Error;
    type Variant = Deserializer<&'a Value>;

    #[inline]
    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let k = seed.deserialize(Deserializer(&self.0 .0))?;
        Ok((k, Deserializer(&self.0 .1)))
    }
}

impl<'a, 'de> de::EnumAccess<'de> for Deserializer<&'a Value> {
    type Error = Error;
    type Variant = Deserializer<&'a Value>;

    #[inline]
    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let k = seed.deserialize(self)?;
        Ok((k, Deserializer(&Value::Null)))
    }
}

impl<'a, 'de> de::VariantAccess<'de> for Deserializer<&'a Value> {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.0 {
            Value::Null => Ok(()),
            _ => Err(de::Error::invalid_type(self.0.into(), &"unit")),
        }
    }

    #[inline]
    fn newtype_variant_seed<U: de::DeserializeSeed<'de>>(
        self,
        seed: U,
    ) -> Result<U::Value, Self::Error> {
        seed.deserialize(self)
    }

    #[inline]
    fn tuple_variant<V: de::Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }
}

impl Value {
    /// Deserializes the `Value` into an object
    #[inline]
    pub fn deserialized<'de, T: de::Deserialize<'de>>(&self) -> Result<T, Error> {
        T::deserialize(Deserializer(self))
    }
}
