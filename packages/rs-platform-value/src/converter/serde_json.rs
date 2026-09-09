use crate::value_map::ValueMap;
use crate::{Error, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::{Map, Number, Value as JsonValue};
use std::collections::BTreeMap;

impl Value {
    pub fn convert_from_serde_json_map<I, R>(map: I) -> R
    where
        I: IntoIterator<Item = (String, JsonValue)>,
        R: FromIterator<(String, Value)>,
    {
        map.into_iter()
            .map(|(key, serde_json_value)| (key, serde_json_value.into()))
            .collect()
    }

    pub fn try_into_validating_json(self) -> Result<JsonValue, Error> {
        Ok(match self {
            Value::U128(i) => {
                if i > u64::MAX as u128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((i as u64).into())
            }
            Value::I128(i) => {
                if i > i64::MAX as i128 {
                    return Err(Error::IntegerSizeError);
                }
                if i < i64::MIN as i128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((i as i64).into())
            }
            Value::U64(i) => JsonValue::Number(i.into()),
            Value::I64(i) => JsonValue::Number(i.into()),
            Value::U32(i) => JsonValue::Number(i.into()),
            Value::I32(i) => JsonValue::Number(i.into()),
            Value::U16(i) => JsonValue::Number(i.into()),
            Value::I16(i) => JsonValue::Number(i.into()),
            Value::U8(i) => JsonValue::Number(i.into()),
            Value::I8(i) => JsonValue::Number(i.into()),
            // JSON cannot represent NaN/±∞ (`from_f64` returns None) — fail
            // loudly instead of silently substituting 0, which would be
            // indistinguishable from a real zero to JSON consumers.
            Value::Float(float) => JsonValue::Number(Number::from_f64(float).ok_or_else(|| {
                Error::Unsupported("non-finite float (NaN/±∞) is not representable in JSON".into())
            })?),
            Value::Text(string) => JsonValue::String(string),
            Value::Bool(value) => JsonValue::Bool(value),
            Value::Null => JsonValue::Null,
            Value::Array(array) => JsonValue::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into_validating_json())
                    .collect::<Result<Vec<JsonValue>, Error>>()?,
            ),
            Value::Map(map) => JsonValue::Object(
                map.into_iter()
                    .map(|(k, v)| {
                        let string = k.into_text()?;
                        Ok((string, v.try_into_validating_json()?))
                    })
                    .collect::<Result<Map<String, JsonValue>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                // In order to be able to validate using JSON schema it needs to be in byte form
                JsonValue::Array(
                    bytes
                        .into_iter()
                        .map(|a| JsonValue::Number(a.into()))
                        .collect(),
                )
            }
            Value::Bytes(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes20(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes32(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes36(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::EnumU8(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumU8 to JSONValue".to_string(),
                ))
            }
            Value::EnumString(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumString to JSONValue".to_string(),
                ))
            }
        })
    }

    pub fn try_into_validating_btree_map_json(self) -> Result<BTreeMap<String, JsonValue>, Error> {
        self.into_btree_string_map()?
            .into_iter()
            .map(|(key, value)| Ok((key, value.try_into_validating_json()?)))
            .collect()
    }

    pub fn try_to_validating_json(&self) -> Result<JsonValue, Error> {
        Ok(match self {
            Value::U128(i) => {
                if *i > u64::MAX as u128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((*i as u64).into())
            }
            Value::I128(i) => {
                if *i > i64::MAX as i128 {
                    return Err(Error::IntegerSizeError);
                }
                if *i < i64::MIN as i128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((*i as i64).into())
            }
            Value::U64(i) => JsonValue::Number((*i).into()),
            Value::I64(i) => JsonValue::Number((*i).into()),
            Value::U32(i) => JsonValue::Number((*i).into()),
            Value::I32(i) => JsonValue::Number((*i).into()),
            Value::U16(i) => JsonValue::Number((*i).into()),
            Value::I16(i) => JsonValue::Number((*i).into()),
            Value::U8(i) => JsonValue::Number((*i).into()),
            Value::I8(i) => JsonValue::Number((*i).into()),
            // JSON cannot represent NaN/±∞ — fail loudly (see owned path above).
            Value::Float(float) => {
                JsonValue::Number(Number::from_f64(*float).ok_or_else(|| {
                    Error::Unsupported(
                        "non-finite float (NaN/±∞) is not representable in JSON".into(),
                    )
                })?)
            }
            Value::Text(string) => JsonValue::String(string.clone()),
            Value::Bool(value) => JsonValue::Bool(*value),
            Value::Null => JsonValue::Null,
            Value::Array(array) => JsonValue::Array(
                array
                    .iter()
                    .map(|value| value.try_to_validating_json())
                    .collect::<Result<Vec<JsonValue>, Error>>()?,
            ),
            Value::Map(map) => JsonValue::Object(
                map.iter()
                    .map(|(k, v)| {
                        let string = k.to_text()?;
                        Ok((string, v.try_to_validating_json()?))
                    })
                    .collect::<Result<Map<String, JsonValue>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                // In order to be able to validate using JSON schema it needs to be in byte form
                JsonValue::Array(
                    bytes
                        .iter()
                        .map(|a| JsonValue::Number((*a).into()))
                        .collect(),
                )
            }
            Value::Bytes(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes20(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes32(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes36(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
                    .collect(),
            ),
            Value::EnumU8(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumU8 to JSONValue".to_string(),
                ))
            }
            Value::EnumString(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumString to JSONValue".to_string(),
                ))
            }
        })
    }
}

