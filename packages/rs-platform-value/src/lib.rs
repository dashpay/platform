//! Platform value
//! A dynamic Platform value
//!
//! A module that is used to represent values in Platform components
//! Forked from ciborium value
//!
//!
extern crate core;

pub mod btreemap_extensions;
pub mod converter;
pub mod display;
mod eq;
mod error;
mod index;
mod inner_array_value;
pub mod inner_value;
mod inner_value_at_path;
mod macros;
pub mod patch;
mod pointer;
mod replace;
pub mod string_encoding;
pub mod system_bytes;
mod types;
mod value_map;
mod value_serialization;

pub use crate::value_map::{ValueMap, ValueMapHelper};
pub use error::Error;
use std::collections::BTreeMap;

pub type Hash256 = [u8; 32];

pub use btreemap_extensions::btreemap_field_replacement::{
    IntegerReplacementType, ReplacementType,
};
pub use types::binary_data::BinaryData;
pub use types::bytes_20::Bytes20;
pub use types::bytes_32::Bytes32;
pub use types::bytes_36::Bytes36;
pub use types::identifier::{Identifier, IdentifierBytes32, IDENTIFIER_MEDIA_TYPE};

pub use value_serialization::{from_value, to_value};

use bincode::{Decode, Encode};
pub use patch::{patch, Patch};

/// A representation of a dynamic value that can handled dynamically
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, PartialOrd, Encode, Decode)]
pub enum Value {
    /// A u128 integer
    U128(u128),

    /// A i128 integer
    I128(i128),

    /// A u64 integer
    U64(u64),

    /// A i64 integer
    I64(i64),

    /// A u32 integer
    U32(u32),

    /// A i32 integer
    I32(i32),

    /// A u16 integer
    U16(u16),

    /// A i16 integer
    I16(i16),

    /// A u8 integer
    U8(u8),

    /// A i8 integer
    I8(i8),

    /// Bytes
    Bytes(Vec<u8>),

    /// Bytes 20
    Bytes20([u8; 20]),

    /// Bytes 32
    Bytes32([u8; 32]),

    /// Bytes 36 : Useful for outpoints
    Bytes36([u8; 36]),

    /// An enumeration of u8
    EnumU8(Vec<u8>),

    /// An enumeration of strings
    EnumString(Vec<String>),

    /// Identifier
    /// The identifier is very similar to bytes, however it is serialized to Base58 when converted
    /// to a JSON Value
    Identifier(Hash256),

    /// A float
    Float(f64),

    /// A string
    Text(String),

    /// A boolean
    Bool(bool),

    /// Null
    Null,

    /// An array
    Array(Vec<Value>),

    /// A map
    Map(ValueMap),
}

