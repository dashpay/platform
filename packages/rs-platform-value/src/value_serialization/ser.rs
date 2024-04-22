use crate::error::Error;
use crate::value_map::ValueMap;
use crate::{to_value, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::ser::{Impossible, Serialize};
use std::fmt::Display;

// We only use our own error type; no need for From conversions provided by the
// standard library's try! macro. This reduces lines of LLVM IR by 4%.
macro_rules! tri {
    ($e:expr $(,)?) => {
        match $e {
            core::result::Result::Ok(val) => val,
            core::result::Result::Err(err) => return core::result::Result::Err(err),
        }
    };
}

impl Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Array(v) => v.serialize(serializer),
            Value::Map(m) => {
                use serde::ser::SerializeMap;
                let mut map = tri!(serializer.serialize_map(Some(m.len())));
                for (k, v) in m {
                    tri!(map.serialize_entry(k, v));
                }
                map.end()
            }
            Value::U128(i) => serializer.serialize_u128(*i),
            Value::I128(i) => serializer.serialize_i128(*i),
            Value::U64(i) => serializer.serialize_u64(*i),
            Value::I64(i) => serializer.serialize_i64(*i),
            Value::U32(i) => serializer.serialize_u32(*i),
            Value::I32(i) => serializer.serialize_i32(*i),
            Value::U16(i) => serializer.serialize_u16(*i),
            Value::I16(i) => serializer.serialize_i16(*i),
            Value::U8(i) => serializer.serialize_u8(*i),
            Value::I8(i) => serializer.serialize_i8(*i),
            Value::Bytes(bytes) => {
                if serializer.is_human_readable() {
                    serializer.serialize_str(BASE64_STANDARD.encode(bytes).as_str())
                } else {
                    serializer.serialize_bytes(bytes)
                }
            }
            Value::Bytes20(bytes) => {
                if serializer.is_human_readable() {
                    serializer.serialize_str(BASE64_STANDARD.encode(bytes).as_str())
                } else {
                    serializer.serialize_bytes(bytes)
                }
            }
            Value::Bytes32(bytes) => {
                if serializer.is_human_readable() {
                    serializer.serialize_str(BASE64_STANDARD.encode(bytes).as_str())
                } else {
                    serializer.serialize_bytes(bytes)
                }
            }
            Value::Bytes36(bytes) => {
                if serializer.is_human_readable() {
                    serializer.serialize_str(BASE64_STANDARD.encode(bytes).as_str())
                } else {
                    serializer.serialize_bytes(bytes)
                }
            }
            Value::Identifier(bytes) => {
                if serializer.is_human_readable() {
                    serializer.serialize_str(bs58::encode(bytes).into_string().as_str())
                } else {
                    serializer.serialize_bytes(bytes)
                }
            }
            Value::Float(f64) => serializer.serialize_f64(*f64),
            Value::Text(string) => serializer.serialize_str(string),
            Value::EnumU8(_x) => todo!(),
            Value::EnumString(_x) => todo!(),
        }
    }
}

