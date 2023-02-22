//! Platform value
//! A dynamic Platform value
//!
//! A module that is used to represent values in Platform components
//! Forked from ciborium value
//!
//!
pub mod value_map;
mod integer;
mod error;
pub mod system_bytes;
pub mod inner_value;
pub mod converter;
pub mod btreemap_extensions;
pub mod display;


pub use error::Error;
pub use integer::Integer;

pub type ValueMap = Vec<(Value, Value)>;
pub type Hash256 = [u8;32];

/// A representation of a dynamic value that can handled dynamically
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
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

    // Todo: add this in
    // /// A 256 bit hash
    // Hash256(Hash256),

    /// Bytes
    Bytes(Vec<u8>),

    /// A float
    Float(f64),

    /// A string
    Text(String),

    /// A boolean
    Bool(bool),

    /// Null
    Null,

    /// Tag
    Tag(u64, Box<Value>),

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
    /// let value = Value::U64(17.into());
    ///
    /// assert!(value.is_integer());
    /// ```
    pub fn is_integer(&self) -> bool {
        match self {
            Value::U128(_)
            | Value::I128(_)
            | Value::U64(_)
            | Value::I64(_)
            | Value::U32(_)
            | Value::I32(_)
            | Value::U16(_)
            | Value::I16(_)
            | Value::U8(_)
            | Value::I8(_) => true,
            _ => false,
        }
    }

    /// If the `Value` is a `Integer`, returns a reference to the associated `Integer` data.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Integer(17.into());
    ///
    /// // We can read the number
    /// assert_eq!(17, value.as_integer().unwrap().try_into().unwrap());
    /// ```
    pub fn as_integer<T>(&self) -> Option<T>
    where T: TryFrom<i128> + TryFrom<u128> + TryFrom<u64> + TryFrom<i64> + TryFrom<u32> + TryFrom<i32> + TryFrom<u16> + TryFrom<i16> + TryFrom<u8> + TryFrom<i8> {
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
    /// # use platform_value::{Value, Integer, Error};
    /// #
    /// let value = Value::Integer(17.into());
    /// assert_eq!(value.into_integer(), Ok(Integer::from(17)));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_integer(), Err(Error::StructureError("value is not an integer".to_string())));
    /// ```
    pub fn into_integer<T>(self) -> Result<T, Error>
        where T: TryFrom<i128> + TryFrom<u128> + TryFrom<u64> + TryFrom<i64> + TryFrom<u32> + TryFrom<i32> + TryFrom<u16> + TryFrom<i16> + TryFrom<u8> + TryFrom<i8> {
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
            other => Err(Error::StructureError("value is not an integer".to_string())),
        }
    }

    /// If the `Value` is a `Integer`, returns a the associated `Integer` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Integer, Error};
    /// #
    /// let value = Value::Integer(17.into());
    /// assert_eq!(value.into_integer(), Ok(Integer::from(17)));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_integer(), Err(Error::StructureError("value is not an integer".to_string())));
    /// ```
    pub fn to_integer<T>(&self) -> Result<T, Error>
        where T: TryFrom<i128> + TryFrom<u128> + TryFrom<u64> + TryFrom<i64> + TryFrom<u32> + TryFrom<i32> + TryFrom<u16> + TryFrom<i16> + TryFrom<u8> + TryFrom<i8> {
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
            other => Err(Error::StructureError("value is not an integer".to_string())),
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
            other => Err(Error::StructureError("value are not bytes".to_string())),
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
    /// assert_eq!(value.to_bytes(), Err(Error::StructureError("value are not bytes".to_string())));
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        match self {
            Value::Bytes(vec) => Ok(vec.clone()),
            other => Err(Error::StructureError("value are not bytes".to_string())),
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
    /// assert_eq!(value.as_bytes_slice(), Err(Error::StructureError("value are not bytes".to_string())));
    /// ```
    pub fn as_bytes_slice(&self) -> Result<&[u8], Error> {
        match self {
            Value::Bytes(vec) => Ok(vec),
            other => Err(Error::StructureError("value are not bytes".to_string())),
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
            other => Err(Error::StructureError("value is not a float".to_string())),
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
            other => Err(Error::StructureError("value is not a string".to_string())),
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
            other => Err(Error::StructureError("value is not a string".to_string())),
        }
    }

    /// If the `Value` is a `String`, returns a reference to the associated `String` data as `Ok`.
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
    pub fn as_str(&self) -> Result<&str, Error> {
        match self {
            Value::Text(s) => Ok(s),
            other => Err(Error::StructureError("value is not a string".to_string())),
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
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Bool(false);
    /// assert_eq!(value.into_bool(), Ok(false));
    ///
    /// let value = Value::Float(17.);
    /// assert_eq!(value.into_bool(), Err(Value::Float(17.)));
    /// ```
    pub fn into_bool(self) -> Result<bool, Error> {
        match self {
            Value::Bool(b) => Ok(b),
            other => Err(Error::StructureError("value is not a bool".to_string())),
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

    /// Returns true if the `Value` is a `Tag`. Returns false otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Tag(61, Box::from(Value::Null));
    ///
    /// assert!(value.is_tag());
    /// ```
    pub fn is_tag(&self) -> bool {
        self.as_tag().is_some()
    }

    /// If the `Value` is a `Tag`, returns the associated tag value and a reference to the tag `Value`.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Tag(61, Box::from(Value::Bytes(vec![104, 101, 108, 108, 111])));
    ///
    /// let (tag, data) = value.as_tag().unwrap();
    /// assert_eq!(tag, 61);
    /// assert_eq!(data, &Value::Bytes(vec![104, 101, 108, 108, 111]));
    /// ```
    pub fn as_tag(&self) -> Option<(u64, &Value)> {
        match self {
            Value::Tag(tag, data) => Some((*tag, data)),
            _ => None,
        }
    }

    /// If the `Value` is a `Tag`, returns the associated tag value and a mutable reference
    /// to the tag `Value`. Returns None otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let mut value = Value::Tag(61, Box::from(Value::Bytes(vec![104, 101, 108, 108, 111])));
    ///
    /// let (tag, mut data) = value.as_tag_mut().unwrap();
    /// data.as_bytes_mut().unwrap().clear();
    /// assert_eq!(tag, &61);
    /// assert_eq!(data, &Value::Bytes(vec![]));
    /// ```
    pub fn as_tag_mut(&mut self) -> Option<(&mut u64, &mut Value)> {
        match self {
            Value::Tag(tag, data) => Some((tag, data.as_mut())),
            _ => None,
        }
    }

    /// If the `Value` is a `Tag`, returns a the associated pair of `u64` and `Box<value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::Value;
    /// # use platform_value::Error;
    /// #
    /// let value = Value::Tag(7, Box::new(Value::Float(12.)));
    /// assert_eq!(value.into_tag(), Ok((7, Box::new(Value::Float(12.)))));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_tag(), Err(Error::StructureError("value is not a tag".to_string())));
    /// ```
    pub fn into_tag(self) -> Result<(u64, Box<Value>), Error> {
        match self {
            Value::Tag(tag, value) => Ok((tag, value)),
            other => Err(Error::StructureError("value is not a tag".to_string())),
        }
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

    /// If the `Value` is a `Array`, returns a the associated `Vec<Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Integer, Error};
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::Integer(17.into()),
    ///         Value::Float(18.),
    ///     ]
    /// );
    /// assert_eq!(value.into_array(), Ok(vec![Value::Integer(17.into()), Value::Float(18.)]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_array(), Err(Error::StructureError("value is not an array".to_string())));
    /// ```
    pub fn into_array(self) -> Result<Vec<Value>, Error> {
        match self {
            Value::Array(vec) => Ok(vec),
            other => Err(Error::StructureError("value is not an array".to_string())),
        }
    }

    /// If the `Value` is a `Array`, returns a the associated `Vec<Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Value, Integer, Error};
    /// #
    /// let mut value = Value::Array(
    ///     vec![
    ///         Value::Integer(17.into()),
    ///         Value::Float(18.),
    ///     ]
    /// );
    /// assert_eq!(value.as_slice(), Ok(vec![Value::Integer(17.into()), Value::Float(18.)]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.as_slice(), Err(Error::StructureError("value is not an array".to_string())));
    /// ```
    pub fn as_slice(&self) -> Result<&[Value], Error> {
        match self {
            Value::Array(vec) => Ok(vec),
            other => Err(Error::StructureError("value is not an array".to_string())),
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
            other => Err(Error::StructureError("value is not a map".to_string())),
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

impl From<char> for Value {
    #[inline]
    fn from(value: char) -> Self {
        let mut v = String::with_capacity(1);
        v.push(value);
        Value::Text(v)
    }
}
