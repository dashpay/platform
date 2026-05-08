use crate::value_map::ValueMap;
use crate::{Error, Value, ValueMapHelper};
use ciborium::value::Integer;
use ciborium::Value as CborValue;

impl Value {
    pub fn convert_from_cbor_map<I, R>(map: I) -> Result<R, Error>
    where
        I: IntoIterator<Item = (String, CborValue)>,
        R: FromIterator<(String, Value)>,
    {
        map.into_iter()
            .map(|(key, cbor_value)| Ok((key, cbor_value.try_into()?)))
            .collect()
    }

    pub fn convert_to_cbor_map<I, R>(map: I) -> Result<R, Error>
    where
        I: IntoIterator<Item = (String, Value)>,
        R: FromIterator<(String, CborValue)>,
    {
        map.into_iter()
            .map(|(key, value)| Ok((key, value.try_into()?)))
            .collect()
    }

    pub fn to_cbor_buffer(&self) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(self, &mut buffer)
            .map_err(|e| Error::CborSerializationError(e.to_string()))?;

        Ok(buffer)
    }
}

impl TryFrom<CborValue> for Value {
    type Error = Error;

    fn try_from(value: CborValue) -> Result<Self, Error> {
        Ok(match value {
            CborValue::Integer(integer) => Self::I128(integer.into()),
            CborValue::Bytes(bytes) => Self::Bytes(bytes),
            CborValue::Float(float) => Self::Float(float),
            CborValue::Text(string) => Self::Text(string),
            CborValue::Bool(value) => Self::Bool(value),
            CborValue::Null => Self::Null,
            CborValue::Tag(_, _) => {
                return Err(Error::Unsupported(
                    "conversion from cbor tags are currently not supported".to_string(),
                ))
            }
            CborValue::Array(array) => {
                let len = array.len();
                if len > 10
                    && array.iter().all(|v| {
                        let Some(int) = v.as_integer() else {
                            return false;
                        };
                        int.le(&Integer::from(u8::MAX)) && int.ge(&Integer::from(0))
                    })
                {
                    //this is an array of bytes
                    Self::Bytes(
                        array
                            .into_iter()
                            .map(|v| v.into_integer().unwrap().try_into().unwrap())
                            .collect(),
                    )
                } else {
                    Self::Array(
                        array
                            .into_iter()
                            .map(|v| v.try_into())
                            .collect::<Result<Vec<Value>, Error>>()?,
                    )
                }
            }
            CborValue::Map(map) => Self::Map(
                map.into_iter()
                    .map(|(k, v)| Ok((k.try_into()?, v.try_into()?)))
                    .collect::<Result<ValueMap, Error>>()?,
            ),
            _ => panic!("unsupported"),
        })
    }
}

impl TryInto<CborValue> for Value {
    type Error = Error;