/// Serializer whose output is a `Value`.
///
/// This is the serializer that backs [`platform_value::to_value`][crate::to_value].
/// Unlike the main platform_value serializer which goes from some serializable
/// value of type `T` to JSON text, this one goes from `T` to
/// `platform_value::Value`.
///
/// The `to_value` function is implementable as:
///
/// ```
/// use serde::Serialize;
/// use serde_json::{Error, Value};
///
/// pub fn to_value<T>(input: T) -> Result<Value, Error>
/// where
///     T: Serialize,
/// {
///     input.serialize(serde_json::value::Serializer)
/// }
/// ```
pub struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Value, Error> {
        Ok(Value::Bool(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Value, Error> {
        Ok(Value::I8(value))
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Value, Error> {
        Ok(Value::I16(value))
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Value, Error> {
        Ok(Value::I32(value))
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Value, Error> {
        Ok(Value::I64(value))
    }

    #[inline]
    fn serialize_i128(self, value: i128) -> Result<Value, Error> {
        Ok(Value::I128(value))
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Value, Error> {
        Ok(Value::U8(value))
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Value, Error> {
        Ok(Value::U16(value))
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Value, Error> {
        Ok(Value::U32(value))
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Value, Error> {
        Ok(Value::U64(value))
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Value, Error> {
        Ok(Value::U128(value))
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Value, Error> {
        self.serialize_f64(value as f64)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Value, Error> {
        Ok(Value::Float(value))
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Value, Error> {
        let mut s = String::new();
        s.push(value);
        Ok(Value::Text(s))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Value, Error> {
        Ok(Value::Text(value.to_owned()))
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Value, Error> {
        Ok(match value.len() {
            32 => Value::Bytes32(value.try_into().unwrap()),
            36 => Value::Bytes36(value.try_into().unwrap()),
            20 => Value::Bytes20(value.try_into().unwrap()),
            _ => Value::Bytes(value.to_vec()),
        })
    }

    #[inline]
    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value, Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        match name {
            "Identifier" => match value.serialize(self)? {
                Value::Bytes32(b) => Ok(Value::Identifier(b)),
                data => {
                    panic!("expected Value::Bytes32, got: {data:#?}")
                }
            },
            _ => value.serialize(self),
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        Ok(Value::Map(vec![(
            Value::Text(String::from(variant)),
            tri!(to_value(value)),
        )]))
    }

    #[inline]
    fn serialize_none(self) -> Result<Value, Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Ok(SerializeTupleVariant {
            name: String::from(variant),
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Ok(SerializeMap::Map {
            map: Vec::new(),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Ok(SerializeStructVariant {
            name: String::from(variant),
            map: Vec::new(),
        })
    }

    fn collect_str<T>(self, value: &T) -> Result<Value, Error>
    where
        T: ?Sized + Display,
    {
        Ok(Value::Text(value.to_string()))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct SerializeVec {
    vec: Vec<Value>,
}

pub struct SerializeTupleVariant {
    name: String,
    vec: Vec<Value>,
}

pub enum SerializeMap {
    Map {
        map: ValueMap,
        next_key: Option<String>,
    },
}

pub struct SerializeStructVariant {
    name: String,
    map: ValueMap,
}

impl serde::ser::SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(tri!(to_value(value)));
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Array(self.vec))
    }
}

impl serde::ser::SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

// impl serde::ser::SerializeTuple for SerializeSizedVec {
//     type Ok = Value;
//     type Error = Error;
//
//     fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
//         where
//             T: ?Sized + Serialize,
//     {
//         serde::ser::SerializeSeq::serialize_element(self, value)
//     }
//
//     fn end(self) -> Result<Value, Error> {
//         if self.size == 32 {
//             Ok(Value::Bytes32(self.vec))
//         } else {
//             serde::ser::SerializeSeq::end(self)
//         }
//     }
// }

impl serde::ser::SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(tri!(to_value(value)));
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(vec![(
            Value::Text(self.name),
            Value::Array(self.vec),
        )]))
    }
}

impl serde::ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            SerializeMap::Map { next_key, .. } => {
                *next_key = Some(tri!(key.serialize(MapKeySerializer)));
                Ok(())
            }
        }
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            SerializeMap::Map { map, next_key } => {
                let key = next_key.take();
                // Panic because this indicates a bug in the program rather than an
                // expected failure.
                let key = key.expect("serialize_value called before serialize_key");
                map.push((Value::Text(key), tri!(to_value(value))));
                Ok(())
            }
        }
    }

    fn end(self) -> Result<Value, Error> {
        match self {
            SerializeMap::Map { map, .. } => Ok(Value::Map(map)),
        }
    }
}

struct MapKeySerializer;

fn key_must_be_a_string() -> Error {
    Error::KeyMustBeAString
}

impl serde::Serializer for MapKeySerializer {
    type Ok = String;
    type Error = Error;

    type SerializeSeq = Impossible<String, Error>;
    type SerializeTuple = Impossible<String, Error>;
    type SerializeTupleStruct = Impossible<String, Error>;
    type SerializeTupleVariant = Impossible<String, Error>;
    type SerializeMap = Impossible<String, Error>;
    type SerializeStruct = Impossible<String, Error>;
    type SerializeStructVariant = Impossible<String, Error>;

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<String, Error> {
        Ok(variant.to_owned())
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<String, Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_bool(self, _value: bool) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_i8(self, value: i8) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_i16(self, value: i16) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_i32(self, value: i32) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_i64(self, value: i64) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_u8(self, value: u8) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_u16(self, value: u16) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_u32(self, value: u32) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_u64(self, value: u64) -> Result<String, Error> {
        Ok(value.to_string())
    }

    fn serialize_f32(self, _value: f32) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_f64(self, _value: f64) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<String, Error> {
        Ok({
            let mut s = String::new();
            s.push(value);
            s
        })
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<String, Error> {
        Ok(value.to_owned())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_unit(self) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String, Error>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }

    fn serialize_none(self) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_some<T>(self, _value: &T) -> Result<String, Error>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(key_must_be_a_string())
    }

    fn collect_str<T>(self, value: &T) -> Result<String, Error>
    where
        T: ?Sized + Display,
    {
        Ok(value.to_string())
    }
}

impl serde::ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            SerializeMap::Map { .. } => serde::ser::SerializeMap::serialize_entry(self, key, value),
        }
    }

    fn end(self) -> Result<Value, Error> {
        match self {
            SerializeMap::Map { .. } => serde::ser::SerializeMap::end(self),
        }
    }
}

impl serde::ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.map
            .push((Value::Text(String::from(key)), tri!(to_value(value))));
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(vec![(
            Value::Text(self.name),
            Value::Map(self.map),
        )]))
    }
}
