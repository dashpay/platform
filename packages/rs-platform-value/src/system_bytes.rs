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
                .map(|byte| byte.to_integer())
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
                .map(|byte| byte.to_integer())
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // into_identifier_bytes / to_identifier_bytes  (base-58 encoding)
    // ---------------------------------------------------------------

    #[test]
    fn into_identifier_bytes_from_bytes() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert_eq!(val.into_identifier_bytes().unwrap(), vec![1, 2, 3]);
    }

    // A binary (ByteArray) document property that round-trips through a schemaless
    // JSON layer (e.g. an edit-and-replace where the client re-serializes the
    // cached document) arrives as a plain JSON array of integers, which decode to
    // wider `Value` integer variants (`U64`), not `Value::U8`. The byte-array
    // encode path must accept those like its `to_bytes_32`/`to_identifier`
    // siblings, otherwise the replace fails with "not an array of bytes".
    #[test]
    fn to_binary_bytes_accepts_array_of_wider_integers() {
        let value = Value::Array(vec![
            Value::U64(0xde),
            Value::U64(0xad),
            Value::U64(0xbe),
            Value::U64(0xef),
        ]);
        assert_eq!(value.to_binary_bytes().unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn into_binary_bytes_accepts_array_of_mixed_integer_variants() {
        let value = Value::Array(vec![Value::U64(1), Value::U8(2), Value::I64(3)]);
        assert_eq!(value.into_binary_bytes().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn into_identifier_bytes_from_text_bs58() {
        // "a811" is a valid base58 string
        let val = Value::Text("a811".to_string());
        let result = val.into_identifier_bytes().unwrap();
        // Verify round-trip: encode back to base58
        let encoded = bs58::encode(&result).into_string();
        assert_eq!(encoded, "a811");
    }

    #[test]
    fn into_identifier_bytes_from_array_of_u8() {
        let val = Value::Array(vec![Value::U8(10), Value::U8(20), Value::U8(30)]);
        assert_eq!(val.into_identifier_bytes().unwrap(), vec![10, 20, 30]);
    }

    #[test]
    fn into_identifier_bytes_from_identifier() {
        let id = [7u8; 32];
        let val = Value::Identifier(id);
        assert_eq!(val.into_identifier_bytes().unwrap(), id.to_vec());
    }

    #[test]
    fn into_identifier_bytes_from_bytes32() {
        let data = [3u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.into_identifier_bytes().unwrap(), data.to_vec());
    }

    #[test]
    fn into_identifier_bytes_wrong_type_errors() {
        let val = Value::Bool(true);
        assert!(val.into_identifier_bytes().is_err());
    }

    #[test]
    fn into_identifier_bytes_bad_array_element_errors() {
        let val = Value::Array(vec![Value::Text("not_a_byte".into())]);
        assert!(val.into_identifier_bytes().is_err());
    }

    #[test]
    fn to_identifier_bytes_round_trip() {
        let original = vec![50, 100, 150, 200];
        let val = Value::Bytes(original.clone());
        assert_eq!(val.to_identifier_bytes().unwrap(), original);
    }

    #[test]
    fn to_identifier_bytes_from_text_bs58() {
        let val = Value::Text("a811".to_string());
        let result = val.to_identifier_bytes().unwrap();
        let encoded = bs58::encode(&result).into_string();
        assert_eq!(encoded, "a811");
    }

    #[test]
    fn to_identifier_bytes_from_array() {
        let val = Value::Array(vec![Value::U8(5), Value::U8(10)]);
        assert_eq!(val.to_identifier_bytes().unwrap(), vec![5, 10]);
    }

    #[test]
    fn to_identifier_bytes_from_bytes32() {
        let data = [9u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.to_identifier_bytes().unwrap(), data.to_vec());
    }

    #[test]
    fn to_identifier_bytes_from_identifier() {
        let id = [11u8; 32];
        let val = Value::Identifier(id);
        assert_eq!(val.to_identifier_bytes().unwrap(), id.to_vec());
    }

    #[test]
    fn to_identifier_bytes_wrong_type_errors() {
        let val = Value::Float(1.0);
        assert!(val.to_identifier_bytes().is_err());
    }

    // ---------------------------------------------------------------
    // into_binary_bytes / to_binary_bytes  (base-64 encoding)
    // ---------------------------------------------------------------

    #[test]
    fn into_binary_bytes_from_bytes() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert_eq!(val.into_binary_bytes().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn into_binary_bytes_from_text_b64() {
        // base64 "AQID" = [1, 2, 3]
        let val = Value::Text("AQID".to_string());
        assert_eq!(val.into_binary_bytes().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn into_binary_bytes_from_array_of_u8() {
        let val = Value::Array(vec![Value::U8(4), Value::U8(5)]);
        assert_eq!(val.into_binary_bytes().unwrap(), vec![4, 5]);
    }

    #[test]
    fn into_binary_bytes_from_identifier() {
        let id = [2u8; 32];
        let val = Value::Identifier(id);
        assert_eq!(val.into_binary_bytes().unwrap(), id.to_vec());
    }

    #[test]
    fn into_binary_bytes_from_bytes32() {
        let data = [6u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.into_binary_bytes().unwrap(), data.to_vec());
    }

    #[test]
    fn into_binary_bytes_wrong_type_errors() {
        let val = Value::Bool(false);
        assert!(val.into_binary_bytes().is_err());
    }

    #[test]
    fn into_binary_bytes_bad_b64_errors() {
        // "!!!!" is not valid base64
        let val = Value::Text("!!!!".to_string());
        assert!(val.into_binary_bytes().is_err());
    }

    #[test]
    fn to_binary_bytes_round_trip() {
        let original = vec![10, 20, 30, 40];
        let val = Value::Bytes(original.clone());
        assert_eq!(val.to_binary_bytes().unwrap(), original);
    }

    #[test]
    fn to_binary_bytes_from_text_b64() {
        let val = Value::Text("AQID".to_string());
        assert_eq!(val.to_binary_bytes().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn to_binary_bytes_from_bytes20() {
        let data = [8u8; 20];
        let val = Value::Bytes20(data);
        assert_eq!(val.to_binary_bytes().unwrap(), data.to_vec());
    }

    #[test]
    fn to_binary_bytes_from_bytes32() {
        let data = [9u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.to_binary_bytes().unwrap(), data.to_vec());
    }

    #[test]
    fn to_binary_bytes_from_bytes36() {
        let data = [10u8; 36];
        let val = Value::Bytes36(data);
        assert_eq!(val.to_binary_bytes().unwrap(), data.to_vec());
    }

    #[test]
    fn to_binary_bytes_from_identifier() {
        let id = [11u8; 32];
        let val = Value::Identifier(id);
        assert_eq!(val.to_binary_bytes().unwrap(), id.to_vec());
    }

    #[test]
    fn to_binary_bytes_wrong_type_errors() {
        let val = Value::Null;
        assert!(val.to_binary_bytes().is_err());
    }

    // ---------------------------------------------------------------
    // into_binary_data / to_binary_data (wraps binary_bytes)
    // ---------------------------------------------------------------

    #[test]
    fn into_binary_data_from_bytes() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert_eq!(
            val.into_binary_data().unwrap(),
            BinaryData::new(vec![1, 2, 3])
        );
    }

    #[test]
    fn into_binary_data_from_identifier() {
        let id = [5u8; 32];
        let val = Value::Identifier(id);
        assert_eq!(
            val.into_binary_data().unwrap(),
            BinaryData::new(id.to_vec())
        );
    }

    #[test]
    fn into_binary_data_wrong_type_errors() {
        let val = Value::Bool(true);
        assert!(val.into_binary_data().is_err());
    }

    // ---------------------------------------------------------------
    // into_hash256 / to_hash256
    // ---------------------------------------------------------------

    #[test]
    fn into_hash256_from_bytes32() {
        let data = [4u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.into_hash256().unwrap(), data);
    }

    #[test]
    fn into_hash256_from_identifier() {
        let data = [5u8; 32];
        let val = Value::Identifier(data);
        assert_eq!(val.into_hash256().unwrap(), data);
    }

    #[test]
    fn into_hash256_from_bytes_correct_length() {
        let data = vec![6u8; 32];
        let val = Value::Bytes(data.clone());
        assert_eq!(
            val.into_hash256().unwrap(),
            <[u8; 32]>::try_from(data.as_slice()).unwrap()
        );
    }

    #[test]
    fn into_hash256_from_bytes_wrong_length_errors() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert!(val.into_hash256().is_err());
    }

    #[test]
    fn into_hash256_from_text_bs58_valid() {
        // Create a valid 32-byte bs58 string
        let data = [7u8; 32];
        let encoded = bs58::encode(data).into_string();
        let val = Value::Text(encoded);
        assert_eq!(val.into_hash256().unwrap(), data);
    }

    #[test]
    fn into_hash256_from_text_bs58_wrong_length() {
        // "a811" decodes to 3 bytes, not 32
        let val = Value::Text("a811".to_string());
        assert!(val.into_hash256().is_err());
    }

    #[test]
    fn into_hash256_from_text_invalid_bs58() {
        let val = Value::Text("a811Ii".to_string());
        assert!(val.into_hash256().is_err());
    }

    #[test]
    fn into_hash256_from_array_of_32_u8s() {
        let arr: Vec<Value> = (0..32).map(|i| Value::U8(i as u8)).collect();
        let val = Value::Array(arr);
        let expected: [u8; 32] = core::array::from_fn(|i| i as u8);
        assert_eq!(val.into_hash256().unwrap(), expected);
    }

    #[test]
    fn into_hash256_from_array_wrong_length() {
        let val = Value::Array(vec![Value::U8(1), Value::U8(2)]);
        assert!(val.into_hash256().is_err());
    }

    #[test]
    fn into_hash256_wrong_type_errors() {
        let val = Value::Bool(true);
        assert!(val.into_hash256().is_err());
    }

    #[test]
    fn to_hash256_from_bytes32() {
        let data = [8u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.to_hash256().unwrap(), data);
    }

    #[test]
    fn to_hash256_from_identifier() {
        let data = [9u8; 32];
        let val = Value::Identifier(data);
        assert_eq!(val.to_hash256().unwrap(), data);
    }

    #[test]
    fn to_hash256_from_bytes_correct_length() {
        let data = vec![10u8; 32];
        let val = Value::Bytes(data.clone());
        assert_eq!(
            val.to_hash256().unwrap(),
            <[u8; 32]>::try_from(data.as_slice()).unwrap()
        );
    }

    #[test]
    fn to_hash256_from_bytes_wrong_length_errors() {
        let val = Value::Bytes(vec![1, 2]);
        assert!(val.to_hash256().is_err());
    }

    #[test]
    fn to_hash256_round_trip_with_bs58() {
        let data = [11u8; 32];
        let encoded = bs58::encode(data).into_string();
        let val = Value::Text(encoded);
        assert_eq!(val.to_hash256().unwrap(), data);
    }

    #[test]
    fn to_hash256_wrong_type_errors() {
        let val = Value::Float(1.0);
        assert!(val.to_hash256().is_err());
    }

    // ---------------------------------------------------------------
    // into_bytes_20 / to_bytes_20
    // ---------------------------------------------------------------

    #[test]
    fn into_bytes_20_from_bytes20() {
        let data = [1u8; 20];
        let val = Value::Bytes20(data);
        assert_eq!(val.into_bytes_20().unwrap(), Bytes20::new(data));
    }

    #[test]
    fn into_bytes_20_from_bytes_correct_length() {
        let data = vec![2u8; 20];
        let val = Value::Bytes(data);
        assert_eq!(val.into_bytes_20().unwrap(), Bytes20::new([2u8; 20]));
    }

    #[test]
    fn into_bytes_20_from_bytes_wrong_length_errors() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert!(val.into_bytes_20().is_err());
    }

    #[test]
    fn into_bytes_20_from_text_b64_round_trip() {
        let data = [3u8; 20];
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let val = Value::Text(encoded);
        assert_eq!(val.into_bytes_20().unwrap(), Bytes20::new(data));
    }

    #[test]
    fn into_bytes_20_from_text_b64_wrong_length() {
        // "AQID" = 3 bytes, not 20
        let val = Value::Text("AQID".to_string());
        assert!(val.into_bytes_20().is_err());
    }

    #[test]
    fn into_bytes_20_from_array_of_20_u8s() {
        let arr: Vec<Value> = (0..20).map(|i| Value::U8(i as u8)).collect();
        let val = Value::Array(arr);
        let expected: [u8; 20] = core::array::from_fn(|i| i as u8);
        assert_eq!(val.into_bytes_20().unwrap(), Bytes20::new(expected));
    }

    #[test]
    fn into_bytes_20_wrong_type_errors() {
        let val = Value::Bool(true);
        assert!(val.into_bytes_20().is_err());
    }

    #[test]
    fn to_bytes_20_from_bytes20() {
        let data = [4u8; 20];
        let val = Value::Bytes20(data);
        assert_eq!(val.to_bytes_20().unwrap(), Bytes20::new(data));
    }

    #[test]
    fn to_bytes_20_from_bytes_correct_length() {
        let data = vec![5u8; 20];
        let val = Value::Bytes(data);
        assert_eq!(val.to_bytes_20().unwrap(), Bytes20::new([5u8; 20]));
    }

    #[test]
    fn to_bytes_20_from_text_b64_round_trip() {
        let data = [6u8; 20];
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let val = Value::Text(encoded);
        assert_eq!(val.to_bytes_20().unwrap(), Bytes20::new(data));
    }

    #[test]
    fn to_bytes_20_wrong_type_errors() {
        let val = Value::Null;
        assert!(val.to_bytes_20().is_err());
    }

    // ---------------------------------------------------------------
    // into_bytes_32 / to_bytes_32
    // ---------------------------------------------------------------

    #[test]
    fn into_bytes_32_from_bytes32() {
        let data = [1u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.into_bytes_32().unwrap(), Bytes32::new(data));
    }

    #[test]
    fn into_bytes_32_from_bytes_correct_length() {
        let data = vec![2u8; 32];
        let val = Value::Bytes(data);
        assert_eq!(val.into_bytes_32().unwrap(), Bytes32::new([2u8; 32]));
    }

    #[test]
    fn into_bytes_32_from_bytes_wrong_length_errors() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert!(val.into_bytes_32().is_err());
    }

    #[test]
    fn into_bytes_32_from_identifier() {
        let data = [3u8; 32];
        let val = Value::Identifier(data);
        assert_eq!(val.into_bytes_32().unwrap(), Bytes32::new(data));
    }

    #[test]
    fn into_bytes_32_from_text_b64_round_trip() {
        let data = [4u8; 32];
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let val = Value::Text(encoded);
        assert_eq!(val.into_bytes_32().unwrap(), Bytes32::new(data));
    }

    #[test]
    fn into_bytes_32_from_text_b64_wrong_length() {
        let val = Value::Text("AQID".to_string());
        assert!(val.into_bytes_32().is_err());
    }

    #[test]
    fn into_bytes_32_from_text_invalid_b64_errors() {
        let val = Value::Text("a811Ii".to_string());
        assert!(val.into_bytes_32().is_err());
    }

    #[test]
    fn into_bytes_32_from_array_of_32_u8s() {
        let arr: Vec<Value> = (0..32).map(|i| Value::U8(i as u8)).collect();
        let val = Value::Array(arr);
        let expected: [u8; 32] = core::array::from_fn(|i| i as u8);
        assert_eq!(val.into_bytes_32().unwrap(), Bytes32::new(expected));
    }

    #[test]
    fn into_bytes_32_wrong_type_errors() {
        let val = Value::Bool(true);
        assert!(val.into_bytes_32().is_err());
    }

    #[test]
    fn to_bytes_32_from_bytes32() {
        let data = [5u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.to_bytes_32().unwrap(), Bytes32::new(data));
    }

    #[test]
    fn to_bytes_32_from_identifier() {
        let data = [6u8; 32];
        let val = Value::Identifier(data);
        assert_eq!(val.to_bytes_32().unwrap(), Bytes32::new(data));
    }

    #[test]
    fn to_bytes_32_from_bytes_correct_length() {
        let data = vec![7u8; 32];
        let val = Value::Bytes(data);
        assert_eq!(val.to_bytes_32().unwrap(), Bytes32::new([7u8; 32]));
    }

    #[test]
    fn to_bytes_32_from_text_b64_round_trip() {
        let data = [8u8; 32];
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let val = Value::Text(encoded);
        assert_eq!(val.to_bytes_32().unwrap(), Bytes32::new(data));
    }

    #[test]
    fn to_bytes_32_wrong_type_errors() {
        let val = Value::Float(1.0);
        assert!(val.to_bytes_32().is_err());
    }

    // ---------------------------------------------------------------
    // into_bytes_36 / to_bytes_36
    // ---------------------------------------------------------------

    #[test]
    fn into_bytes_36_from_bytes36() {
        let data = [1u8; 36];
        let val = Value::Bytes36(data);
        assert_eq!(val.into_bytes_36().unwrap(), Bytes36::new(data));
    }

    #[test]
    fn into_bytes_36_from_bytes_correct_length() {
        let data = vec![2u8; 36];
        let val = Value::Bytes(data);
        assert_eq!(val.into_bytes_36().unwrap(), Bytes36::new([2u8; 36]));
    }

    #[test]
    fn into_bytes_36_from_bytes_wrong_length_errors() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert!(val.into_bytes_36().is_err());
    }

    #[test]
    fn into_bytes_36_from_text_b64_round_trip() {
        let data = [3u8; 36];
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let val = Value::Text(encoded);
        assert_eq!(val.into_bytes_36().unwrap(), Bytes36::new(data));
    }

    #[test]
    fn into_bytes_36_from_text_b64_wrong_length() {
        let val = Value::Text("AQID".to_string());
        assert!(val.into_bytes_36().is_err());
    }

    #[test]
    fn into_bytes_36_from_text_invalid_b64_errors() {
        let val = Value::Text("a811Ii".to_string());
        assert!(val.into_bytes_36().is_err());
    }

    #[test]
    fn into_bytes_36_from_array_of_36_u8s() {
        let arr: Vec<Value> = (0..36).map(|i| Value::U8(i as u8)).collect();
        let val = Value::Array(arr);
        let expected: [u8; 36] = core::array::from_fn(|i| i as u8);
        assert_eq!(val.into_bytes_36().unwrap(), Bytes36::new(expected));
    }

    #[test]
    fn into_bytes_36_wrong_type_errors() {
        let val = Value::Bool(true);
        assert!(val.into_bytes_36().is_err());
    }

    #[test]
    fn to_bytes_36_from_bytes36() {
        let data = [4u8; 36];
        let val = Value::Bytes36(data);
        assert_eq!(val.to_bytes_36().unwrap(), Bytes36::new(data));
    }

    #[test]
    fn to_bytes_36_from_bytes_correct_length() {
        let data = vec![5u8; 36];
        let val = Value::Bytes(data);
        assert_eq!(val.to_bytes_36().unwrap(), Bytes36::new([5u8; 36]));
    }

    #[test]
    fn to_bytes_36_from_text_b64_round_trip() {
        let data = [6u8; 36];
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let val = Value::Text(encoded);
        assert_eq!(val.to_bytes_36().unwrap(), Bytes36::new(data));
    }

    #[test]
    fn to_bytes_36_wrong_type_errors() {
        let val = Value::Null;
        assert!(val.to_bytes_36().is_err());
    }

    // ---------------------------------------------------------------
    // into_identifier / to_identifier
    // ---------------------------------------------------------------

    #[test]
    fn into_identifier_from_identifier() {
        let data = [1u8; 32];
        let val = Value::Identifier(data);
        assert_eq!(val.into_identifier().unwrap(), Identifier::new(data));
    }

    #[test]
    fn into_identifier_from_bytes32() {
        let data = [2u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.into_identifier().unwrap(), Identifier::new(data));
    }

    #[test]
    fn into_identifier_from_bytes_correct_length() {
        let data = vec![3u8; 32];
        let val = Value::Bytes(data);
        assert_eq!(val.into_identifier().unwrap(), Identifier::new([3u8; 32]));
    }

    #[test]
    fn into_identifier_from_bytes_wrong_length_errors() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert!(val.into_identifier().is_err());
    }

    #[test]
    fn into_identifier_from_text_bs58_round_trip() {
        let data = [4u8; 32];
        let encoded = bs58::encode(data).into_string();
        let val = Value::Text(encoded);
        assert_eq!(val.into_identifier().unwrap(), Identifier::new(data));
    }

    #[test]
    fn into_identifier_from_text_bs58_wrong_length() {
        let val = Value::Text("a811".to_string());
        assert!(val.into_identifier().is_err());
    }

    #[test]
    fn into_identifier_from_text_invalid_bs58() {
        let val = Value::Text("a811Ii".to_string());
        assert!(val.into_identifier().is_err());
    }

    #[test]
    fn into_identifier_from_array_of_32_u8s() {
        let arr: Vec<Value> = (0..32).map(|i| Value::U8(i as u8)).collect();
        let val = Value::Array(arr);
        let expected: [u8; 32] = core::array::from_fn(|i| i as u8);
        assert_eq!(val.into_identifier().unwrap(), Identifier::new(expected));
    }

    #[test]
    fn into_identifier_from_array_wrong_length() {
        let val = Value::Array(vec![Value::U8(1), Value::U8(2)]);
        assert!(val.into_identifier().is_err());
    }

    #[test]
    fn into_identifier_wrong_type_errors() {
        let val = Value::Bool(true);
        assert!(val.into_identifier().is_err());
    }

    #[test]
    fn to_identifier_from_identifier() {
        let data = [5u8; 32];
        let val = Value::Identifier(data);
        assert_eq!(val.to_identifier().unwrap(), Identifier::new(data));
    }

    #[test]
    fn to_identifier_from_bytes32() {
        let data = [6u8; 32];
        let val = Value::Bytes32(data);
        assert_eq!(val.to_identifier().unwrap(), Identifier::new(data));
    }

    #[test]
    fn to_identifier_from_bytes_correct_length() {
        let data = vec![7u8; 32];
        let val = Value::Bytes(data);
        assert_eq!(val.to_identifier().unwrap(), Identifier::new([7u8; 32]));
    }

    #[test]
    fn to_identifier_from_text_bs58_round_trip() {
        let data = [8u8; 32];
        let encoded = bs58::encode(data).into_string();
        let val = Value::Text(encoded);
        assert_eq!(val.to_identifier().unwrap(), Identifier::new(data));
    }

    #[test]
    fn to_identifier_from_array_of_32_u8s() {
        let arr: Vec<Value> = (0..32).map(|i| Value::U8(i as u8)).collect();
        let val = Value::Array(arr.clone());
        let expected: [u8; 32] = core::array::from_fn(|i| i as u8);
        assert_eq!(val.to_identifier().unwrap(), Identifier::new(expected));
    }

    #[test]
    fn to_identifier_wrong_type_errors() {
        let val = Value::Float(1.0);
        assert!(val.to_identifier().is_err());
    }
}
