use crate::error::Error;
use crate::value_map::ValueMap;
use crate::{to_value, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::ser::Serialize;
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
        next_key: Option<Value>,
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
        // Route keys through the regular Serializer (HR=false) so typed keys
        // — e.g. `Value::Bytes32` for `BTreeMap<ProTxHash, _>` — survive the
        // round-trip. The previous design routed keys through a dedicated
        // string-only `MapKeySerializer` (HR=true), which forced hash-typed
        // keys to be hex strings on serialize while the deserialize side
        // (HR=false) expected bytes — non-round-trippable.
        match self {
            SerializeMap::Map { next_key, .. } => {
                *next_key = Some(tri!(to_value(key)));
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
                map.push((key, tri!(to_value(value))));
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

// `MapKeySerializer` was removed: keys now flow through the regular
// `Serializer` so typed keys (e.g. `Value::Bytes32` for `BTreeMap<ProTxHash, _>`)
// round-trip symmetrically with the deserialize side. The previous
// string-only serializer artificially forced every key to `Value::Text`,
// causing an HR-asymmetry with the deserialize path. The
// `Error::KeyMustBeAString` variant is left in the error enum for SemVer
// stability but is no longer produced by this crate.

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

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // ---------------------------------------------------------------
    // Serialize primitives
    // ---------------------------------------------------------------

    #[test]
    fn serialize_bool() {
        assert_eq!(to_value(true).unwrap(), Value::Bool(true));
        assert_eq!(to_value(false).unwrap(), Value::Bool(false));
    }

    #[test]
    fn serialize_i8() {
        assert_eq!(to_value(-42i8).unwrap(), Value::I8(-42));
    }

    #[test]
    fn serialize_i16() {
        assert_eq!(to_value(-1000i16).unwrap(), Value::I16(-1000));
    }

    #[test]
    fn serialize_i32() {
        assert_eq!(to_value(-100_000i32).unwrap(), Value::I32(-100_000));
    }

    #[test]
    fn serialize_i64() {
        assert_eq!(
            to_value(-10_000_000_000i64).unwrap(),
            Value::I64(-10_000_000_000)
        );
    }

    #[test]
    fn serialize_i128() {
        assert_eq!(to_value(i128::MIN).unwrap(), Value::I128(i128::MIN));
    }

    #[test]
    fn serialize_u8() {
        assert_eq!(to_value(42u8).unwrap(), Value::U8(42));
    }

    #[test]
    fn serialize_u16() {
        assert_eq!(to_value(1000u16).unwrap(), Value::U16(1000));
    }

    #[test]
    fn serialize_u32() {
        assert_eq!(to_value(100_000u32).unwrap(), Value::U32(100_000));
    }

    #[test]
    fn serialize_u64() {
        assert_eq!(
            to_value(10_000_000_000u64).unwrap(),
            Value::U64(10_000_000_000)
        );
    }

    #[test]
    fn serialize_u128() {
        assert_eq!(to_value(u128::MAX).unwrap(), Value::U128(u128::MAX));
    }

    #[test]
    fn serialize_f32() {
        let val = to_value(3.14f32).unwrap();
        match val {
            Value::Float(f) => assert!((f - 3.14f32 as f64).abs() < 1e-6),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn serialize_f64() {
        assert_eq!(to_value(2.718f64).unwrap(), Value::Float(2.718));
    }

    #[test]
    fn serialize_char() {
        let val = to_value('A').unwrap();
        assert_eq!(val, Value::Text("A".to_string()));
    }

    #[test]
    fn serialize_str() {
        assert_eq!(to_value("hello").unwrap(), Value::Text("hello".to_string()));
    }

    #[test]
    fn serialize_string() {
        assert_eq!(
            to_value("world".to_string()).unwrap(),
            Value::Text("world".to_string())
        );
    }

    // ---------------------------------------------------------------
    // Serialize unit / None / Some
    // ---------------------------------------------------------------

    #[test]
    fn serialize_unit() {
        assert_eq!(to_value(()).unwrap(), Value::Null);
    }

    #[test]
    fn serialize_none() {
        let val: Option<u32> = None;
        assert_eq!(to_value(val).unwrap(), Value::Null);
    }

    #[test]
    fn serialize_some() {
        let val: Option<u32> = Some(42);
        assert_eq!(to_value(val).unwrap(), Value::U32(42));
    }

    // ---------------------------------------------------------------
    // Serialize sequences and tuples
    // ---------------------------------------------------------------

    #[test]
    fn serialize_vec() {
        let val = to_value(vec![1u32, 2, 3]).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![Value::U32(1), Value::U32(2), Value::U32(3)])
        );
    }

    #[test]
    fn serialize_empty_vec() {
        let val = to_value(Vec::<u32>::new()).unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    #[test]
    fn serialize_tuple() {
        let val = to_value((1u32, "hello")).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![Value::U32(1), Value::Text("hello".into())])
        );
    }

    // ---------------------------------------------------------------
    // Serialize maps
    // ---------------------------------------------------------------

    #[test]
    fn serialize_hashmap() {
        let mut map = std::collections::HashMap::new();
        map.insert("key", 42u32);
        let val = to_value(map).unwrap();
        match val {
            Value::Map(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].0, Value::Text("key".into()));
                assert_eq!(entries[0].1, Value::U32(42));
            }
            _ => panic!("expected Map"),
        }
    }

    // ---------------------------------------------------------------
    // Serialize structs
    // ---------------------------------------------------------------

    #[test]
    fn serialize_struct() {
        #[derive(Serialize)]
        struct Point {
            x: i32,
            y: i32,
        }
        let val = to_value(Point { x: 10, y: 20 }).unwrap();
        match val {
            Value::Map(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].0, Value::Text("x".into()));
                assert_eq!(entries[0].1, Value::I32(10));
                assert_eq!(entries[1].0, Value::Text("y".into()));
                assert_eq!(entries[1].1, Value::I32(20));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn serialize_unit_struct() {
        #[derive(Serialize)]
        struct Empty;
        assert_eq!(to_value(Empty).unwrap(), Value::Null);
    }

    #[test]
    fn serialize_newtype_struct() {
        #[derive(Serialize)]
        struct Wrapper(u32);
        assert_eq!(to_value(Wrapper(42)).unwrap(), Value::U32(42));
    }

    #[test]
    fn serialize_tuple_struct() {
        #[derive(Serialize)]
        struct Pair(u32, String);
        let val = to_value(Pair(1, "two".into())).unwrap();
        assert_eq!(
            val,
            Value::Array(vec![Value::U32(1), Value::Text("two".into())])
        );
    }

    // ---------------------------------------------------------------
    // Serialize enums
    // ---------------------------------------------------------------

    #[test]
    fn serialize_unit_variant() {
        #[derive(Serialize)]
        enum Color {
            Red,
        }
        assert_eq!(
            to_value(Color::Red).unwrap(),
            Value::Text("Red".to_string())
        );
    }

    #[test]
    fn serialize_newtype_variant() {
        #[derive(Serialize)]
        enum Wrapper {
            Count(u32),
        }
        let val = to_value(Wrapper::Count(42)).unwrap();
        assert_eq!(
            val,
            Value::Map(vec![(Value::Text("Count".into()), Value::U32(42))])
        );
    }

    #[test]
    fn serialize_tuple_variant() {
        #[derive(Serialize)]
        enum Pair {
            Coords(i32, i32),
        }
        let val = to_value(Pair::Coords(10, 20)).unwrap();
        assert_eq!(
            val,
            Value::Map(vec![(
                Value::Text("Coords".into()),
                Value::Array(vec![Value::I32(10), Value::I32(20)])
            )])
        );
    }

    #[test]
    fn serialize_struct_variant() {
        #[derive(Serialize)]
        enum Shape {
            Circle { radius: u32 },
        }
        let val = to_value(Shape::Circle { radius: 5 }).unwrap();
        assert_eq!(
            val,
            Value::Map(vec![(
                Value::Text("Circle".into()),
                Value::Map(vec![(Value::Text("radius".into()), Value::U32(5))])
            )])
        );
    }

    // ---------------------------------------------------------------
    // Serialize bytes (non-human-readable mode)
    // ---------------------------------------------------------------

    #[test]
    fn serialize_bytes_32_becomes_bytes32() {
        // Exactly 32 bytes should become Bytes32
        let data = [1u8; 32];
        use serde::Serializer;
        let val = Serializer.serialize_bytes(&data).unwrap();
        assert_eq!(val, Value::Bytes32(data));
    }

    #[test]
    fn serialize_bytes_20_becomes_bytes20() {
        let data = [2u8; 20];
        use serde::Serializer;
        let val = Serializer.serialize_bytes(&data).unwrap();
        assert_eq!(val, Value::Bytes20(data));
    }

    #[test]
    fn serialize_bytes_36_becomes_bytes36() {
        let data = [3u8; 36];
        use serde::Serializer;
        let val = Serializer.serialize_bytes(&data).unwrap();
        assert_eq!(val, Value::Bytes36(data));
    }

    #[test]
    fn serialize_bytes_other_len_becomes_bytes() {
        let data = vec![4u8; 10];
        use serde::Serializer;
        let val = Serializer.serialize_bytes(&data).unwrap();
        assert_eq!(val, Value::Bytes(data));
    }

    // ---------------------------------------------------------------
    // Serialize Value (the Serialize impl for Value)
    // ---------------------------------------------------------------

    #[test]
    fn serialize_value_null() {
        let val = Value::Null;
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, Value::Null);
    }

    #[test]
    fn serialize_value_bool() {
        let val = Value::Bool(true);
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, Value::Bool(true));
    }

    #[test]
    fn serialize_value_integer_types() {
        let cases = vec![
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
        ];
        for val in cases {
            let serialized = to_value(&val).unwrap();
            assert_eq!(serialized, val);
        }
    }

    #[test]
    fn serialize_value_float() {
        let val = Value::Float(3.14);
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, Value::Float(3.14));
    }

    #[test]
    fn serialize_value_text() {
        let val = Value::Text("hello".into());
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, Value::Text("hello".into()));
    }

    #[test]
    fn serialize_value_array() {
        let val = Value::Array(vec![Value::U8(1), Value::U8(2)]);
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, val);
    }

    #[test]
    fn serialize_value_map() {
        let val = Value::Map(vec![(Value::Text("k".into()), Value::U32(42))]);
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, val);
    }

    #[test]
    fn serialize_value_bytes() {
        let val = Value::Bytes(vec![1, 2, 3, 4, 5]);
        let serialized = to_value(&val).unwrap();
        // Non-human-readable serializer: serialized as bytes
        assert_eq!(serialized, val);
    }

    #[test]
    fn serialize_value_bytes20() {
        let val = Value::Bytes20([1u8; 20]);
        let serialized = to_value(&val).unwrap();
        // serialize_bytes with len=20 -> Bytes20
        assert_eq!(serialized, val);
    }

    #[test]
    fn serialize_value_bytes32() {
        let val = Value::Bytes32([2u8; 32]);
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, val);
    }

    #[test]
    fn serialize_value_bytes36() {
        let val = Value::Bytes36([3u8; 36]);
        let serialized = to_value(&val).unwrap();
        assert_eq!(serialized, val);
    }

    #[test]
    fn serialize_value_identifier() {
        let val = Value::Identifier([4u8; 32]);
        let serialized = to_value(&val).unwrap();
        // Non-human-readable: identifier serialized as bytes (32 bytes -> Bytes32)
        assert_eq!(serialized, Value::Bytes32([4u8; 32]));
    }

    // ---------------------------------------------------------------
    // Map key types — platform_value allows any Value variant as a key.
    // This is unlike serde_json (which mandates string keys for JSON
    // compatibility); platform_value's richer Value type means a
    // `BTreeMap<ProTxHash, _>` round-trips with `Value::Bytes32` keys.
    // ---------------------------------------------------------------

    #[test]
    fn map_key_bool_now_supported() {
        let mut map = std::collections::HashMap::new();
        map.insert(true, "value");
        let result = to_value(map).expect("bool keys are now allowed");
        match result {
            Value::Map(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].0, Value::Bool(true));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn map_key_string_works() {
        let mut map = std::collections::HashMap::new();
        map.insert("key".to_string(), 42u32);
        let result = to_value(map);
        assert!(result.is_ok());
    }

    #[test]
    fn map_key_integer_works() {
        let mut map = std::collections::HashMap::new();
        map.insert(42u32, "value");
        let result = to_value(map);
        assert!(result.is_ok());
    }

    #[test]
    fn map_key_bytes_round_trips_as_bytes32() {
        // The motivating use case: BTreeMap<ProTxHash, _> in dashpay/platform.
        // Hash types serialize via `serialize_bytes` (HR=false). With typed
        // map keys, the result must be `Value::Bytes32`, not a stringified
        // hex form — symmetric with the deserialize side which expects bytes.
        use serde::{ser::SerializeMap as _, Serializer};

        // Drive serialize_bytes directly — the cheapest way to exercise
        // the path without pulling in a full Hash type.
        let mut s = Serializer.serialize_map(Some(1)).unwrap();
        struct BytesKey([u8; 32]);
        impl serde::Serialize for BytesKey {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(&self.0)
            }
        }
        s.serialize_entry(&BytesKey([0xab; 32]), &7u32).unwrap();
        let val = serde::ser::SerializeMap::end(s).unwrap();
        match val {
            Value::Map(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].0, Value::Bytes32([0xab; 32]));
                assert_eq!(entries[0].1, Value::U32(7));
            }
            _ => panic!("expected Value::Map"),
        }
    }

    // ---------------------------------------------------------------
    // Round-trip tests: Rust -> Value -> Rust
    // ---------------------------------------------------------------

    #[test]
    fn round_trip_complex_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Complex {
            name: String,
            count: u64,
            active: bool,
            score: f64,
            tags: Vec<String>,
            metadata: Option<String>,
        }

        let original = Complex {
            name: "test".into(),
            count: 42,
            active: true,
            score: 3.14,
            tags: vec!["a".into(), "b".into()],
            metadata: Some("info".into()),
        };
        let val = to_value(&original).unwrap();
        let recovered: Complex = crate::from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_nested() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Inner {
            x: i32,
        }
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Outer {
            inner: Inner,
        }

        let original = Outer {
            inner: Inner { x: -5 },
        };
        let val = to_value(&original).unwrap();
        let recovered: Outer = crate::from_value(val).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn collect_str_produces_text() {
        use serde::Serializer;
        let val = Serializer.collect_str(&42).unwrap();
        assert_eq!(val, Value::Text("42".to_string()));
    }
}
