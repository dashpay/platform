use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::{alphabet, Engine};

use crate::{BinaryData, Bytes20, Bytes32, Bytes36, Error, Identifier, Value};

/// The longest base 58 text a 32-byte identifier is written in. A longer text never decodes to
/// one, and decoding base 58 takes time quadratic in the text's length.
pub const MAX_BASE58_IDENTIFIER_LENGTH: usize = 44;

/// Whether `byte` is a character of the base 58 alphabet `bs58` decodes by default, the digits
/// and letters but `0`, `O`, `I` and `l`: decoding refuses a text exactly when it holds another.
fn is_base58_character(byte: u8) -> bool {
    matches!(byte, b'1'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'a'..=b'k' | b'm'..=b'z')
}

/// What [`Value::to_identifier_bytes`] refuses a text that does not decode from base 58 with.
fn not_base58_error() -> Error {
    Error::StructureError("value was a string, but could not be decoded from base 58".to_string())
}

/// The 32 bytes a base 58 `text` decodes to, refused as the 32-byte readers refuse it: when it
/// does not decode, and when it decodes to another length. A text longer than
/// [`MAX_BASE58_IDENTIFIER_LENGTH`] decodes to more than 32 bytes when it decodes at all, and
/// decoding fails exactly when a character is outside the base 58 alphabet, so such a text is
/// refused by its characters alone, with the same error, without decoding it: base 58 decoding
/// takes time quadratic in the text's length.
fn base58_32_bytes<T: TryFrom<Vec<u8>>>(text: &str) -> Result<T, Error> {
    let not_32_bytes = || {
        Error::StructureError(
            "value was a string, could be decoded from base 58, but was not 32 bytes long"
                .to_string(),
        )
    };
    if text.len() > MAX_BASE58_IDENTIFIER_LENGTH {
        return Err(if text.bytes().all(is_base58_character) {
            not_32_bytes()
        } else {
            not_base58_error()
        });
    }
    bs58::decode(text)
        .into_vec()
        .map_err(|_| not_base58_error())?
        .try_into()
        .map_err(|_| not_32_bytes())
}

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
            Value::Text(text) => bs58::decode(text)
                .into_vec()
                .map_err(|_| not_base58_error()),
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

    /// [`Value::to_identifier_bytes`] for a caller that refuses what any text longer than
    /// `max_text_len` bytes decodes to, without decoding such a text: base 58 decoding takes
    /// time quadratic in the text's length. Decoding fails exactly when a character is outside
    /// the base 58 alphabet, so such a text is `Ok(None)` when every character is in it, and is
    /// refused with the error decoding it gives otherwise. Every other value, and a text no
    /// longer than `max_text_len`, is read as `to_identifier_bytes` reads it.
    ///
    /// ```
    /// # use platform_value::Value;
    /// #
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_identifier_bytes_unless_text_longer_than(44), Ok(Some(vec![98, 155, 36])));
    ///
    /// let value = Value::Text("2".repeat(45));
    /// assert_eq!(value.to_identifier_bytes_unless_text_longer_than(44), Ok(None));
    ///
    /// let value = Value::Text(format!("{}0", "2".repeat(44)));
    /// assert_eq!(
    ///     value.to_identifier_bytes_unless_text_longer_than(44),
    ///     value.to_identifier_bytes().map(Some)
    /// );
    /// ```
    pub fn to_identifier_bytes_unless_text_longer_than(
        &self,
        max_text_len: usize,
    ) -> Result<Option<Vec<u8>>, Error> {
        match self {
            Value::Text(text) if text.len() > max_text_len => {
                if text.bytes().all(is_base58_character) {
                    Ok(None)
                } else {
                    Err(not_base58_error())
                }
            }
            value => value.to_identifier_bytes().map(Some),
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
            Value::Text(text) => base58_32_bytes(&text),
            Value::Array(array) => Ok(array
                .into_iter()
                .map(|byte| match byte {
                    Value::U8(value_as_u8) => Ok(value_as_u8),
                    _ => Err(Error::StructureError("not an array of bytes".to_string())),
                })
                .collect::<Result<Vec<u8>, Error>>()?
                .try_into()
                .map_err(|_| {
                    Error::StructureError(
                        "value was an array of bytes, but was not 32 bytes long".to_string(),
                    )
                })?),
            Value::Bytes(vec) => vec.try_into().map_err(|_| {
                Error::StructureError("value was bytes, but was not 32 bytes long".to_string())
            }),
            Value::Bytes32(bytes) => Ok(bytes),
            Value::Identifier(identifier) => Ok(identifier),
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
            Value::Text(text) => base58_32_bytes(text),
            Value::Array(array) => Ok(array
                .iter()
                .map(|byte| byte.to_integer())
                .collect::<Result<Vec<u8>, Error>>()?
                .try_into()
                .map_err(|_| {
                    Error::StructureError(
                        "value was an array of bytes, but was not 32 bytes long".to_string(),
                    )
                })?),
            Value::Bytes32(bytes) => Ok(*bytes),
            Value::Bytes(vec) => vec.clone().try_into().map_err(|_| {
                Error::StructureError("value was bytes, but was not 32 bytes long".to_string())
            }),
            Value::Identifier(identifier) => Ok(identifier.to_owned()),
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
            Value::Text(text) => base58_32_bytes(&text),
            Value::Array(array) => Ok(array
                .into_iter()
                .map(|byte| match byte {
                    Value::U8(value_as_u8) => Ok(value_as_u8),
                    _ => Err(Error::StructureError("not an array of bytes".to_string())),
                })
                .collect::<Result<Vec<u8>, Error>>()?
                .try_into()
                .map_err(|_| {
                    Error::StructureError(
                        "value was an array of bytes, but was not 32 bytes long".to_string(),
                    )
                })?),
            Value::Bytes(vec) => vec.try_into().map_err(|_| {
                Error::StructureError("value was bytes, but was not 32 bytes long".to_string())
            }),
            Value::Bytes32(bytes) => Ok(Identifier::new(bytes)),
            Value::Identifier(identifier) => Ok(Identifier::new(identifier)),
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
            Value::Text(text) => base58_32_bytes(text),
            Value::Array(array) => Ok(array
                .iter()
                .map(|byte| byte.to_integer())
                .collect::<Result<Vec<u8>, Error>>()?
                .try_into()
                .map_err(|_| {
                    Error::StructureError(
                        "value was an array of bytes, but was not 32 bytes long".to_string(),
                    )
                })?),
            Value::Bytes32(bytes) => Ok(Identifier::new(*bytes)),
            Value::Bytes(vec) => vec.clone().try_into().map_err(|_| {
                Error::StructureError("value was bytes, but was not 32 bytes long".to_string())
            }),
            Value::Identifier(identifier) => Ok(Identifier::new(*identifier)),
            _other => Err(Error::StructureError(
                "value are not bytes, a string, or an array of values representing bytes"
                    .to_string(),
            )),
        }
    }

    /// [`Value::to_identifier`], refusing a text longer than any identifier's base 58 form
    /// ([`MAX_BASE58_IDENTIFIER_LENGTH`]) with an error of its own. Every other value reads as
    /// `to_identifier` reads it, and `to_identifier` refuses such a text too, without decoding
    /// it either, with the error a full decode gives.
    pub fn to_bounded_identifier(&self) -> Result<Identifier, Error> {
        match self {
            Value::Text(text) if text.len() > MAX_BASE58_IDENTIFIER_LENGTH => {
                Err(Error::StructureError(
                    "value was a string too long to be a base 58 identifier".to_string(),
                ))
            }
            value => value.to_identifier(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // to_bounded_identifier
    // ---------------------------------------------------------------

    #[test]
    fn should_read_every_identifier_text_as_to_identifier_does() {
        for bytes in [[0u8; 32], [5u8; 32], [255u8; 32]] {
            let text = bs58::encode(bytes).into_string();
            assert!(text.len() <= MAX_BASE58_IDENTIFIER_LENGTH);
            let value = Value::Text(text);
            assert_eq!(value.to_bounded_identifier(), Ok(Identifier::new(bytes)));
            assert_eq!(value.to_bounded_identifier(), value.to_identifier());
        }
        assert_eq!(
            bs58::encode([255u8; 32]).into_string().len(),
            MAX_BASE58_IDENTIFIER_LENGTH
        );
        let short = Value::Text("a811".to_string());
        assert_eq!(short.to_bounded_identifier(), short.to_identifier());
        let identifier = Value::Identifier([7; 32]);
        assert_eq!(
            identifier.to_bounded_identifier(),
            identifier.to_identifier()
        );
    }

    #[test]
    fn should_refuse_a_text_too_long_to_be_an_identifier_without_decoding_it() {
        let too_long = Value::Text("2".repeat(MAX_BASE58_IDENTIFIER_LENGTH + 1));
        assert!(too_long.to_identifier().is_err());
        assert_eq!(
            too_long.to_bounded_identifier(),
            Err(Error::StructureError(
                "value was a string too long to be a base 58 identifier".to_string()
            ))
        );
        assert!(Value::Text("2".repeat(19 * 1024))
            .to_bounded_identifier()
            .is_err());
    }

    // ---------------------------------------------------------------
    // to_identifier_bytes_unless_text_longer_than
    // ---------------------------------------------------------------

    #[test]
    fn should_take_a_character_for_base_58_exactly_when_bs58_decodes_it() {
        for byte in 0..=u8::MAX {
            assert_eq!(
                is_base58_character(byte),
                bs58::decode([byte]).into_vec().is_ok(),
                "{byte}"
            );
        }
    }

    /// A text of n characters, z of them leading `1`s, decodes to z zero bytes and the bytes of
    /// a number of at least 58^(n - z - 1), whose first other digit is at least 1. A `1` takes a
    /// byte of its own and any other character less than one, so the text of each z decoding to
    /// the fewest bytes is `1` z times, `2`, then `1`s: none of them is 32 bytes past 44
    /// characters, so no text longer than [`MAX_BASE58_IDENTIFIER_LENGTH`] is an identifier.
    #[test]
    fn should_decode_no_text_longer_than_an_identifiers_to_32_bytes() {
        let length = MAX_BASE58_IDENTIFIER_LENGTH + 1;
        for leading_ones in 0..=length {
            let text = if leading_ones == length {
                "1".repeat(length)
            } else {
                format!(
                    "{}2{}",
                    "1".repeat(leading_ones),
                    "1".repeat(length - leading_ones - 1)
                )
            };
            let decoded = bs58::decode(&text).into_vec().expect("a base 58 text");
            assert!(decoded.len() > 32, "{text} decodes to {}", decoded.len());
        }
    }

    #[test]
    fn should_read_every_value_as_to_identifier_bytes_reads_it_up_to_the_bound() {
        for value in [
            Value::Text(bs58::encode([0xFF; 32]).into_string()),
            Value::Text("a811".to_string()),
            Value::Text("a811Ii".to_string()),
            Value::Text(String::new()),
            Value::Array(vec![Value::U8(1), Value::U8(2)]),
            Value::Array(vec![Value::U64(1)]),
            Value::Bytes(vec![1, 2, 3]),
            Value::Bytes32([4; 32]),
            Value::Identifier([5; 32]),
            Value::Null,
            Value::Bool(true),
        ] {
            assert_eq!(
                value.to_identifier_bytes_unless_text_longer_than(MAX_BASE58_IDENTIFIER_LENGTH),
                value.to_identifier_bytes().map(Some),
                "{value:?}"
            );
        }
    }

    /// A text read as 32 bytes by a full decode, refused when the decode fails or gives another
    /// length.
    fn decoded_32_bytes(text: &str) -> Result<[u8; 32], Error> {
        bs58::decode(text)
            .into_vec()
            .map_err(|_| not_base58_error())?
            .try_into()
            .map_err(|_| {
                Error::StructureError(
                    "value was a string, could be decoded from base 58, but was not 32 bytes long"
                        .to_string(),
                )
            })
    }

    /// The 32-byte readers read every text as a full decode does: the longest identifier's
    /// text, one character more (`2` then `1`s, the 45 characters decoding to the fewest
    /// bytes), texts holding a character outside the alphabet, short texts, and long ones.
    #[test]
    fn should_read_a_text_as_32_bytes_as_a_full_decode_does() {
        for text in [
            bs58::encode([0xFF; 32]).into_string(),
            bs58::encode([0x00; 32]).into_string(),
            format!("2{}", "1".repeat(MAX_BASE58_IDENTIFIER_LENGTH)),
            format!("{}0", "2".repeat(MAX_BASE58_IDENTIFIER_LENGTH)),
            "a811".to_string(),
            "a811Ii".to_string(),
            String::new(),
            "2".repeat(2_000),
            format!("{}l", "2".repeat(1_999)),
        ] {
            let expected = decoded_32_bytes(&text);
            let value = Value::Text(text.clone());
            assert_eq!(value.to_hash256(), expected, "{text}");
            assert_eq!(value.clone().into_hash256(), expected, "{text}");
            assert_eq!(
                value.to_identifier(),
                expected.clone().map(Identifier::new),
                "{text}"
            );
            assert_eq!(
                value.into_identifier(),
                expected.map(Identifier::new),
                "{text}"
            );
        }
    }

    /// A 32-byte reader refuses a text too long to be 32 bytes in time linear in its length, as
    /// a full decode refuses it, base 58 or not.
    #[test]
    fn should_refuse_a_long_text_as_32_bytes_without_decoding_it() {
        for text in ["2".repeat(19_500), format!("{}0", "2".repeat(19_499))] {
            let expected = decoded_32_bytes(&text);
            assert!(expected.is_err());
            let value = Value::Text(text);
            let elapsed = (0..3)
                .map(|_| {
                    let started = std::time::Instant::now();
                    assert_eq!(value.to_hash256(), expected);
                    assert_eq!(value.to_identifier(), expected.clone().map(Identifier::new));
                    started.elapsed()
                })
                .min()
                .expect("three runs");
            assert!(
                elapsed < std::time::Duration::from_millis(10),
                "{elapsed:?}"
            );
        }
    }

    /// Past the bound a text is not decoded: a base 58 one is told apart from one decoding
    /// refuses by its characters alone, in time linear in its length.
    #[test]
    fn should_tell_a_text_past_the_bound_without_decoding_it() {
        let base_58 = Value::Text("2".repeat(19_500));
        let not_base_58 = Value::Text(format!("{}0", "2".repeat(19_499)));
        assert!(base_58.to_identifier_bytes().is_ok());
        let decoding_error = not_base_58.to_identifier_bytes();
        assert!(decoding_error.is_err());
        for (value, expected) in [(base_58, Ok(None)), (not_base_58, decoding_error.map(Some))] {
            let elapsed = (0..3)
                .map(|_| {
                    let started = std::time::Instant::now();
                    assert_eq!(
                        value.to_identifier_bytes_unless_text_longer_than(
                            MAX_BASE58_IDENTIFIER_LENGTH
                        ),
                        expected
                    );
                    started.elapsed()
                })
                .min()
                .expect("three runs");
            assert!(
                elapsed < std::time::Duration::from_millis(10),
                "{elapsed:?}"
            );
        }
    }

    // ---------------------------------------------------------------
    // into_identifier_bytes / to_identifier_bytes  (base-58 encoding)
    // ---------------------------------------------------------------

    #[test]
    fn into_identifier_bytes_from_bytes() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert_eq!(val.into_identifier_bytes().unwrap(), vec![1, 2, 3]);
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