impl From<JsonValue> for Value {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(value) => Self::Bool(value),
            JsonValue::Number(number) => {
                if let Some(value) = number.as_u64() {
                    return Self::U64(value);
                } else if let Some(value) = number.as_i64() {
                    return Self::I64(value);
                } else if let Some(value) = number.as_f64() {
                    return Self::Float(value);
                }
                unreachable!("this shouldn't be reachable")
            }
            JsonValue::String(string) => Self::Text(string),
            JsonValue::Array(array) => {
                // Critical-2 fix: faithful array → array conversion. The previous
                // heuristic ("if len >= 10 and all elements ≤ 255 then call it
                // bytes") was a JS-DPP-era workaround for clients that sent
                // binary as JSON arrays of u8. With the canonical-trait
                // unification (HR = base64 strings, non-HR = Value::Bytes;
                // BinaryData/Identifier/Bytes* deserializers handle both),
                // the heuristic is no longer needed and was actively corrupting
                // genuine arrays of small integers (e.g., a document property
                // typed as "list of small ints" of length 10+).
                Self::Array(array.into_iter().map(|v| v.into()).collect())
            }
            JsonValue::Object(map) => {
                Self::Map(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
        }
    }
}

impl From<&JsonValue> for Value {
    fn from(value: &JsonValue) -> Self {
        match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(value) => Self::Bool(*value),
            JsonValue::Number(number) => {
                if let Some(value) = number.as_u64() {
                    return Self::U64(value);
                } else if let Some(value) = number.as_i64() {
                    return Self::I64(value);
                } else if let Some(value) = number.as_f64() {
                    return Self::Float(value);
                }
                unreachable!("this shouldn't be reachable")
            }
            JsonValue::String(string) => Self::Text(string.clone()),
            JsonValue::Array(array) => {
                // Critical-2 fix: see owned-form comment above.
                Self::Array(array.iter().map(|v| v.into()).collect())
            }
            JsonValue::Object(map) => Self::Map(
                map.into_iter()
                    .map(|(k, v)| (k.clone().into(), v.into()))
                    .collect(),
            ),
        }
    }
}

impl TryInto<JsonValue> for Value {
    type Error = Error;

    fn try_into(self) -> Result<JsonValue, Self::Error> {
        Ok(match self {
            // U128/I128 can't fit in serde_json::Number (backed by u64/i64/f64) — must be strings
            Value::U128(i) => JsonValue::String(i.to_string()),
            Value::I128(i) => JsonValue::String(i.to_string()),
            Value::U64(i) => JsonValue::Number(i.into()),
            Value::I64(i) => JsonValue::Number(i.into()),
            Value::U32(i) => JsonValue::Number(i.into()),
            Value::I32(i) => JsonValue::Number(i.into()),
            Value::U16(i) => JsonValue::Number(i.into()),
            Value::I16(i) => JsonValue::Number(i.into()),
            Value::U8(i) => JsonValue::Number(i.into()),
            Value::I8(i) => JsonValue::Number(i.into()),
            Value::Bytes(bytes) => JsonValue::String(BASE64_STANDARD.encode(bytes.as_slice())),
            Value::Bytes20(bytes) => JsonValue::String(BASE64_STANDARD.encode(bytes.as_slice())),
            Value::Bytes32(bytes) => JsonValue::String(BASE64_STANDARD.encode(bytes.as_slice())),
            Value::Bytes36(bytes) => JsonValue::String(BASE64_STANDARD.encode(bytes.as_slice())),
            // JSON cannot represent NaN/±∞ — fail loudly (see validating path above).
            Value::Float(float) => JsonValue::Number(Number::from_f64(float).ok_or_else(|| {
                Error::Unsupported("non-finite float (NaN/±∞) is not representable in JSON".into())
            })?),
            Value::Text(string) => JsonValue::String(string),
            Value::Bool(value) => JsonValue::Bool(value),
            Value::Null => JsonValue::Null,
            Value::Array(array) => JsonValue::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into())
                    .collect::<Result<Vec<JsonValue>, Error>>()?,
            ),
            Value::Map(map) => JsonValue::Object(
                map.into_iter()
                    .map(|(k, v)| {
                        let string = k.into_text()?;
                        Ok((string, v.try_into()?))
                    })
                    .collect::<Result<Map<String, JsonValue>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                JsonValue::String(bs58::encode(bytes.as_slice()).into_string())
            }
            Value::EnumU8(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumU8 to JSONValue".to_string(),
                ))
            }
            Value::EnumString(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumString to JSONValue".to_string(),
                ))
            }
        })
    }
}