    fn try_into(self) -> Result<CborValue, Self::Error> {
        Ok(match self {
            Value::U128(i) => CborValue::Integer((i as u64).into()),
            Value::I128(i) => CborValue::Integer((i as i64).into()),
            Value::U64(i) => CborValue::Integer(i.into()),
            Value::I64(i) => CborValue::Integer(i.into()),
            Value::U32(i) => CborValue::Integer(i.into()),
            Value::I32(i) => CborValue::Integer(i.into()),
            Value::U16(i) => CborValue::Integer(i.into()),
            Value::I16(i) => CborValue::Integer(i.into()),
            Value::U8(i) => CborValue::Integer(i.into()),
            Value::I8(i) => CborValue::Integer(i.into()),
            Value::Bytes(bytes) => CborValue::Bytes(bytes),
            Value::Bytes20(bytes) => CborValue::Bytes(bytes.to_vec()),
            Value::Bytes32(bytes) => CborValue::Bytes(bytes.to_vec()),
            Value::Bytes36(bytes) => CborValue::Bytes(bytes.to_vec()),
            Value::Float(float) => CborValue::Float(float),
            Value::Text(string) => CborValue::Text(string),
            Value::Bool(value) => CborValue::Bool(value),
            Value::Null => CborValue::Null,
            Value::Array(array) => CborValue::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into())
                    .collect::<Result<Vec<CborValue>, Error>>()?,
            ),
            Value::Map(mut map) => {
                map.sort_by_keys();
                CborValue::Map(
                    map.into_iter()
                        .map(|(k, v)| Ok((k.try_into()?, v.try_into()?)))
                        .collect::<Result<Vec<(CborValue, CborValue)>, Error>>()?,
                )
            }
            Value::Identifier(bytes) => CborValue::Bytes(bytes.to_vec()),
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

impl TryInto<Box<CborValue>> for Box<Value> {
    type Error = Error;
    fn try_into(self) -> Result<Box<CborValue>, Self::Error> {
        (*self).try_into().map(Box::new)
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use crate::{Error, Value};
    use ciborium::value::Integer;
    use ciborium::Value as CborValue;

    // -----------------------------------------------------------------------
    // Round-trip: Value -> CborValue -> Value for basic types
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_null() {
        let original = Value::Null;
        let cbor: CborValue = original.clone().try_into().unwrap();
        assert_eq!(cbor, CborValue::Null);
        let back: Value = cbor.try_into().unwrap();
        assert_eq!(back, Value::Null);
    }

    #[test]
    fn round_trip_bool_true() {
        let original = Value::Bool(true);
        let cbor: CborValue = original.clone().try_into().unwrap();
        assert_eq!(cbor, CborValue::Bool(true));
        let back: Value = cbor.try_into().unwrap();
        // Comes back as I128(1) since CBOR integers are unified
        assert_eq!(back, Value::Bool(true));
    }

    #[test]
    fn round_trip_bool_false() {
        let original = Value::Bool(false);
        let cbor: CborValue = original.clone().try_into().unwrap();
        let back: Value = cbor.try_into().unwrap();
        assert_eq!(back, Value::Bool(false));
    }

    #[test]
    fn round_trip_text() {
        let original = Value::Text("hello world".into());
        let cbor: CborValue = original.clone().try_into().unwrap();
        assert_eq!(cbor, CborValue::Text("hello world".into()));
        let back: Value = cbor.try_into().unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn round_trip_float() {
        let original = Value::Float(3.14);
        let cbor: CborValue = original.clone().try_into().unwrap();
        assert_eq!(cbor, CborValue::Float(3.14));
        let back: Value = cbor.try_into().unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn round_trip_bytes() {
        let original = Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let cbor: CborValue = original.clone().try_into().unwrap();
        assert_eq!(cbor, CborValue::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
        let back: Value = cbor.try_into().unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn round_trip_u64() {
        let original = Value::U64(42);
        let cbor: CborValue = original.clone().try_into().unwrap();
        // Comes back as Integer
        let back: Value = cbor.try_into().unwrap();
        // CBOR integers come back as I128
        assert_eq!(back, Value::I128(42));
    }

    #[test]
    fn round_trip_i64_negative() {
        let original = Value::I64(-99);
        let cbor: CborValue = original.clone().try_into().unwrap();
        let back: Value = cbor.try_into().unwrap();
        assert_eq!(back, Value::I128(-99));
    }

    // -----------------------------------------------------------------------
    // Tag rejection in TryFrom<CborValue>
    // -----------------------------------------------------------------------

    #[test]
    fn cbor_tag_rejected() {
        let tagged = CborValue::Tag(42, Box::new(CborValue::Null));
        let result: Result<Value, Error> = tagged.try_into();
        assert!(matches!(result, Err(Error::Unsupported(_))));
    }

    #[test]
    fn cbor_tag_rejection_message() {
        let tagged = CborValue::Tag(0, Box::new(CborValue::Text("date".into())));
        let err = Value::try_from(tagged).unwrap_err();
        match err {
            Error::Unsupported(msg) => {
                assert!(msg.contains("tag"), "error message should mention tags");
            }
            _ => panic!("expected Unsupported error"),
        }
    }

    // -----------------------------------------------------------------------
    // Byte-array heuristic boundary (10 vs 11 integer elements)
    // Note: the CBOR heuristic uses > 10 (strictly greater), unlike JSON's >= 10
    // -----------------------------------------------------------------------

    #[test]
    fn cbor_array_10_integers_stays_array() {
        // Exactly 10 elements -> stays as Array (boundary: > 10 needed for bytes)
        let arr: Vec<CborValue> = (0..10)
            .map(|i| CborValue::Integer(Integer::from(i as u8)))
            .collect();
        let cbor = CborValue::Array(arr);
        let val: Value = cbor.try_into().unwrap();
        assert!(
            matches!(val, Value::Array(_)),
            "10 elements should stay as Array in CBOR heuristic"
        );
    }

    #[test]
    fn cbor_array_11_integers_becomes_bytes() {
        // 11 elements, all in u8 range -> becomes Bytes
        let arr: Vec<CborValue> = (0..11)
            .map(|i| CborValue::Integer(Integer::from(i as u8)))
            .collect();
        let cbor = CborValue::Array(arr);
        let val: Value = cbor.try_into().unwrap();
        assert!(
            matches!(val, Value::Bytes(_)),
            "11 elements of u8-range integers should become Bytes"
        );
        if let Value::Bytes(bytes) = val {
            assert_eq!(bytes.len(), 11);
            assert_eq!(bytes[0], 0);
            assert_eq!(bytes[10], 10);
        }
    }

    #[test]
    fn cbor_array_mixed_types_stays_array() {
        // 12 elements but mixed types -> stays as Array
        let mut arr: Vec<CborValue> = (0..11)
            .map(|i| CborValue::Integer(Integer::from(i as u8)))
            .collect();
        arr.push(CborValue::Text("not an int".into()));
        let cbor = CborValue::Array(arr);
        let val: Value = cbor.try_into().unwrap();
        assert!(matches!(val, Value::Array(_)));
    }

    #[test]
    fn cbor_array_negative_values_stays_array() {
        // Negative values are not in 0..=u8::MAX range
        let arr: Vec<CborValue> = (0..12)
            .map(|i| CborValue::Integer(Integer::from(-(i as i64))))
            .collect();
        let cbor = CborValue::Array(arr);
        let val: Value = cbor.try_into().unwrap();
        // First element is 0 which is fine, but most are negative -> fails the ge(0) check
        assert!(matches!(val, Value::Array(_)));
    }

    // -----------------------------------------------------------------------
    // Map key sorting in TryInto<CborValue>
    // -----------------------------------------------------------------------

    #[test]
    fn map_keys_sorted_in_cbor_output() {
        // Keys inserted in reverse order should be sorted in output
        let map = vec![
            (Value::Text("z".into()), Value::U64(1)),
            (Value::Text("a".into()), Value::U64(2)),
            (Value::Text("m".into()), Value::U64(3)),
        ];
        let val = Value::Map(map);
        let cbor: CborValue = val.try_into().unwrap();
        if let CborValue::Map(pairs) = cbor {
            let keys: Vec<String> = pairs
                .iter()
                .map(|(k, _)| {
                    if let CborValue::Text(s) = k {
                        s.clone()
                    } else {
                        panic!("expected text key")
                    }
                })
                .collect();
            assert_eq!(keys, vec!["a", "m", "z"]);
        } else {
            panic!("expected CborValue::Map");
        }
    }

    // -----------------------------------------------------------------------
    // EnumU8 / EnumString error paths
    // -----------------------------------------------------------------------

    #[test]
    fn enum_u8_to_cbor_error() {
        let val = Value::EnumU8(vec![1, 2, 3]);
        let result: Result<CborValue, Error> = val.try_into();
        assert!(matches!(result, Err(Error::Unsupported(_))));
    }

    #[test]
    fn enum_string_to_cbor_error() {
        let val = Value::EnumString(vec!["a".into(), "b".into()]);
        let result: Result<CborValue, Error> = val.try_into();
        assert!(matches!(result, Err(Error::Unsupported(_))));
    }

    // -----------------------------------------------------------------------
    // U128 / I128 narrowing in TryInto<CborValue>
    // -----------------------------------------------------------------------

    #[test]
    fn u128_narrowed_to_u64_in_cbor() {
        // U128 is cast to u64 when converting to CborValue::Integer
        let val = Value::U128(42);
        let cbor: CborValue = val.try_into().unwrap();
        assert_eq!(cbor, CborValue::Integer(42u64.into()));
    }

    #[test]
    fn i128_narrowed_to_i64_in_cbor() {
        // I128 is cast to i64 when converting to CborValue::Integer
        let val = Value::I128(-99);
        let cbor: CborValue = val.try_into().unwrap();
        assert_eq!(cbor, CborValue::Integer((-99i64).into()));
    }

    // -----------------------------------------------------------------------
    // Integer variant from CBOR -> Value
    // -----------------------------------------------------------------------

    #[test]
    fn cbor_positive_integer_to_i128() {
        let cbor = CborValue::Integer(Integer::from(255u8));
        let val: Value = cbor.try_into().unwrap();
        assert_eq!(val, Value::I128(255));
    }

    #[test]
    fn cbor_negative_integer_to_i128() {
        let cbor = CborValue::Integer(Integer::from(-1i64));
        let val: Value = cbor.try_into().unwrap();
        assert_eq!(val, Value::I128(-1));
    }

    #[test]
    fn cbor_zero_integer_to_i128() {
        let cbor = CborValue::Integer(Integer::from(0));
        let val: Value = cbor.try_into().unwrap();
        assert_eq!(val, Value::I128(0));
    }

    // -----------------------------------------------------------------------
    // Bytes20 / Bytes32 / Bytes36 / Identifier -> CborValue::Bytes
    // -----------------------------------------------------------------------

    #[test]
    fn bytes20_to_cbor_bytes() {
        let bytes = [0xAAu8; 20];
        let val = Value::Bytes20(bytes);
        let cbor: CborValue = val.try_into().unwrap();
        assert_eq!(cbor, CborValue::Bytes(bytes.to_vec()));
    }

    #[test]
    fn bytes32_to_cbor_bytes() {
        let bytes = [0xBBu8; 32];
        let val = Value::Bytes32(bytes);
        let cbor: CborValue = val.try_into().unwrap();
        assert_eq!(cbor, CborValue::Bytes(bytes.to_vec()));
    }

    #[test]
    fn bytes36_to_cbor_bytes() {
        let bytes = [0xCCu8; 36];
        let val = Value::Bytes36(bytes);
        let cbor: CborValue = val.try_into().unwrap();
        assert_eq!(cbor, CborValue::Bytes(bytes.to_vec()));
    }

    #[test]
    fn identifier_to_cbor_bytes() {
        let bytes = [0x01u8; 32];
        let val = Value::Identifier(bytes);
        let cbor: CborValue = val.try_into().unwrap();
        assert_eq!(cbor, CborValue::Bytes(bytes.to_vec()));
    }

    // -----------------------------------------------------------------------
    // Integer types round through CBOR
    // -----------------------------------------------------------------------

    #[test]
    fn all_integer_types_to_cbor() {
        let cases: Vec<Value> = vec![
            Value::U8(255),
            Value::I8(-128),
            Value::U16(65535),
            Value::I16(-32768),
            Value::U32(u32::MAX),
            Value::I32(i32::MIN),
            Value::U64(u64::MAX),
            Value::I64(i64::MIN),
        ];
        for val in cases {
            let cbor: CborValue = val.clone().try_into().unwrap();
            assert!(
                matches!(cbor, CborValue::Integer(_)),
                "expected Integer for {:?}",
                val
            );
        }
    }

    // -----------------------------------------------------------------------
    // Box<Value> TryInto<Box<CborValue>>
    // -----------------------------------------------------------------------

    #[test]
    fn boxed_value_to_boxed_cbor() {
        let val = Box::new(Value::Text("boxed".into()));
        let cbor: Box<CborValue> = val.try_into().unwrap();
        assert_eq!(*cbor, CborValue::Text("boxed".into()));
    }

    // -----------------------------------------------------------------------
    // CBOR Map conversion
    // -----------------------------------------------------------------------

    #[test]
    fn cbor_map_to_value_map() {
        let cbor = CborValue::Map(vec![
            (
                CborValue::Text("key".into()),
                CborValue::Integer(42u64.into()),
            ),
            (CborValue::Text("flag".into()), CborValue::Bool(true)),
        ]);
        let val: Value = cbor.try_into().unwrap();
        assert!(val.is_map());
    }

    // -----------------------------------------------------------------------
    // convert_from_cbor_map / convert_to_cbor_map
    // -----------------------------------------------------------------------

    #[test]
    fn convert_from_cbor_map_basic() {
        let pairs = vec![
            ("a".to_string(), CborValue::Bool(true)),
            ("b".to_string(), CborValue::Text("hello".into())),
        ];
        let result: std::collections::BTreeMap<String, Value> =
            Value::convert_from_cbor_map(pairs).unwrap();
        assert_eq!(result.get("a"), Some(&Value::Bool(true)));
        assert_eq!(result.get("b"), Some(&Value::Text("hello".into())));
    }

    #[test]
    fn convert_to_cbor_map_basic() {
        let pairs = vec![
            ("x".to_string(), Value::U64(10)),
            ("y".to_string(), Value::Bool(false)),
        ];
        let result: std::collections::BTreeMap<String, CborValue> =
            Value::convert_to_cbor_map(pairs).unwrap();
        assert_eq!(result.get("x"), Some(&CborValue::Integer(10u64.into())));
        assert_eq!(result.get("y"), Some(&CborValue::Bool(false)));
    }

    // -----------------------------------------------------------------------
    // to_cbor_buffer
    // -----------------------------------------------------------------------

    #[test]
    fn to_cbor_buffer_roundtrip() {
        let val = Value::Text("cbor buffer test".into());
        let buf = val.to_cbor_buffer().unwrap();
        assert!(!buf.is_empty());
    }

    // -----------------------------------------------------------------------
    // CBOR array with nested values
    // -----------------------------------------------------------------------

    #[test]
    fn cbor_array_with_nested_map() {
        let cbor = CborValue::Array(vec![
            CborValue::Text("item".into()),
            CborValue::Map(vec![(
                CborValue::Text("inner".into()),
                CborValue::Bool(true),
            )]),
        ]);
        let val: Value = cbor.try_into().unwrap();
        assert!(matches!(val, Value::Array(_)));
        if let Value::Array(arr) = &val {
            assert_eq!(arr.len(), 2);
            assert!(arr[1].is_map());
        }
    }
}
