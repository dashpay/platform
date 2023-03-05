use crate::{Error, Value};

impl Value {
    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111]);
    /// assert_eq!(value.into_system_bytes(), Ok(vec![104, 101, 108, 108, 111]));    ///
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_system_bytes(), Ok(vec![98, 155, 36]));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108)]);
    /// assert_eq!(value.into_system_bytes(), Ok(vec![104, 101, 108]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_system_bytes(), Ok(vec![5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_system_bytes(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_system_bytes(self) -> Result<Vec<u8>, Error> {
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
    /// assert_eq!(value.to_system_bytes(), Ok(vec![104, 101, 108, 108, 111]));    ///
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.to_system_bytes(), Ok(vec![98, 155, 36]));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108)]);
    /// assert_eq!(value.to_system_bytes(), Ok(vec![104, 101, 108]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.to_system_bytes(), Ok(vec![5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_system_bytes(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn to_system_bytes(&self) -> Result<Vec<u8>, Error> {
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

    /// If the `Value` is a `Bytes`, a `Text` using base 58 or Vector of `U8`, returns the
    /// associated `Vec<u8>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use platform_value::{Error, Value};
    /// #
    /// let value = Value::Bytes(vec![104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]);
    /// assert_eq!(value.into_system_hash256(), Ok([104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50, 104, 101, 108, 108, 111, 32, 12, 50]));    ///
    ///
    /// let value = Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string());
    /// assert_eq!(value.into_system_hash256(), Ok([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117]));
    ///
    /// let value = Value::Text("a811".to_string());
    /// assert_eq!(value.into_system_hash256(), Err(Error::StructureError("value was a string, could be decoded from base 58, but was not 32 bytes long".to_string())));
    ///
    /// let value = Value::Text("a811Ii".to_string());
    /// assert_eq!(value.into_system_hash256(), Err(Error::StructureError("value was a string, but could not be decoded from base 58".to_string())));
    ///
    /// let value = Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)]);
    /// assert_eq!(value.into_system_hash256(), Ok([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101]));
    ///
    /// let value = Value::Identifier([5u8;32]);
    /// assert_eq!(value.into_system_hash256(), Ok([5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5,5, 5, 5,5,5,5,5,5]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_system_hash256(), Err(Error::StructureError("value are not bytes, a string, or an array of values representing bytes".to_string())));
    /// ```
    pub fn into_system_hash256(self) -> Result<[u8; 32], Error> {
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
}