pub trait BTreeValueJsonConverter {
    fn into_json_value(self) -> Result<JsonValue, Error>;
    fn into_validating_json_value(self) -> Result<JsonValue, Error>;
    fn to_json_value(&self) -> Result<JsonValue, Error>;
    fn to_validating_json_value(&self) -> Result<JsonValue, Error>;
    fn from_json_value(value: JsonValue) -> Result<Self, Error>
    where
        Self: Sized;
}

impl BTreeValueJsonConverter for BTreeMap<String, Value> {
    fn into_json_value(self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.into_iter()
                .map(|(key, value)| Ok((key, value.try_into()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn into_validating_json_value(self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.into_iter()
                .map(|(key, value)| Ok((key, value.try_into_validating_json()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn to_json_value(&self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.iter()
                .map(|(key, value)| Ok((key.clone(), value.clone().try_into()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn to_validating_json_value(&self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.iter()
                .map(|(key, value)| Ok((key.to_owned(), value.try_to_validating_json()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn from_json_value(value: JsonValue) -> Result<Self, Error> {
        let platform_value: Value = value.into();
        platform_value.into_btree_string_map()
    }
}

impl From<BTreeMap<String, JsonValue>> for Value {
    fn from(value: BTreeMap<String, JsonValue>) -> Self {
        let map: ValueMap = value
            .into_iter()
            .map(|(key, json_value)| {
                let value: Value = json_value.into();
                (Value::Text(key), value)
            })
            .collect();
        Value::Map(map)
    }
}

impl From<&BTreeMap<String, JsonValue>> for Value {
    fn from(value: &BTreeMap<String, JsonValue>) -> Self {
        let map: ValueMap = value
            .iter()
            .map(|(key, json_value)| {
                let value: Value = json_value.into();
                (Value::Text(key.clone()), value)
            })
            .collect();
        Value::Map(map)
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use crate::converter::serde_json::BTreeValueJsonConverter;
    use crate::{Error, Value};
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use serde_json::{json, Value as JsonValue};
    use std::collections::BTreeMap;

    #[test]
    fn test_json_array() {
        let json = json!({
          "type": 5,
          "protocolVersion": 1,
          "revision": 0,
          "signature": "HxtcTSpRdACokorvpx/f4ezM40e0WtgW2GUvjiwNkHPwKDppkIoS2cirhqpZURlhDuYdu+E0KllbHNlYghcK9Bg=",
          "signaturePublicKeyId": 1,
          "addPublicKeys": [
            {
              "id": 0,
              "purpose": 0,
              "securityLevel": 0,
              "type": 0,
              "data": "Aya0WP8EhKQ6Dq+51sAnqdPah664X9CUciVJYAfvfTnX",
              "readOnly": false,
              "signature": "HxtcTSpRdACokorvpx/f4ezM40e0WtgW2GUvjiwNkHPwKDppkIoS2cirhqpZURlhDuYdu+E0KllbHNlYghcK9Bg="
            }
          ],
          "disablePublicKeys": [ 0 ],
          "identityId": "62DHhTfZV3NvUbXUha1mavLqSEy2GaWYja2qeTYNUhk"
        });

        let value: Value = json.into();
        let array = value
            .get_optional_array_slice("addPublicKeys")
            .expect("expected to get array slice")
            .unwrap();
        assert_eq!(array.len(), 1);
        assert!(array.first().unwrap().is_map());
        let array = value
            .get_optional_array_slice("disablePublicKeys")
            .expect("expected to get array slice")
            .unwrap();
        assert_eq!(array.len(), 1);
    }

    // -----------------------------------------------------------------------
    // try_into_validating_json — all Value variants
    // -----------------------------------------------------------------------

    #[test]
    fn validating_json_null() {
        let result = Value::Null.try_into_validating_json().unwrap();
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn validating_json_bool() {
        assert_eq!(
            Value::Bool(true).try_into_validating_json().unwrap(),
            JsonValue::Bool(true)
        );
        assert_eq!(
            Value::Bool(false).try_into_validating_json().unwrap(),
            JsonValue::Bool(false)
        );
    }

    #[test]
    fn validating_json_u8() {
        let result = Value::U8(42).try_into_validating_json().unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn validating_json_i8() {
        let result = Value::I8(-5).try_into_validating_json().unwrap();
        assert_eq!(result, json!(-5));
    }

    #[test]
    fn validating_json_u16() {
        let result = Value::U16(1000).try_into_validating_json().unwrap();
        assert_eq!(result, json!(1000));
    }

    #[test]
    fn validating_json_i16() {
        let result = Value::I16(-1000).try_into_validating_json().unwrap();
        assert_eq!(result, json!(-1000));
    }

    #[test]
    fn validating_json_u32() {
        let result = Value::U32(100_000).try_into_validating_json().unwrap();
        assert_eq!(result, json!(100_000));
    }

    #[test]
    fn validating_json_i32() {
        let result = Value::I32(-100_000).try_into_validating_json().unwrap();
        assert_eq!(result, json!(-100_000));
    }

    #[test]
    fn validating_json_u64() {
        let result = Value::U64(u64::MAX).try_into_validating_json().unwrap();
        assert_eq!(result, json!(u64::MAX));
    }

    #[test]
    fn validating_json_i64() {
        let result = Value::I64(i64::MIN).try_into_validating_json().unwrap();
        assert_eq!(result, json!(i64::MIN));
    }

    #[test]
    fn validating_json_float() {
        let result = Value::Float(3.14).try_into_validating_json().unwrap();
        assert_eq!(result, json!(3.14));
    }

    #[test]
    fn validating_json_text() {
        let result = Value::Text("hello".into())
            .try_into_validating_json()
            .unwrap();
        assert_eq!(result, json!("hello"));
    }

    #[test]
    fn validating_json_u128_fits_u64() {
        let val = u64::MAX as u128;
        let result = Value::U128(val).try_into_validating_json().unwrap();
        assert_eq!(result, json!(u64::MAX));
    }

    #[test]
    fn validating_json_u128_too_large() {
        let val = u64::MAX as u128 + 1;
        let err = Value::U128(val).try_into_validating_json().unwrap_err();
        assert_eq!(err, Error::IntegerSizeError);
    }

    #[test]
    fn validating_json_i128_fits_i64_positive() {
        let val = i64::MAX as i128;
        let result = Value::I128(val).try_into_validating_json().unwrap();
        assert_eq!(result, json!(i64::MAX));
    }

    #[test]
    fn validating_json_i128_fits_i64_negative() {
        let val = i64::MIN as i128;
        let result = Value::I128(val).try_into_validating_json().unwrap();
        assert_eq!(result, json!(i64::MIN));
    }

    #[test]
    fn validating_json_i128_too_large_positive() {
        let val = i64::MAX as i128 + 1;
        let err = Value::I128(val).try_into_validating_json().unwrap_err();
        assert_eq!(err, Error::IntegerSizeError);
    }

    #[test]
    fn validating_json_i128_too_small_negative() {
        let val = i64::MIN as i128 - 1;
        let err = Value::I128(val).try_into_validating_json().unwrap_err();
        assert_eq!(err, Error::IntegerSizeError);
    }

    #[test]
    fn validating_json_bytes() {
        let result = Value::Bytes(vec![1, 2, 3])
            .try_into_validating_json()
            .unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn validating_json_bytes20() {
        let bytes = [7u8; 20];
        let result = Value::Bytes20(bytes).try_into_validating_json().unwrap();
        let arr: Vec<JsonValue> = bytes.iter().map(|b| json!(*b)).collect();
        assert_eq!(result, JsonValue::Array(arr));
    }

    #[test]
    fn validating_json_bytes32() {
        let bytes = [9u8; 32];
        let result = Value::Bytes32(bytes).try_into_validating_json().unwrap();
        let arr: Vec<JsonValue> = bytes.iter().map(|b| json!(*b)).collect();
        assert_eq!(result, JsonValue::Array(arr));
    }

    #[test]
    fn validating_json_bytes36() {
        let bytes = [11u8; 36];
        let result = Value::Bytes36(bytes).try_into_validating_json().unwrap();
        let arr: Vec<JsonValue> = bytes.iter().map(|b| json!(*b)).collect();
        assert_eq!(result, JsonValue::Array(arr));
    }

    #[test]
    fn validating_json_identifier() {
        let bytes = [0xABu8; 32];
        let result = Value::Identifier(bytes).try_into_validating_json().unwrap();
        let arr: Vec<JsonValue> = bytes.iter().map(|b| json!(*b)).collect();
        assert_eq!(result, JsonValue::Array(arr));
    }

    #[test]
    fn validating_json_array_nested() {
        let val = Value::Array(vec![Value::U64(1), Value::Text("two".into())]);
        let result = val.try_into_validating_json().unwrap();
        assert_eq!(result, json!([1, "two"]));
    }

    #[test]
    fn validating_json_map() {
        let map = vec![
            (Value::Text("a".into()), Value::U64(1)),
            (Value::Text("b".into()), Value::Bool(true)),
        ];
        let val = Value::Map(map);
        let result = val.try_into_validating_json().unwrap();
        assert_eq!(result, json!({"a": 1, "b": true}));
    }

    #[test]
    fn validating_json_enum_u8_unsupported() {
        let err = Value::EnumU8(vec![1, 2])
            .try_into_validating_json()
            .unwrap_err();
        assert!(matches!(err, Error::Unsupported(_)));
    }

    #[test]
    fn validating_json_enum_string_unsupported() {
        let err = Value::EnumString(vec!["a".into()])
            .try_into_validating_json()
            .unwrap_err();
        assert!(matches!(err, Error::Unsupported(_)));
    }

    // -----------------------------------------------------------------------
    // From<JsonValue> for Value — all JSON variants
    // -----------------------------------------------------------------------

    #[test]
    fn from_json_null() {
        let val: Value = JsonValue::Null.into();
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn from_json_bool_true() {
        let val: Value = json!(true).into();
        assert_eq!(val, Value::Bool(true));
    }

    #[test]
    fn from_json_bool_false() {
        let val: Value = json!(false).into();
        assert_eq!(val, Value::Bool(false));
    }

    #[test]
    fn from_json_positive_integer() {
        let val: Value = json!(42).into();
        assert_eq!(val, Value::U64(42));
    }

    #[test]
    fn from_json_negative_integer() {
        let val: Value = json!(-7).into();
        assert_eq!(val, Value::I64(-7));
    }

    #[test]
    fn from_json_float() {
        let val: Value = json!(2.5).into();
        assert_eq!(val, Value::Float(2.5));
    }

    #[test]
    fn from_json_string() {
        let val: Value = json!("hello").into();
        assert_eq!(val, Value::Text("hello".into()));
    }

    #[test]
    fn from_json_object() {
        let val: Value = json!({"key": "value"}).into();
        assert!(val.is_map());
    }

    // -----------------------------------------------------------------------
    // From<JsonValue> for Value — array conversion is faithful
    //
    // The previous `len >= 10 && all u8-range` heuristic that silently
    // reclassified JSON arrays as `Value::Bytes` was removed. JSON arrays now
    // always become `Value::Array(...)` regardless of length / content
    // shape. Binary fields should flow through canonical encodings
    // (base64 strings in JSON, decoded by the receiver's Deserialize impl).
    // -----------------------------------------------------------------------

    #[test]
    fn from_json_array_10_u8_range_stays_array_not_bytes() {
        // Previously: heuristic silently coerced this to Value::Bytes.
        // Now: faithful array → array conversion.
        let arr: Vec<JsonValue> = (0u64..10).map(|i| json!(i)).collect();
        let val: Value = JsonValue::Array(arr).into();
        assert_eq!(
            val,
            Value::Array(vec![
                Value::U64(0),
                Value::U64(1),
                Value::U64(2),
                Value::U64(3),
                Value::U64(4),
                Value::U64(5),
                Value::U64(6),
                Value::U64(7),
                Value::U64(8),
                Value::U64(9),
            ])
        );
    }

    #[test]
    fn from_json_array_9_u8_range_stays_array() {
        let arr: Vec<JsonValue> = (0u64..9).map(|i| json!(i)).collect();
        let val: Value = JsonValue::Array(arr).into();
        assert!(matches!(val, Value::Array(_)));
    }

    #[test]
    fn from_json_array_mixed_types_stays_array() {
        let mut arr: Vec<JsonValue> = (0u64..10).map(|i| json!(i)).collect();
        arr.push(json!("not_a_number"));
        let val: Value = JsonValue::Array(arr).into();
        assert!(matches!(val, Value::Array(_)));
    }

    #[test]
    fn from_json_array_large_values_stays_array() {
        let arr: Vec<JsonValue> = (0u64..12).map(|i| json!(i * 100)).collect();
        let val: Value = JsonValue::Array(arr).into();
        assert!(matches!(val, Value::Array(_)));
    }

    #[test]
    fn from_json_array_all_255_stays_array_not_bytes() {
        // Previously: heuristic coerced this to Value::Bytes.
        // Now: faithful array → array.
        let arr: Vec<JsonValue> = vec![json!(255); 10];
        let val: Value = JsonValue::Array(arr).into();
        assert_eq!(val, Value::Array(vec![Value::U64(255); 10]));
    }

    #[test]
    fn from_json_array_with_negative_stays_array() {
        let mut arr: Vec<JsonValue> = (0u64..9).map(|i| json!(i)).collect();
        arr.push(json!(-1));
        let val: Value = JsonValue::Array(arr).into();
        assert!(matches!(val, Value::Array(_)));
    }

    #[test]
    fn from_json_long_byte_like_array_stays_array_not_bytes() {
        // Round-trip pin: a 1000-element JSON array of small ints (e.g., a
        // document property typed as `array of integers`) previously got
        // silently corrupted into Value::Bytes(1000 bytes). The roundtrip
        // back via `try_into::<JsonValue>` would then emit a base64 string
        // instead of the original array. After the Critical-2 fix, the
        // conversion is faithful in both directions.
        let original: JsonValue = JsonValue::Array(vec![json!(7u64); 1000]);
        let value: Value = original.clone().into();
        assert!(
            matches!(value, Value::Array(_)),
            "1000-element u8-range JSON array must stay an array, not silently \
             become bytes"
        );
        let recovered: JsonValue = value.try_into().unwrap();
        assert_eq!(original, recovered, "round-trip JSON array → Value → JSON");
    }

    // -----------------------------------------------------------------------
    // From<&JsonValue> for Value — reference variant
    // -----------------------------------------------------------------------

    #[test]
    fn from_json_ref_null() {
        let jv = JsonValue::Null;
        let val: Value = (&jv).into();
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn from_json_ref_array_stays_array_not_bytes() {
        // Mirrors `from_json_array_10_u8_range_stays_array_not_bytes`
        // for the borrowed `From<&JsonValue>` variant — same Critical-2
        // fix applies to both impls.
        let arr: Vec<JsonValue> = (0u64..15).map(|i| json!(i)).collect();
        let jv = JsonValue::Array(arr);
        let val: Value = (&jv).into();
        assert!(matches!(val, Value::Array(_)));
    }

    #[test]
    fn from_json_ref_array_short_stays_array() {
        let arr: Vec<JsonValue> = (0u64..5).map(|i| json!(i)).collect();
        let jv = JsonValue::Array(arr);
        let val: Value = (&jv).into();
        assert!(matches!(val, Value::Array(_)));
    }

    // -----------------------------------------------------------------------
    // TryInto<JsonValue> for Value — bytes become base64, identifiers become bs58
    // -----------------------------------------------------------------------

    #[test]
    fn try_into_json_bytes_become_base64() {
        let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let expected = BASE64_STANDARD.encode(&bytes);
        let result: JsonValue = Value::Bytes(bytes).try_into().unwrap();
        assert_eq!(result, JsonValue::String(expected));
    }

    #[test]
    fn try_into_json_bytes20_become_base64() {
        let bytes = [0xAAu8; 20];
        let expected = BASE64_STANDARD.encode(bytes);
        let result: JsonValue = Value::Bytes20(bytes).try_into().unwrap();
        assert_eq!(result, JsonValue::String(expected));
    }

    #[test]
    fn try_into_json_bytes32_become_base64() {
        let bytes = [0xBBu8; 32];
        let expected = BASE64_STANDARD.encode(bytes);
        let result: JsonValue = Value::Bytes32(bytes).try_into().unwrap();
        assert_eq!(result, JsonValue::String(expected));
    }

    #[test]
    fn try_into_json_bytes36_become_base64() {
        let bytes = [0xCCu8; 36];
        let expected = BASE64_STANDARD.encode(bytes);
        let result: JsonValue = Value::Bytes36(bytes).try_into().unwrap();
        assert_eq!(result, JsonValue::String(expected));
    }

    #[test]
    fn try_into_json_identifier_becomes_bs58() {
        let bytes = [0x01u8; 32];
        let expected = bs58::encode(&bytes).into_string();
        let result: JsonValue = Value::Identifier(bytes).try_into().unwrap();
        assert_eq!(result, JsonValue::String(expected));
    }

    #[test]
    fn try_into_json_u128_becomes_string() {
        let result: JsonValue = Value::U128(u128::MAX).try_into().unwrap();
        assert_eq!(result, JsonValue::String(u128::MAX.to_string()));
    }

    #[test]
    fn try_into_json_i128_becomes_string() {
        let result: JsonValue = Value::I128(i128::MIN).try_into().unwrap();
        assert_eq!(result, JsonValue::String(i128::MIN.to_string()));
    }

    #[test]
    fn try_into_json_null() {
        let result: JsonValue = Value::Null.try_into().unwrap();
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn try_into_json_bool() {
        let result: JsonValue = Value::Bool(true).try_into().unwrap();
        assert_eq!(result, JsonValue::Bool(true));
    }

    #[test]
    fn try_into_json_text() {
        let result: JsonValue = Value::Text("abc".into()).try_into().unwrap();
        assert_eq!(result, json!("abc"));
    }

    #[test]
    fn try_into_json_integer_types() {
        let r: JsonValue = Value::U8(1).try_into().unwrap();
        assert_eq!(r, json!(1));
        let r: JsonValue = Value::I8(-1).try_into().unwrap();
        assert_eq!(r, json!(-1));
        let r: JsonValue = Value::U16(500).try_into().unwrap();
        assert_eq!(r, json!(500));
        let r: JsonValue = Value::I16(-500).try_into().unwrap();
        assert_eq!(r, json!(-500));
        let r: JsonValue = Value::U32(70000).try_into().unwrap();
        assert_eq!(r, json!(70000));
        let r: JsonValue = Value::I32(-70000).try_into().unwrap();
        assert_eq!(r, json!(-70000));
        let r: JsonValue = Value::U64(123456789).try_into().unwrap();
        assert_eq!(r, json!(123456789));
        let r: JsonValue = Value::I64(-123456789).try_into().unwrap();
        assert_eq!(r, json!(-123456789));
    }

    #[test]
    fn try_into_json_array() {
        let val = Value::Array(vec![Value::U64(1), Value::Bool(false)]);
        let result: JsonValue = val.try_into().unwrap();
        assert_eq!(result, json!([1, false]));
    }

    #[test]
    fn try_into_json_map() {
        let map = vec![(Value::Text("x".into()), Value::U64(99))];
        let val = Value::Map(map);
        let result: JsonValue = val.try_into().unwrap();
        assert_eq!(result, json!({"x": 99}));
    }

    #[test]
    fn try_into_json_enum_u8_error() {
        let result: Result<JsonValue, Error> = Value::EnumU8(vec![1]).try_into();
        assert!(matches!(result, Err(Error::Unsupported(_))));
    }

    #[test]
    fn try_into_json_enum_string_error() {
        let result: Result<JsonValue, Error> = Value::EnumString(vec!["a".into()]).try_into();
        assert!(matches!(result, Err(Error::Unsupported(_))));
    }

    // -----------------------------------------------------------------------
    // Round-trip: Value -> JsonValue -> Value for basic types
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_null() {
        let original = Value::Null;
        let json: JsonValue = original.clone().try_into_validating_json().unwrap();
        let back: Value = json.into();
        assert_eq!(back, original);
    }

    #[test]
    fn round_trip_bool() {
        let original = Value::Bool(true);
        let json: JsonValue = original.clone().try_into_validating_json().unwrap();
        let back: Value = json.into();
        assert_eq!(back, Value::Bool(true));
    }

    #[test]
    fn round_trip_u64() {
        let original = Value::U64(42);
        let json: JsonValue = original.clone().try_into_validating_json().unwrap();
        let back: Value = json.into();
        // JSON numbers parse back as U64
        assert_eq!(back, Value::U64(42));
    }

    #[test]
    fn round_trip_i64() {
        let original = Value::I64(-42);
        let json: JsonValue = original.clone().try_into_validating_json().unwrap();
        let back: Value = json.into();
        assert_eq!(back, Value::I64(-42));
    }

    #[test]
    fn round_trip_text() {
        let original = Value::Text("hello world".into());
        let json: JsonValue = original.clone().try_into_validating_json().unwrap();
        let back: Value = json.into();
        assert_eq!(back, original);
    }

    // -----------------------------------------------------------------------
    // BTreeValueJsonConverter methods
    // -----------------------------------------------------------------------

    #[test]
    fn btree_into_json_value() {
        let mut map = BTreeMap::new();
        map.insert("x".to_string(), Value::U64(10));
        map.insert("y".to_string(), Value::Text("test".into()));
        let json = map.into_json_value().unwrap();
        assert!(json.is_object());
        assert_eq!(json["x"], json!(10));
        assert_eq!(json["y"], json!("test"));
    }

    #[test]
    fn btree_into_validating_json_value() {
        let mut map = BTreeMap::new();
        map.insert("n".to_string(), Value::U64(5));
        let json = map.into_validating_json_value().unwrap();
        assert_eq!(json["n"], json!(5));
    }

    #[test]
    fn btree_to_json_value() {
        let mut map = BTreeMap::new();
        map.insert("k".to_string(), Value::Bool(true));
        let json = map.to_json_value().unwrap();
        assert_eq!(json["k"], json!(true));
        // Original map is still available (borrow, not move)
        assert!(map.contains_key("k"));
    }

    #[test]
    fn btree_to_validating_json_value() {
        let mut map = BTreeMap::new();
        map.insert("v".to_string(), Value::I64(-1));
        let json = map.to_validating_json_value().unwrap();
        assert_eq!(json["v"], json!(-1));
    }

    #[test]
    fn btree_from_json_value() {
        let json = json!({"a": 1, "b": "two"});
        let map = BTreeMap::<String, Value>::from_json_value(json).unwrap();
        assert_eq!(map.get("a"), Some(&Value::U64(1)));
        assert_eq!(map.get("b"), Some(&Value::Text("two".into())));
    }

    #[test]
    fn btree_from_json_value_non_object_error() {
        let json = json!([1, 2, 3]);
        let result = BTreeMap::<String, Value>::from_json_value(json);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // From<BTreeMap<String, JsonValue>> for Value
    // -----------------------------------------------------------------------

    #[test]
    fn from_btree_json_map() {
        let mut btree = BTreeMap::new();
        btree.insert("key".to_string(), json!(42));
        let val: Value = btree.into();
        assert!(val.is_map());
    }

    #[test]
    fn from_btree_json_map_ref() {
        let mut btree = BTreeMap::new();
        btree.insert("key".to_string(), json!(42));
        let val: Value = (&btree).into();
        assert!(val.is_map());
    }

    // -----------------------------------------------------------------------
    // try_to_validating_json (borrow variant) mirrors try_into_validating_json
    // -----------------------------------------------------------------------

    #[test]
    fn try_to_validating_json_basic() {
        let val = Value::U64(99);
        let json = val.try_to_validating_json().unwrap();
        assert_eq!(json, json!(99));
    }

    #[test]
    fn try_to_validating_json_u128_too_large() {
        let val = Value::U128(u128::MAX);
        let err = val.try_to_validating_json().unwrap_err();
        assert_eq!(err, Error::IntegerSizeError);
    }

    #[test]
    fn try_to_validating_json_i128_too_large() {
        let val = Value::I128(i128::MAX);
        let err = val.try_to_validating_json().unwrap_err();
        assert_eq!(err, Error::IntegerSizeError);
    }

    #[test]
    fn try_to_validating_json_i128_too_small() {
        let val = Value::I128(i128::MIN);
        let err = val.try_to_validating_json().unwrap_err();
        assert_eq!(err, Error::IntegerSizeError);
    }

    #[test]
    fn try_to_validating_json_enum_u8_error() {
        let val = Value::EnumU8(vec![1]);
        let err = val.try_to_validating_json().unwrap_err();
        assert!(matches!(err, Error::Unsupported(_)));
    }

    #[test]
    fn try_to_validating_json_enum_string_error() {
        let val = Value::EnumString(vec!["a".into()]);
        let err = val.try_to_validating_json().unwrap_err();
        assert!(matches!(err, Error::Unsupported(_)));
    }

    // -----------------------------------------------------------------------
    // try_into_validating_btree_map_json
    // -----------------------------------------------------------------------

    #[test]
    fn try_into_validating_btree_map_json_success() {
        let map = vec![(Value::Text("k".into()), Value::U64(7))];
        let val = Value::Map(map);
        let result = val.try_into_validating_btree_map_json().unwrap();
        assert_eq!(result.get("k"), Some(&json!(7)));
    }

    // -----------------------------------------------------------------------
    // convert_from_serde_json_map
    // -----------------------------------------------------------------------

    #[test]
    fn convert_from_serde_json_map_basic() {
        let pairs = vec![
            ("a".to_string(), json!(1)),
            ("b".to_string(), json!("hello")),
        ];
        let result: BTreeMap<String, Value> = Value::convert_from_serde_json_map(pairs);
        assert_eq!(result.get("a"), Some(&Value::U64(1)));
        assert_eq!(result.get("b"), Some(&Value::Text("hello".into())));
    }

    #[test]
    fn non_finite_floats_error_in_all_three_json_converters() {
        // JSON (RFC 8259) cannot represent NaN/±∞. The converters used to
        // silently substitute 0 (`Number::from_f64(..).unwrap_or(0)`), which
        // made `Value::Float(f64::NAN)` indistinguishable from a real zero to
        // every JSON consumer. All three Value→JSON paths now fail loudly
        // instead — consistent with the loud IntegerSizeError the same
        // functions return for out-of-range integers.
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            // owned validating path
            assert!(matches!(
                Value::Float(bad).try_into_validating_json(),
                Err(Error::Unsupported(_))
            ));
            // by-ref validating path
            assert!(matches!(
                Value::Float(bad).try_to_validating_json(),
                Err(Error::Unsupported(_))
            ));
            // plain TryInto path
            let plain: Result<JsonValue, Error> = Value::Float(bad).try_into();
            assert!(matches!(plain, Err(Error::Unsupported(_))));
        }
        // Finite floats still convert.
        assert_eq!(
            Value::Float(1.5).try_into_validating_json().unwrap(),
            json!(1.5)
        );
    }
}
