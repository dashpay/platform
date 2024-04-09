use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::{alphabet, Engine};

use crate::{BinaryData, Bytes20, Bytes32, Bytes36, Error, Identifier, Value};

pub const PADDING_INDIFFERENT: GeneralPurposeConfig = GeneralPurposeConfig::new()
    .with_encode_padding(false)
    .with_decode_padding_mode(DecodePaddingMode::Indifferent);

impl Value {
    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.into_identifier_bytes(), Ok(vec![104, 101, 108, 108, 111]));    ///
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_identifier_bytes(), Ok(vec![98, 155, 36]));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108)]);
    /// assert_eq!(value.into_identifier_bytes(), Ok(vec![104, 101, 108]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_identifier_bytes(), Ok(vec![5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_identifier_bytes(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_identifier_bytes(self) -> Result<Vec<u8>, Error> {
        match self {
            Value::Text(text) => bs58::decode(text).into_vec().map_err(|_| {
                Error::StructureError(
                    "value was a string, but could not be decoded from base 58".to_string(),
                )
            }),
            Value::Array(array) => array
                .into_iter()
                .map(|byte| match byte {
                    Value::U8(value_as_u8) => Ok(value_as_u8),
                    _ => Err(Error::StructureError("not an array of bytes".to_string())),
                })
                .collect::<Result<Vec<u8>, Error>>(),
            Value::Bytes(vec) => Ok(vec),
            Value::Bytes32(bytes) => Ok(bytes.into()),
            Value::Identifier(identifier) => Ok(Vec::from(identifier)),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a ref to a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.to_identifier_bytes(), Ok(vec![104, 101, 108, 108, 111]));    ///
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_identifier_bytes(), Ok(vec![98, 155, 36]));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108)]);
    /// assert_eq!(value.to_identifier_bytes(), Ok(vec![104, 101, 108]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.to_identifier_bytes(), Ok(vec![5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_identifier_bytes(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_identifier_bytes(&self) -> Result<Vec<u8>, Error> {
        match self {
            Value::Text(text) => bs58::decode(text).into_vec().map_err(|_| {
                Error::StructureError(
                    "value was a string, but could not be decoded from base 58".to_string(),
                )
            }),
            Value::Array(array) => array
                .iter()
                .map(|byte| match byte {
                    Value::U8(value_as_u8) => Ok(*value_as_u8),
                    _ => Err(Error::StructureError("not an array of bytes".to_string())),
                })
                .collect::<Result<Vec<u8>, Error>>(),
            Value::Bytes(vec) => Ok(vec.clone()),
            Value::Bytes32(vec) => Ok(vec.to_vec()),
            Value::Identifier(identifier) => Ok(Vec::from(identifier.as_slice())),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 64 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.into_binary_bytes(), Ok(vec![104, 101, 108, 108, 111]));    ///
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_binary_bytes(), Ok(vec![107, 205, 117]));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108)]);
    /// assert_eq!(value.into_binary_bytes(), Ok(vec![104, 101, 108]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_binary_bytes(), Ok(vec![5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_binary_bytes(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_binary_bytes(self) -> Result<Vec<u8>, Error> {
        match self {
            Value::Text(text) => GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT).decode(text).map_err(|e| {
                Error::StructureError(
                    format!("value was a string, but could not be decoded from base 64 into binary bytes, error: {}", e),
                )
            }),
            Value::Array(array) => array
                .into_iter()
                .map(|byte| match byte {
                    Value::U8(value_as_u8) => Ok(value_as_u8),
                    _ => Err(Error::StructureError("not an array of bytes".to_string())),
                })
                .collect::<Result<Vec<u8>, Error>>(),
            Value::Bytes(vec) => Ok(vec),
            Value::Bytes32(bytes) => Ok(bytes.into()),
            Value::Identifier(identifier) => Ok(Vec::from(identifier)),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 64 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{BinaryData, Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.into_binary_data(), Ok(BinaryData::new(vec![104, 101, 108, 108, 111])));    ///
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_binary_data(), Ok(BinaryData::new(vec![107, 205, 117])));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108)]);
    /// assert_eq!(value.into_binary_data(), Ok(BinaryData::new(vec![104, 101, 108])));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_binary_data(), Ok(BinaryData::new(vec![5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_binary_data(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_binary_data(self) -> Result<BinaryData, Error> {
        Ok(BinaryData::new(self.into_binary_bytes()?))
    }

    /// If the `Value` is a ref to a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.to_binary_bytes(), Ok(vec![104, 101, 108, 108, 111]));    ///
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_binary_bytes(), Ok(vec![107, 205, 117]));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108)]);
    /// assert_eq!(value.to_binary_bytes(), Ok(vec![104, 101, 108]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.to_binary_bytes(), Ok(vec![5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_binary_bytes(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_binary_bytes(&self) -> Result<Vec<u8>, Error> {
        match self {
            Value::Text(text) => GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT)
                .decode(text)
                .map_err(|e| {
                    Error::StructureError(format!(
                        "value was a string, but could not be decoded from base 64, error: {}",
                        e
                    ))
                }),
            Value::Array(array) => array
                .iter()
                .map(|byte| match byte {
                    Value::U8(value_as_u8) => Ok(*value_as_u8),
                    _ => Err(Error::StructureError("not an array of bytes".to_string())),
                })
                .collect::<Result<Vec<u8>, Error>>(),
            Value::Bytes(vec) => Ok(vec.clone()),
            Value::Bytes20(vec) => Ok(vec.to_vec()),
            Value::Bytes32(vec) => Ok(vec.to_vec()),
            Value::Bytes36(vec) => Ok(vec.to_vec()),
            Value::Identifier(identifier) => Ok(Vec::from(identifier.as_slice())),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]);
    /// assert_eq!(value.into_hash256(), Ok([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]));    ///
    ///
    /// let value = Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string());
    /// assert_eq!(value.into_hash256(), Ok([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117]));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_hash256(), Err(Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.into_hash256(), Err(Error::StructureError("value was a string, but could not be decoded from base 58".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)]);
    /// assert_eq!(value.into_hash256(), Ok([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_hash256(), Ok([5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_hash256(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_hash256(self) -> Result<[u8; 32], Error> {
        match self {
            Value::Text(text) => {
                bs58::decode(text).into_vec()
                    .map_err(|_| Error::StructureError("value was a string, but could not be decoded from base 58".to_string()))?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string()))
            }
            Value::Array(array) => {
                Ok(array
                    .into_iter()
                    .map(|byte| match byte {
                        Value::U8(value_as_u8) => {
                            Ok(value_as_u8)
                        }
                        _ => Err(Error::StructureError("not an array of bytes".to_string())),
                    })
                    .collect::<Result<Vec<u8>, Error>>()?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was an array of bytes, but was not 32 bytes long".to_string()))?)
            }
            Value::Bytes(vec) => {
                vec.try_into()
                    .map_err(|_| Error::StructureError("value was bytes, but was not 32 bytes long".to_string()))
            },
            Value::Bytes32(bytes) => Ok(bytes),
            Value::Identifier(identifier) => Ok(identifier),
            _other => Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]);
    /// assert_eq!(value.to_hash256(), Ok([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]));    ///
    ///
    /// let value = Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string());
    /// assert_eq!(value.to_hash256(), Ok([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117]));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_hash256(), Err(Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.to_hash256(), Err(Error::StructureError("value was a string, but could not be decoded from base 58".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)]);
    /// assert_eq!(value.to_hash256(), Ok([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.to_hash256(), Ok([5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_hash256(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_hash256(&self) -> Result<[u8; 32], Error> {
        match self {
            Value::Text(text) => {
                bs58::decode(text).into_vec()
                    .map_err(|_| Error::StructureError("value was a string, but could not be decoded from base 58".to_string()))?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string()))
            },
            Value::Array(array) => {
                Ok(array
                    .iter()
                    .map(|byte| byte.to_integer())
                    .collect::<Result<Vec<u8>, Error>>()?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was an array of bytes, but was not 32 bytes long".to_string()))?)
            },
            Value::Bytes32(bytes) => Ok(*bytes),
            Value::Bytes(vec) => {
                vec.clone().try_into()
                    .map_err(|_| Error::StructureError("value was bytes, but was not 32 bytes long".to_string()))
            },
            Value::Identifier(identifier) => Ok(identifier.to_owned()),
            _other => Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 64 or Vector of `U8`, returns the
    /// associated `Bytes20` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Bytes20, Error, Value};
    ///
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108]);
    /// assert_eq!(value.into_bytes_20(), Ok(Bytes20([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108])));    ///
    ///
    /// let value = Value::Text("WRdZY72e8yUfNJ5VC9ckzga7ysE=".to_string());
    /// assert_eq!(value.into_bytes_20(), Ok(Bytes20([89, 23, 89, 99, 189, 158, 243, 37, 31, 52, 158, 85, 11, 215, 36, 206, 6, 187, 202, 193])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_bytes_20(), Err(Error::ByteLengthNot20BytesError("buffer was not 20 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.into_bytes_20(), Err(Error::StructureError("value was a string, but could not be decoded from base 64 into bytes 20, error: Invalid last symbol 105, offset 5.".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104)]);
    /// assert_eq!(value.into_bytes_20(), Ok(Bytes20([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_bytes_20(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_bytes_20(self) -> Result<Bytes20, Error> {
        match self {
            Value::Text(text) => Bytes20::from_vec(GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT).decode(text).map_err(|e| {
                Error::StructureError(
                    format!("value was a string, but could not be decoded from base 64 into bytes 20, error: {}", e),
                )
            })?),
            Value::Array(array) => Bytes20::from_vec(
                array
                    .into_iter()
                    .map(|byte| byte.into_integer())
                    .collect::<Result<Vec<u8>, Error>>()?,
            ),
            Value::Bytes20(bytes) => Ok(Bytes20::new(bytes)),
            Value::Bytes(vec) => Bytes20::from_vec(vec),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 64 or Vector of `U8`, returns the
    /// associated `Bytes20` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Bytes20, Error, Value};
    ///
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108]);
    /// assert_eq!(value.to_bytes_20(), Ok(Bytes20([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108])));    ///
    ///
    /// let value = Value::Text("WRdZY72e8yUfNJ5VC9ckzga7ysE=".to_string());
    /// assert_eq!(value.to_bytes_20(), Ok(Bytes20([89, 23, 89, 99, 189, 158, 243, 37, 31, 52, 158, 85, 11, 215, 36, 206, 6, 187, 202, 193])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_bytes_20(), Err(Error::ByteLengthNot20BytesError("buffer was not 20 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.to_bytes_20(), Err(Error::StructureError("value was a string, but could not be decoded from base 64 to bytes 20, error: Invalid last symbol 105, offset 5.".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104)]);
    /// assert_eq!(value.to_bytes_20(), Ok(Bytes20([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_bytes_20(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_bytes_20(&self) -> Result<Bytes20, Error> {
        match self {
            Value::Text(text) => Bytes20::from_vec(GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT).decode(text).map_err(|e| {
                Error::StructureError(
                    format!("value was a string, but could not be decoded from base 64 to bytes 20, error: {}", e),
                )
            })?),
            Value::Array(array) => Bytes20::from_vec(
                array
                    .iter()
                    .map(|byte| byte.to_integer())
                    .collect::<Result<Vec<u8>, Error>>()?,
            ),
            Value::Bytes20(bytes) => Ok(Bytes20::new(*bytes)),
            Value::Bytes(vec) => Bytes20::from_vec(vec.clone()),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 64 or Vector of `U8`, returns the
    /// associated `Bytes32` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Bytes32, Error, Value};
    ///
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]);
    /// assert_eq!(value.into_bytes_32(), Ok(Bytes32([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50])));    ///
    ///
    /// let value = Value::Text("ViN2Q6crZW1IYSNjAP5smv6avijtGTr2bxMs142MnHU=".to_string());
    /// assert_eq!(value.into_bytes_32(), Ok(Bytes32([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_bytes_32(), Err(Error::ByteLengthNot32BytesError("buffer was not 32 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.into_bytes_32(), Err(Error::StructureError("value was a string, but could not be decoded from base 64 into bytes 32, error: Invalid last symbol 105, offset 5.".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)]);
    /// assert_eq!(value.into_bytes_32(), Ok(Bytes32([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101])));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_bytes_32(), Ok(Bytes32([5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_bytes_32(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_bytes_32(self) -> Result<Bytes32, Error> {
        match self {
            Value::Text(text) => Bytes32::from_vec(GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT).decode(text).map_err(|e| {
                Error::StructureError(
                    format!("value was a string, but could not be decoded from base 64 into bytes 32, error: {}", e),
                )
            })?),
            Value::Array(array) => Bytes32::from_vec(
                array
                    .into_iter()
                    .map(|byte| byte.into_integer())
                    .collect::<Result<Vec<u8>, Error>>()?,
            ),
            Value::Bytes32(bytes) => Ok(Bytes32::new(bytes)),
            Value::Bytes(vec) => Bytes32::from_vec(vec),
            Value::Identifier(identifier) => Ok(Bytes32::new(identifier)),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Bytes32, Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]);
    /// assert_eq!(value.to_bytes_32(), Ok(Bytes32::new([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50])));    ///
    ///
    /// let value = Value::Text("ViN2Q6crZW1IYSNjAP5smv6avijtGTr2bxMs142MnHU=".to_string());
    /// assert_eq!(value.to_bytes_32(), Ok(Bytes32::new([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_bytes_32(), Err(Error::ByteLengthNot32BytesError("buffer was not 32 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.to_bytes_32(), Err(Error::StructureError("value was a string, but could not be decoded from base 64 to bytes 32, error: Invalid last symbol 105, offset 5.".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)]);
    /// assert_eq!(value.to_bytes_32(), Ok(Bytes32::new([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101])));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.to_bytes_32(), Ok(Bytes32::new([5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_bytes_32(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_bytes_32(&self) -> Result<Bytes32, Error> {
        match self {
            Value::Text(text) => Bytes32::from_vec(GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT).decode(text).map_err(|e| {
                Error::StructureError(
                    format!("value was a string, but could not be decoded from base 64 to bytes 32, error: {}", e),
                )
            })?),
            Value::Array(array) => Bytes32::from_vec(
                array
                    .iter()
                    .map(|byte| byte.to_integer())
                    .collect::<Result<Vec<u8>, Error>>()?,
            ),
            Value::Bytes32(bytes) => Ok(Bytes32::new(*bytes)),
            Value::Bytes(vec) => Bytes32::from_vec(vec.clone()),
            Value::Identifier(identifier) => Ok(Bytes32::new(*identifier)),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 64 or Vector of `U8`, returns the
    /// associated `Bytes36` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Bytes36, Error, Value};
    ///
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 50, 51, 52, 53]);
    /// assert_eq!(value.into_bytes_36(), Ok(Bytes36([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 50, 51, 52, 53])));    ///
    ///
    /// let value = Value::Text("sNr9aH0VUnzvKDADkuJUlZ4Yj7Yd7gbNnnOR4ANNat2g498D".to_string());
    /// assert_eq!(value.into_bytes_36(), Ok(Bytes36([176, 218, 253, 104, 125, 21, 82, 124, 239, 40, 48, 3, 146, 226, 84, 149, 158, 24, 143, 182, 29, 238, 6, 205, 158, 115, 145, 224, 3, 77, 106, 221, 160, 227, 223, 3])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_bytes_36(), Err(Error::ByteLengthNot36BytesError("buffer was not 36 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.into_bytes_36(), Err(Error::StructureError("value was a string, but could not be decoded from base 64 into bytes 36, error: Invalid last symbol 105, offset 5.".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(101), Value::U8(101), Value::U8(101), Value::U8(101)]);
    /// assert_eq!(value.into_bytes_36(), Ok(Bytes36([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 101, 101, 101, 101])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_bytes_36(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_bytes_36(self) -> Result<Bytes36, Error> {
        match self {
            Value::Text(text) => Bytes36::from_vec(GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT).decode(text).map_err(|e| {
                Error::StructureError(
                    format!("value was a string, but could not be decoded from base 64 into bytes 36, error: {}", e),
                )
            })?),
            Value::Array(array) => Bytes36::from_vec(
                array
                    .into_iter()
                    .map(|byte| byte.into_integer())
                    .collect::<Result<Vec<u8>, Error>>()?,
            ),
            Value::Bytes36(bytes) => Ok(Bytes36::new(bytes)),
            Value::Bytes(vec) => Bytes36::from_vec(vec),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 64 or Vector of `U8`, returns the
    /// associated `Bytes36` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Bytes36, Error, Value};
    ///
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 50, 51, 52, 53]);
    /// assert_eq!(value.to_bytes_36(), Ok(Bytes36([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 50, 51, 52, 53])));    ///
    ///
    /// let value = Value::Text("sNr9aH0VUnzvKDADkuJUlZ4Yj7Yd7gbNnnOR4ANNat2g498D".to_string());
    /// assert_eq!(value.to_bytes_36(), Ok(Bytes36([176, 218, 253, 104, 125, 21, 82, 124, 239, 40, 48, 3, 146, 226, 84, 149, 158, 24, 143, 182, 29, 238, 6, 205, 158, 115, 145, 224, 3, 77, 106, 221, 160, 227, 223, 3])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_bytes_36(), Err(Error::ByteLengthNot36BytesError("buffer was not 36 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.to_bytes_36(), Err(Error::StructureError("value was a string, but could not be decoded from base 64 to bytes 36, error: Invalid last symbol 105, offset 5.".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(101), Value::U8(101), Value::U8(101), Value::U8(101)]);
    /// assert_eq!(value.to_bytes_36(), Ok(Bytes36([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 101, 101, 101, 101])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_bytes_36(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_bytes_36(&self) -> Result<Bytes36, Error> {
        match self {
            Value::Text(text) => Bytes36::from_vec(GeneralPurpose::new(&alphabet::STANDARD, PADDING_INDIFFERENT).decode(text).map_err(|e| {
                Error::StructureError(
                    format!("value was a string, but could not be decoded from base 64 to bytes 36, error: {}", e),
                )
            })?),
            Value::Array(array) => Bytes36::from_vec(
                array
                    .iter()
                    .map(|byte| byte.to_integer())
                    .collect::<Result<Vec<u8>, Error>>()?,
            ),
            Value::Bytes36(bytes) => Ok(Bytes36::new(*bytes)),
            Value::Bytes(vec) => Bytes36::from_vec(vec.clone()),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Identifier` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Identifier, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]);
    /// assert_eq!(value.into_identifier(), Ok(Identifier::new([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50])));    ///
    ///
    /// let value = Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string());
    /// assert_eq!(value.into_identifier(), Ok(Identifier::new([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_identifier(), Err(Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.into_identifier(), Err(Error::StructureError("value was a string, but could not be decoded from base 58".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)]);
    /// assert_eq!(value.into_identifier(), Ok(Identifier::new([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101])));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_identifier(), Ok(Identifier::new([5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_identifier(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_identifier(self) -> Result<Identifier, Error> {
        match self {
            Value::Text(text) => {
                bs58::decode(text).into_vec()
                    .map_err(|_| Error::StructureError("value was a string, but could not be decoded from base 58".to_string()))?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string()))
            }
            Value::Array(array) => {
                Ok(array
                    .into_iter()
                    .map(|byte| match byte {
                        Value::U8(value_as_u8) => {
                            Ok(value_as_u8)
                        }
                        _ => Err(Error::StructureError("not an array of bytes".to_string())),
                    })
                    .collect::<Result<Vec<u8>, Error>>()?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was an array of bytes, but was not 32 bytes long".to_string()))?)
            }
            Value::Bytes(vec) => {
                vec.try_into()
                    .map_err(|_| Error::StructureError("value was bytes, but was not 32 bytes long".to_string()))
            },
            Value::Bytes32(bytes) => Ok(Identifier::new(bytes)),
            Value::Identifier(identifier) => Ok(Identifier::new(identifier)),
            _other => Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())),
        }
    }

    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Identifier` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Identifier, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]);
    /// assert_eq!(value.to_identifier(), Ok(Identifier::new([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50])));    ///
    ///
    /// let value = Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string());
    /// assert_eq!(value.to_identifier(), Ok(Identifier::new([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117])));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_identifier(), Err(Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.to_identifier(), Err(Error::StructureError("value was a string, but could not be decoded from base 58".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)]);
    /// assert_eq!(value.to_identifier(), Ok(Identifier::new([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101])));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.to_identifier(), Ok(Identifier::new([5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_identifier(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_identifier(&self) -> Result<Identifier, Error> {
        match self {
            Value::Text(text) => {
                bs58::decode(text).into_vec()
                    .map_err(|_| Error::StructureError("value was a string, but could not be decoded from base 58".to_string()))?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string()))
            },
            Value::Array(array) => {
                Ok(array
                    .iter()
                    .map(|byte| byte.to_integer())
                    .collect::<Result<Vec<u8>, Error>>()?
                    .try_into()
                    .map_err(|_| Error::StructureError("value was an array of bytes, but was not 32 bytes long".to_string()))?)
            },
            Value::Bytes32(bytes) => Ok(Identifier::new(*bytes)),
            Value::Bytes(vec) => {
                vec.clone().try_into()
                    .map_err(|_| Error::StructureError("value was bytes, but was not 32 bytes long".to_string()))
            },
            Value::Identifier(identifier) => Ok(Identifier::new(*identifier)),
            _other => Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())),
        }
    }
}