impl Value {
    /// Returns true if the `Value` is an `Integer`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::U64(17);
    ///
    /// assert!(value.is_integer());
    /// ```
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Value::U128(_)
                | Value::I128(_)
                | Value::U64(_)
                | Value::I64(_)
                | Value::U32(_)
                | Value::I32(_)
                | Value::U16(_)
                | Value::I16(_)
                | Value::U8(_)
                | Value::I8(_)
        )
    }

    /// Returns true if the `Value` is an integer that fits in 64 bits (u64/i64).
    /// Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::U128(17);
    ///
    /// assert!(value.is_integer_can_fit_64_bytes());
    /// ```
    pub fn is_integer_can_fit_64_bytes(&self) -> bool {
        match self {
            // Already ≤ 64-bit widths
            Value::U64(_)
            | Value::I64(_)
            | Value::U32(_)
            | Value::I32(_)
            | Value::U16(_)
            | Value::I16(_)
            | Value::U8(_)
            | Value::I8(_) => true,

            // 128-bit -> check if within 64-bit range
            Value::U128(v) => *v <= u64::MAX as u128,
            Value::I128(v) => (*v >= i64::MIN as i128) && (*v <= i64::MAX as i128),

            // Non-integer variants
            _ => false,
        }
    }

    /// If the `Value` is a `Integer`, returns a reference to the associated `Integer` data.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::U64(17);
    ///
    /// // We can read the number
    /// let r_value : u64 = value.as_integer().unwrap();
    /// assert_eq!(17, r_value);
    /// ```
    pub fn as_integer<T>(&self) -> Option<T>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        match self {
            Value::U128(int) => (*int).try_into().ok(),
            Value::I128(int) => (*int).try_into().ok(),
            Value::U64(int) => (*int).try_into().ok(),
            Value::I64(int) => (*int).try_into().ok(),
            Value::U32(int) => (*int).try_into().ok(),
            Value::I32(int) => (*int).try_into().ok(),
            Value::U16(int) => (*int).try_into().ok(),
            Value::I16(int) => (*int).try_into().ok(),
            Value::U8(int) => (*int).try_into().ok(),
            Value::I8(int) => (*int).try_into().ok(),
            _ => None,
        }
    }

    /// If the `Value` is a `Integer`, returns a the associated `Integer` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let value = Value::U64(17);
    /// let r_value : Result<u64,Error> = value.into_integer();
    /// assert_eq!(r_value, Ok(17));
    ///
    /// let value = Value::Bool(true);
    /// let r_value : Result<u64,Error> = value.into_integer();
    /// assert_eq!(r_value, Err(Error::StructureError("value is not an integer".to_string())));
    /// ```
    pub fn into_integer<T>(self) -> Result<T, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        match self {
            Value::U128(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::I128(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::U64(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::I64(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::U32(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::I32(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::U16(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::I16(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::U8(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            Value::I8(int) => int.try_into().map_err(|_| Error::IntegerSizeError),
            _other => Err(Error::StructureError("value is not an integer".to_string())),
        }
    }

    /// If the `Value` is a `Integer`, returns a the associated `Integer` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let value = Value::U64(17);
    /// let r_value : Result<u64,Error> = value.to_integer();
    /// assert_eq!(r_value, Ok(17));
    ///
    /// let value = Value::Bool(true);
    /// let r_value : Result<u64,Error> = value.to_integer();
    /// assert_eq!(r_value, Err(Error::StructureError("value is not an integer, found bool true".to_string())));
    /// ```
    pub fn to_integer<T>(&self) -> Result<T, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        match self {
            Value::U128(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I128(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U64(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I64(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U32(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I32(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U16(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I16(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U8(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I8(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            other => Err(Error::StructureError(format!(
                "value is not an integer, found {}",
                other
            ))),
        }
    }

    /// If the `Value` is an `Integer`, a `String` or a `Float` or even a `Bool`, returns the
    /// associated `Integer` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let value = Value::U64(17);
    /// let r_value : Result<u64,Error> = value.to_integer_broad_conversion();
    /// assert_eq!(r_value, Ok(17));
    ///
    /// let value = Value::Text("17".to_string());
    /// let r_value : Result<u64,Error> = value.to_integer_broad_conversion();
    /// assert_eq!(r_value, Ok(17));
    ///
    /// let value = Value::Bool(true);
    /// let r_value : Result<u64,Error> = value.to_integer_broad_conversion();
    /// assert_eq!(r_value, Ok(1));
    /// ```
    pub fn to_integer_broad_conversion<T>(&self) -> Result<T, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        match self {
            Value::U128(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I128(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U64(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I64(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U32(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I32(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U16(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I16(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::U8(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::I8(int) => (*int).try_into().map_err(|_| Error::IntegerSizeError),
            Value::Float(float) => {
                let max_f64 = u128::MAX as f64;
                let min_f64 = i128::MIN as f64;
                if *float > 0f64 && *float < max_f64 {
                    (*float as u128)
                        .try_into()
                        .map_err(|_| Error::IntegerSizeError)
                } else if *float > min_f64 && *float < 0f64 {
                    (*float as i128)
                        .try_into()
                        .map_err(|_| Error::IntegerSizeError)
                } else {
                    Err(Error::IntegerSizeError)
                }
            }
            Value::Bool(bool) => {
                let i: u8 = (*bool).into();
                i.try_into().map_err(|_| Error::IntegerSizeError)
            }
            Value::Text(text) => text
                .parse::<i128>()
                .map_err(|_| Error::IntegerSizeError)?
                .try_into()
                .map_err(|_| Error::IntegerSizeError),
            other => Err(Error::StructureError(format!(
                "value can not be converted to an integer, found {}",
                other
            ))),
        }
    }

    /// Returns true if the `Value` is a `Bytes`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    ///
    /// assert!(value.is_bytes());
    /// ```
    pub fn is_bytes(&self) -> bool {
        self.as_bytes().is_some()
    }

    /// Returns true if the `Value` is a `Bytes`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    ///
    /// assert!(value.is_any_bytes_type());
    ///
    /// let value = Value::Identifier([1u8;32]);
    ///
    /// assert!(value.is_any_bytes_type());
    ///
    /// let value = Value::Bytes20([1u8;20]);
    ///
    /// assert!(value.is_any_bytes_type());
    ///
    /// let value = Value::Bytes32([1u8;32]);
    ///
    /// assert!(value.is_any_bytes_type());
    ///
    /// let value = Value::Bytes36([1u8;36]);
    ///
    /// assert!(value.is_any_bytes_type());
    /// ```
    pub fn is_any_bytes_type(&self) -> bool {
        matches!(
            self,
            Value::Bytes(_)
                | Value::Bytes20(_)
                | Value::Bytes32(_)
                | Value::Bytes36(_)
                | Value::Identifier(_)
        )
    }

    /// If the `Value` is a `Bytes`, returns a reference to the associated bytes vector.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    ///
    /// assert_eq!(std::str::from_utf8(value.as_bytes().unwrap()).unwrap(), "hello");
    /// ```
    pub fn as_bytes(&self) -> Option<&Vec<u8>> {
        match *self {
            Value::Bytes(ref bytes) => Some(bytes),
            _ => None,
        }
    }

    /// If the `Value` is a `Bytes`, returns a mutable reference to the associated bytes vector.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let mut value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// value.as_bytes_mut().unwrap().clear();
    ///
    /// assert_eq!(value, Value::Bytes(vec![]));
    /// ```
    pub fn as_bytes_mut(&mut self) -> Option<&mut Vec<u8>> {
        match *self {
            Value::Bytes(ref mut bytes) => Some(bytes),
            _ => None,
        }
    }

    /// If the `Value` is a `Bytes`, returns a the associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.into_bytes(), Ok(vec![104, 101, 108, 108, 111]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_bytes(), Err(Error::StructureError("value are not bytes".to_string())));
    /// ```
    pub fn into_bytes(self) -> Result<Vec<u8>, Error> {
        match self {
            Value::Bytes(vec) => Ok(vec),
            Value::Bytes20(vec) => Ok(vec.to_vec()),
            Value::Bytes32(vec) => Ok(vec.to_vec()),
            Value::Bytes36(vec) => Ok(vec.to_vec()),
            Value::Identifier(vec) => Ok(vec.to_vec()),
            Value::Array(array) => Ok(array
                .into_iter()
                .map(|byte| byte.into_integer())
                .collect::<Result<Vec<u8>, Error>>()?),
            _other => Err(Error::StructureError("value are not bytes".to_string())),
        }
    }

    /// If the `Value` is a ref to `Bytes`, returns a the associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.to_bytes(), Ok(vec![104, 101, 108, 108, 111]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_bytes(), Err(Error::StructureError("ref value are not bytes found bool true instead".to_string())));
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        match self {
            Value::Bytes(vec) => Ok(vec.clone()),
            Value::Bytes20(vec) => Ok(vec.to_vec()),
            Value::Bytes32(vec) => Ok(vec.to_vec()),
            Value::Bytes36(vec) => Ok(vec.to_vec()),
            Value::Identifier(vec) => Ok(vec.to_vec()),
            Value::Array(array) => Ok(array
                .iter()
                .map(|byte| byte.to_integer())
                .collect::<Result<Vec<u8>, Error>>()?),
            other => Err(Error::StructureError(format!(
                "ref value are not bytes found {} instead",
                other
            ))),
        }
    }

    /// If the `Value` is a ref to `Bytes`, returns a the associated `BinaryData` data as `Ok`.
    /// BinaryData wraps Vec<u8>
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{BinaryData, Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.to_binary_data(), Ok(BinaryData::new(vec![104, 101, 108, 108, 111])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_binary_data(), Err(Error::StructureError("ref value are not bytes found bool true instead".to_string())));
    /// ```
    pub fn to_binary_data(&self) -> Result<BinaryData, Error> {
        match self {
            Value::Bytes(vec) => Ok(BinaryData::new(vec.clone())),
            Value::Bytes20(vec) => Ok(BinaryData::new(vec.to_vec())),
            Value::Bytes32(vec) => Ok(BinaryData::new(vec.to_vec())),
            Value::Bytes36(vec) => Ok(BinaryData::new(vec.to_vec())),
            Value::Identifier(vec) => Ok(BinaryData::new(vec.to_vec())),
            Value::Array(array) => Ok(BinaryData::new(
                array
                    .iter()
                    .map(|byte| byte.to_integer())
                    .collect::<Result<Vec<u8>, Error>>()?,
            )),
            other => Err(Error::StructureError(format!(
                "ref value are not bytes found {} instead",
                other
            ))),
        }
    }

    /// If the `Value` is a ref to `Bytes`, returns a the associated `&[u8]` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.as_bytes_slice(), Ok(vec![104, 101, 108, 108, 111].as_slice()));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.as_bytes_slice(), Err(Error::StructureError("ref value are not bytes slice".to_string())));
    /// ```
    pub fn as_bytes_slice(&self) -> Result<&[u8], Error> {
        match self {
            Value::Bytes(vec) => Ok(vec),
            Value::Bytes20(vec) => Ok(vec.as_slice()),
            Value::Bytes32(vec) => Ok(vec.as_slice()),
            Value::Bytes36(vec) => Ok(vec.as_slice()),
            Value::Identifier(vec) => Ok(vec.as_slice()),
            _other => Err(Error::StructureError(
                "ref value are not bytes slice".to_string(),
            )),
        }
    }

    /// Returns true if the `Value` is a `Float`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Float(17.0.into());
    ///
    /// assert!(value.is_float());
    /// ```
    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    /// If the `Value` is a `Float`, returns a reference to the associated float data.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Float(17.0.into());
    ///
    /// // We can read the float number
    /// assert_eq!(value.as_float().unwrap(), 17.0_f64);
    /// ```
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Value::U128(int) => Some(int as f64),
            Value::I128(int) => Some(int as f64),
            Value::U64(int) => Some(int as f64),
            Value::I64(int) => Some(int as f64),
            Value::U32(int) => Some(int as f64),
            Value::I32(int) => Some(int as f64),
            Value::U16(int) => Some(int as f64),
            Value::I16(int) => Some(int as f64),
            Value::U8(int) => Some(int as f64),
            Value::I8(int) => Some(int as f64),
            Value::Float(f) => Some(f),
            _ => None,
        }
    }

    /// If the `Value` is a `Float`, returns a the associated `f64` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Float(17.);
    /// assert_eq!(value.into_float(), Ok(17.));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_float(), Err(Error::StructureError("value is not a float".to_string())));
    /// ```
    pub fn into_float(self) -> Result<f64, Error> {
        match self {
            Value::U128(int) => Ok(int as f64),
            Value::I128(int) => Ok(int as f64),
            Value::U64(int) => Ok(int as f64),
            Value::I64(int) => Ok(int as f64),
            Value::U32(int) => Ok(int as f64),
            Value::I32(int) => Ok(int as f64),
            Value::U16(int) => Ok(int as f64),
            Value::I16(int) => Ok(int as f64),
            Value::U8(int) => Ok(int as f64),
            Value::I8(int) => Ok(int as f64),
            Value::Float(f) => Ok(f),
            _other => Err(Error::StructureError("value is not a float".to_string())),
        }
    }

    /// If the `Value` is a `Float`, returns a the associated `f64` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Float(17.);
    /// assert_eq!(value.to_float(), Ok(17.));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_float(), Err(Error::StructureError("value is not a float".to_string())));
    /// ```
    pub fn to_float(&self) -> Result<f64, Error> {
        match self {
            Value::U128(int) => Ok(*int as f64),
            Value::I128(int) => Ok(*int as f64),
            Value::U64(int) => Ok(*int as f64),
            Value::I64(int) => Ok(*int as f64),
            Value::U32(int) => Ok(*int as f64),
            Value::I32(int) => Ok(*int as f64),
            Value::U16(int) => Ok(*int as f64),
            Value::I16(int) => Ok(*int as f64),
            Value::U8(int) => Ok(*int as f64),
            Value::I8(int) => Ok(*int as f64),
            Value::Float(f) => Ok(*f),
            _other => Err(Error::StructureError("value is not a float".to_string())),
        }
    }

    /// Returns true if the `Value` is a `Text`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Text(String::from("hello"));
    ///
    /// assert!(value.is_text());
    /// ```
    pub fn is_text(&self) -> bool {
        self.as_text().is_some()
    }

    /// If the `Value` is a `Text`, returns a reference to the associated `String` data.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Text(String::from("hello"));
    ///
    /// // We can read the String
    /// assert_eq!(value.as_text().unwrap(), "hello");
    /// ```
    pub fn as_text(&self) -> Option<&str> {
        match *self {
            Value::Text(ref s) => Some(s),
            _ => None,
        }
    }

    /// If the `Value` is a `Text`, returns a mutable reference to the associated `String` data.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let mut value = Value::Text(String::from("hello"));
    /// value.as_text_mut().unwrap().clear();
    ///
    /// assert_eq!(value.as_text().unwrap(), &String::from(""));
    /// ```
    pub fn as_text_mut(&mut self) -> Option<&mut String> {
        match *self {
            Value::Text(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// If the `Value` is a `String`, returns a the associated `String` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Text(String::from("hello"));
    /// assert_eq!(value.into_text().as_deref(), Ok("hello"));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_text(), Err(Error::StructureError("value is not a string".to_string())));
    /// ```
    pub fn into_text(self) -> Result<String, Error> {
        match self {
            Value::Text(s) => Ok(s),
            _other => Err(Error::StructureError("value is not a string".to_string())),
        }
    }

    /// If the `Value` is a `String`, returns a the associated `String` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Text(String::from("hello"));
    /// assert_eq!(value.to_text().as_deref(), Ok("hello"));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_text(), Err(Error::StructureError("value is not a string".to_string())));
    /// ```
    pub fn to_text(&self) -> Result<String, Error> {
        match self {
            Value::Text(s) => Ok(s.clone()),
            _other => Err(Error::StructureError("value is not a string".to_string())),
        }
    }

    /// If the `Value` is a `String`, returns a the associated `&str` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Text(String::from("hello"));
    /// assert_eq!(value.to_str(), Ok("hello"));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_str(), Err(Error::StructureError("value is not a string".to_string())));
    /// ```
    pub fn to_str(&self) -> Result<&str, Error> {
        match self {
            Value::Text(s) => Ok(s),
            _other => Err(Error::StructureError("value is not a string".to_string())),
        }
    }

    /// If the `Value` is a `String`, returns a reference to the associated `String` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Text(String::from("hello"));
    /// assert_eq!(value.as_str(), Some("hello"));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a `Bool`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Bool(false);
    ///
    /// assert!(value.is_bool());
    /// ```
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// If the `Value` is a `Bool`, returns a copy of the associated boolean value. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Bool(false);
    ///
    /// assert_eq!(value.as_bool().unwrap(), false);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// If the `Value` is a `Bool`, returns a the associated `bool` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bool(false);
    /// assert_eq!(value.into_bool(), Ok(false));
    ///
    /// let value = Value::Float(17.);
    /// assert_eq!(value.into_bool(), Err(Error::StructureError("value is not a bool".to_string())));
    /// ```
    pub fn into_bool(self) -> Result<bool, Error> {
        match self {
            Value::Bool(b) => Ok(b),
            _other => Err(Error::StructureError("value is not a bool".to_string())),
        }
    }

    /// If the `Value` is a `Bool`, returns a the associated `bool` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bool(false);
    /// assert_eq!(value.to_bool(), Ok(false));
    ///
    /// let value = Value::Float(17.);
    /// assert_eq!(value.to_bool(), Err(Error::StructureError("value is not a bool".to_string())));
    /// ```
    pub fn to_bool(&self) -> Result<bool, Error> {
        match self {
            Value::Bool(b) => Ok(*b),
            _other => Err(Error::StructureError("value is not a bool".to_string())),
        }
    }

    /// Returns true if the `Value` is a `Null`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Null;
    ///
    /// assert!(value.is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns true if the `Value` is an Array. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Array(
    ///     vec![
    ///         Value::Text(String::from("foo")),
    ///         Value::Text(String::from("bar"))
    ///     ]
    /// );
    ///
    /// assert!(value.is_array());
    /// ```
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// If the `Value` is an Array, returns a reference to the associated vector. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Array(
    ///     vec![
    ///         Value::Text(String::from("foo")),
    ///         Value::Text(String::from("bar"))
    ///     ]
    /// );
    ///
    /// // The length of `value` is 2 elements.
    /// assert_eq!(value.as_array().unwrap().len(), 2);
    /// ```
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match *self {
            Value::Array(ref array) => Some(array),
            _ => None,
        }
    }

    /// If the `Value` is an Array, returns a mutable reference to the associated vector.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::Text(String::from("foo")),
    ///         Value::Text(String::from("bar"))
    ///     ]
    /// );
    ///
    /// value.as_array_mut().unwrap().clear();
    /// assert_eq!(value, Value::Array(vec![]));
    /// ```
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match *self {
            Value::Array(ref mut list) => Some(list),
            _ => None,
        }
    }

    /// If the `Value` is an Array, returns a mutable reference to the associated vector.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::Text(String::from("foo")),
    ///         Value::Text(String::from("bar"))
    ///     ]
    /// );
    ///
    /// value.to_array_mut().unwrap().clear();
    /// assert_eq!(value, Value::Array(vec![]));
    /// ```
    pub fn to_array_mut(&mut self) -> Result<&mut Vec<Value>, Error> {
        match self {
            Value::Array(vec) => Ok(vec),
            other => Err(Error::StructureError(format!(
                "value is not a mut array got {}",
                other
            ))),
        }
    }

    /// If the `Value` is a `Array`, returns a the associated `&[Value]` slice as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::U64(17),
    ///         Value::Float(18.),
    ///     ]
    /// );
    /// assert_eq!(value.to_array_slice(), Ok(vec![Value::U64(17), Value::Float(18.)].as_slice()));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_array_slice(), Err(Error::StructureError("value is not an array got bool true".to_string())));
    /// ```
    pub fn to_array_slice(&self) -> Result<&[Value], Error> {
        match self {
            Value::Array(vec) => Ok(vec.as_slice()),
            other => Err(Error::StructureError(format!(
                "value is not an array got {}",
                other
            ))),
        }
    }

    /// If the `Value` is a `Array`, returns a the associated `Vec<&Value>` array as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::U64(17),
    ///         Value::Float(18.),
    ///     ]
    /// );
    /// assert_eq!(value.to_array_ref(), Ok(&vec![Value::U64(17), Value::Float(18.)]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_array_ref(), Err(Error::StructureError("value is not an array got bool true".to_string())));
    /// ```
    pub fn to_array_ref(&self) -> Result<&Vec<Value>, Error> {
        match self {
            Value::Array(vec) => Ok(vec),
            other => Err(Error::StructureError(format!(
                "value is not an array got {}",
                other
            ))),
        }
    }

    /// If the `Value` is a `Array`, returns a the associated `Vec<Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::U64(17),
    ///         Value::Float(18.),
    ///     ]
    /// );
    /// assert_eq!(value.to_array_owned(), Ok(vec![Value::U64(17), Value::Float(18.)]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_array_owned(), Err(Error::StructureError("value is not an owned array got bool true".to_string())));
    /// ```
    pub fn to_array_owned(&self) -> Result<Vec<Value>, Error> {
        match self {
            Value::Array(vec) => Ok(vec.clone()),
            other => Err(Error::StructureError(format!(
                "value is not an owned array got {}",
                other
            ))),
        }
    }

    /// If the `Value` is a `Array`, returns a the associated `Vec<Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::U64(17),
    ///         Value::Float(18.),
    ///     ]
    /// );
    /// assert_eq!(value.into_array(), Ok(vec![Value::U64(17), Value::Float(18.)]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_array(), Err(Error::StructureError("value is not an array (into) got bool true".to_string())));
    /// ```
    pub fn into_array(self) -> Result<Vec<Value>, Error> {
        match self {
            Value::Array(vec) => Ok(vec),
            other => Err(Error::StructureError(format!(
                "value is not an array (into) got {}",
                other
            ))),
        }
    }

    /// If the `Value` is a `Array`, returns a the associated `Vec<Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Error};
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::U64(17),
    ///         Value::Float(18.),
    ///     ]
    /// );
    /// assert_eq!(value.as_slice(), Ok(vec![Value::U64(17), Value::Float(18.)].as_slice()));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.as_slice(), Err(Error::StructureError("value is not a slice got bool true".to_string())));
    /// ```
    pub fn as_slice(&self) -> Result<&[Value], Error> {
        match self {
            Value::Array(vec) => Ok(vec),
            other => Err(Error::StructureError(format!(
                "value is not a slice got {}",
                other
            ))),
        }
    }

    /// Returns true if the `Value` is a Map. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foo")), Value::Text(String::from("bar")))
    ///     ]
    /// );
    ///
    /// assert!(value.is_map());
    /// ```
    pub fn is_map(&self) -> bool {
        self.as_map().is_some()
    }

    /// If the `Value` is a Map, returns a reference to the associated Map data. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foo")), Value::Text(String::from("bar")))
    ///     ]
    /// );
    ///
    /// // The length of data is 1 entry (1 key/value pair).
    /// assert_eq!(value.as_map().unwrap().len(), 1);
    ///
    /// // The content of the first element is what we expect
    /// assert_eq!(
    ///     value.as_map().unwrap().get(0).unwrap(),
    ///     &(Value::Text(String::from("foo")), Value::Text(String::from("bar")))
    /// );
    /// ```
    pub fn as_map(&self) -> Option<&Vec<(Value, Value)>> {
        match *self {
            Value::Map(ref map) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is a Map, returns a mutable reference to the associated Map Data.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foo")), Value::Text(String::from("bar")))
    ///     ]
    /// );
    ///
    /// value.as_map_mut().unwrap().clear();
    /// assert_eq!(value, Value::Map(vec![]));
    /// assert_eq!(value.as_map().unwrap().len(), 0);
    /// ```
    pub fn as_map_mut(&mut self) -> Option<&mut Vec<(Value, Value)>> {
        match *self {
            Value::Map(ref mut map) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is a Map, returns a mutable reference to the associated Map Data.
    /// Returns Err otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foo")), Value::Text(String::from("bar")))
    ///     ]
    /// );
    ///
    /// value.to_map_mut().unwrap().clear();
    /// assert_eq!(value, Value::Map(vec![]));
    /// assert_eq!(value.as_map().unwrap().len(), 0);
    /// ```
    pub fn to_map_mut(&mut self) -> Result<&mut ValueMap, Error> {
        match *self {
            Value::Map(ref mut map) => Ok(map),
            _ => Err(Error::StructureError("value is not a map".to_string())),
        }
    }

    /// If the `Value` is a `Map`, returns a the associated ValueMap which is a `Vec<(Value, Value)>`
    /// data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.into_map(), Ok(vec![(Value::Text(String::from("key")), Value::Float(18.))]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_map(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn into_map(self) -> Result<ValueMap, Error> {
        match self {
            Value::Map(map) => Ok(map),
            _other => Err(Error::StructureError("value is not a map".to_string())),
        }
    }

    /// If the `Value` is a `Map`, returns a the associated ValueMap which is a `Vec<(Value, Value)>`
    /// data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.to_map(), Ok(&vec![(Value::Text(String::from("key")), Value::Float(18.))]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_map(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn to_map(&self) -> Result<&ValueMap, Error> {
        match self {
            Value::Map(map) => Ok(map),
            _other => Err(Error::StructureError("value is not a map".to_string())),
        }
    }

    /// If the `Value` is a `Map`, returns the associated ValueMap ref which is a `&Vec<(Value, Value)>`
    /// data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.to_map_ref(), Ok(&vec![(Value::Text(String::from("key")), Value::Float(18.))]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_map_ref(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn to_map_ref(&self) -> Result<&ValueMap, Error> {
        match self {
            Value::Map(map) => Ok(map),
            _other => Err(Error::StructureError("value is not a map".to_string())),
        }
    }

    /// If the `Value` is a `Map`, returns the associated ValueMap ref which is a `&Vec<(Value, Value)>`
    /// data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.as_map_mut_ref(), Ok(&mut vec![(Value::Text(String::from("key")), Value::Float(18.))]));
    ///
    /// let mut value = Value::Bool(true);
    /// assert_eq!(value.as_map_mut_ref(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn as_map_mut_ref(&mut self) -> Result<&mut ValueMap, Error> {
        match self {
            Value::Map(map) => Ok(map),
            _other => Err(Error::StructureError("value is not a map".to_string())),
        }
    }

    /// Returns the numeric value as `i128` if this is any signed /
    /// unsigned integer variant **and** the conversion is loss-less.
    #[inline]
    fn as_i128_unified(&self) -> Option<i128> {
        use Value::*;
        match self {
            I128(v) => Some(*v),
            I64(v) => Some(*v as i128),
            I32(v) => Some(*v as i128),
            I16(v) => Some(*v as i128),
            I8(v) => Some(*v as i128),

            U128(v) if *v <= i128::MAX as u128 => Some(*v as i128),
            U64(v) => Some(*v as i128),
            U32(v) => Some(*v as i128),
            U16(v) => Some(*v as i128),
            U8(v) => Some(*v as i128),

            _ => None,
        }
    }

    /// can determine if there is any very big data in a value
    pub fn has_data_larger_than(&self, size: u32) -> Option<(Option<Value>, u32)> {
        match self {
            Value::U128(_) => {
                if size < 16 {
                    Some((None, 16))
                } else {
                    None
                }
            }
            Value::I128(_) => {
                if size < 16 {
                    Some((None, 16))
                } else {
                    None
                }
            }
            Value::U64(_) => {
                if size < 8 {
                    Some((None, 8))
                } else {
                    None
                }
            }
            Value::I64(_) => {
                if size < 8 {
                    Some((None, 8))
                } else {
                    None
                }
            }
            Value::U32(_) => {
                if size < 4 {
                    Some((None, 4))
                } else {
                    None
                }
            }
            Value::I32(_) => {
                if size < 4 {
                    Some((None, 4))
                } else {
                    None
                }
            }
            Value::U16(_) => {
                if size < 2 {
                    Some((None, 2))
                } else {
                    None
                }
            }
            Value::I16(_) => {
                if size < 2 {
                    Some((None, 2))
                } else {
                    None
                }
            }
            Value::U8(_) => {
                if size < 1 {
                    Some((None, 1))
                } else {
                    None
                }
            }
            Value::I8(_) => {
                if size < 1 {
                    Some((None, 1))
                } else {
                    None
                }
            }
            Value::Bytes(bytes) => {
                if (size as usize) < bytes.len() {
                    Some((None, bytes.len() as u32))
                } else {
                    None
                }
            }
            Value::Bytes20(_) => {
                if size < 20 {
                    Some((None, 20))
                } else {
                    None
                }
            }
            Value::Bytes32(_) => {
                if size < 32 {
                    Some((None, 32))
                } else {
                    None
                }
            }
            Value::Bytes36(_) => {
                if size < 36 {
                    Some((None, 36))
                } else {
                    None
                }
            }
            Value::EnumU8(_) => {
                if size < 1 {
                    Some((None, 1))
                } else {
                    None
                }
            }
            Value::EnumString(strings) => {
                let max_len = strings.iter().map(|string| string.len()).max();
                if let Some(max) = max_len {
                    if max > size as usize {
                        Some((None, max as u32))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Value::Identifier(_) => {
                if size < 32 {
                    Some((None, 32))
                } else {
                    None
                }
            }
            Value::Float(_) => {
                if size < 8 {
                    Some((None, 8))
                } else {
                    None
                }
            }
            Value::Text(string) => {
                if string.len() > size as usize {
                    Some((None, string.len() as u32))
                } else {
                    None
                }
            }
            Value::Bool(_) => {
                if size < 1 {
                    Some((None, 1))
                } else {
                    None
                }
            }
            Value::Null => {
                if size < 1 {
                    Some((None, 1))
                } else {
                    None
                }
            }
            Value::Array(values) => {
                for value in values {
                    if let Some(result) = value.has_data_larger_than(size) {
                        return Some((Some(value.clone()), result.1));
                    }
                }
                None
            }
            Value::Map(map) => {
                for (key, value) in map {
                    if let Some(result) = value.has_data_larger_than(size) {
                        return Some((Some(key.clone()), result.1));
                    }
                }
                None
            }
        }
    }
}

macro_rules! implfrom {
    ($($v:ident($t:ty)),+ $(,)?) => {
        $(
            impl From<$t> for Value {
                #[inline]
                fn from(value: $t) -> Self {
                    Self::$v(value.into())
                }
            }
        )+
    };
}

macro_rules! impltryinto {
    ($($t:ty),+ $(,)?) => {
        $(
            impl TryFrom<Value> for $t {
                type Error = Error;
                #[inline]
                fn try_from(value: Value) -> Result<Self, Self::Error> {
                    value.to_integer()
                }
            }
        )+
    };
}

impltryinto! {
    u128,
    i128,
    u64,
    i64,
    u32,
    i32,
    u16,
    i16,
    u8,
    i8,
}

implfrom! {
    U128(u128),
    I128(i128),
    U64(u64),
    I64(i64),
    U32(u32),
    I32(i32),
    U16(u16),
    I16(i16),
    U8(u8),
    I8(i8),

    Bytes(Vec<u8>),
    Bytes(&[u8]),
    Bytes32([u8;32]),
    Bytes20([u8;20]),
    Bytes36([u8;36]),

    Float(f64),
    Float(f32),

    Text(String),
    Text(&str),

    Bool(bool),

    Array(&[Value]),
    Array(Vec<Value>),

    Map(&[(Value, Value)]),
    Map(Vec<(Value, Value)>),
}

impl<const N: usize> From<[(Value, Value); N]> for Value {
    /// Converts a `[(Value, Value); N]` into a `Value`.
    ///
    /// ```
    /// use platform_value::Value;
    ///
    /// let map1 = Value::from([(Value::from(1), Value::from(2)), (Value::from(3), Value::from(4))]);
    /// let map2: Value = [(Value::from(1), Value::from(2)), (Value::from(3), Value::from(4))].into();
    /// assert_eq!(map1, map2);
    /// ```
    fn from(arr: [(Value, Value); N]) -> Self {
        if N == 0 {
            return Value::Map(vec![]);
        }

        Value::Map(arr.into_iter().collect())
    }
}

impl<const N: usize> From<[(String, Value); N]> for Value {
    /// Converts a `[(String, Value); N]` into a `Value`.
    ///
    /// ```
    /// use platform_value::Value;
    ///
    /// let map1 = Value::from([("1".to_string(), Value::from(2)), ("3".to_string(), Value::from(4))]);
    /// let map2: Value = [("1".to_string(), Value::from(2)), ("3".to_string(), Value::from(4))].into();
    /// assert_eq!(map1, map2);
    /// ```
    fn from(mut arr: [(String, Value); N]) -> Self {
        if N == 0 {
            return Value::Map(vec![]);
        }

        // use stable sort to preserve the insertion order.
        arr.sort_by(|a, b| a.0.cmp(&b.0));
        Value::Map(arr.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<const N: usize> From<[(&str, Value); N]> for Value {
    /// Converts a `[($str, Value); N]` into a `Value`.
    ///
    /// ```
    /// use platform_value::Value;
    ///
    /// let map1 = Value::from([("1", Value::from(2)), ("3", Value::from(4))]);
    /// let map2: Value = [("1", Value::from(2)), ("3", Value::from(4))].into();
    /// assert_eq!(map1, map2);
    /// ```
    fn from(mut arr: [(&str, Value); N]) -> Self {
        if N == 0 {
            return Value::Map(vec![]);
        }

        // use stable sort to preserve the insertion order.
        arr.sort_by(|a, b| a.0.cmp(b.0));
        Value::Map(arr.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<T> From<BTreeMap<T, &Value>> for Value
where
    T: Into<Value>,
{
    fn from(value: BTreeMap<T, &Value>) -> Self {
        Value::Map(
            value
                .into_iter()
                .map(|(key, value)| (key.into(), value.clone()))
                .collect(),
        )
    }
}

impl From<&BTreeMap<String, Value>> for Value {
    fn from(value: &BTreeMap<String, Value>) -> Self {
        Value::Map(
            value
                .iter()
                .map(|(key, value)| (key.into(), value.clone()))
                .collect(),
        )
    }
}

impl<T> From<BTreeMap<T, Value>> for Value
where
    T: Into<Value>,
{
    fn from(value: BTreeMap<T, Value>) -> Self {
        Value::Map(
            value
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }
}

impl<T> From<BTreeMap<T, Option<T>>> for Value
where
    T: Into<Value>,
{
    fn from(value: BTreeMap<T, Option<T>>) -> Self {
        Value::Map(
            value
                .into_iter()
                .map(|(key, value)| (key.into(), value.map(|a| a.into()).into()))
                .collect(),
        )
    }
}

impl From<Option<Value>> for Value {
    fn from(value: Option<Value>) -> Self {
        match value {
            None => Value::Null,
            Some(value) => value,
        }
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Value::Text(value.clone())
    }
}

impl From<char> for Value {
    #[inline]
    fn from(value: char) -> Self {
        let mut v = String::with_capacity(1);
        v.push(value);
        Value::Text(v)
    }
}

impl From<Vec<&str>> for Value {
    fn from(value: Vec<&str>) -> Self {
        Value::Array(value.into_iter().map(|string| string.into()).collect())
    }
}

impl From<&[&str]> for Value {
    fn from(value: &[&str]) -> Self {
        Value::Array(
            value
                .iter()
                .map(|string| string.to_owned().into())
                .collect(),
        )
    }
}
impl TryFrom<Value> for Vec<u8> {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.into_bytes()
    }
}

impl TryFrom<Value> for String {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.into_text()
    }
}
