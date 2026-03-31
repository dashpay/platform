// Comprehensive coverage tests for platform-value crate.
//
// These tests target previously uncovered code paths across multiple modules:
// - Value type conversions and coercions (lib.rs)
// - Inner value map operations (inner_value.rs)
// - Path-based value access and mutation (inner_value_at_path.rs)
// - JSON/CBOR serialization conversions (converter/)
// - Display formatting (display.rs)
// - PartialEq implementations (eq.rs)
// - String encoding/decoding (string_encoding.rs)
// - Value map helpers (value_map.rs)
// - Index/IndexMut operations (index.rs)
// - Pointer/take operations (pointer.rs)
// - Patch operations (patch/)
// - System bytes conversions (system_bytes.rs)
// - BTreeMap extension helpers (btreemap_extensions/)
// - Replace operations (replace.rs)
// - Inner array push (inner_array_value.rs)
// - From/TryFrom implementations (lib.rs)

use platform_value::patch::{diff, merge};
use platform_value::{
    from_value, patch, platform_value, to_value, BinaryData, Error, Identifier,
    IntegerReplacementType, Patch, Value, ValueMap, ValueMapHelper,
};
use std::collections::BTreeMap;

// ===========================================================================
// lib.rs  --  integer type checks, broad conversion, has_data_larger_than
// ===========================================================================

mod value_type_checks {
    use super::*;

    #[test]
    fn is_integer_covers_all_variants() {
        assert!(Value::U128(1).is_integer());
        assert!(Value::I128(-1).is_integer());
        assert!(Value::U64(1).is_integer());
        assert!(Value::I64(-1).is_integer());
        assert!(Value::U32(1).is_integer());
        assert!(Value::I32(-1).is_integer());
        assert!(Value::U16(1).is_integer());
        assert!(Value::I16(-1).is_integer());
        assert!(Value::U8(1).is_integer());
        assert!(Value::I8(-1).is_integer());
        assert!(!Value::Float(1.0).is_integer());
        assert!(!Value::Text("x".into()).is_integer());
        assert!(!Value::Null.is_integer());
    }

    #[test]
    fn is_integer_can_fit_in_64_bits() {
        // All <= 64-bit widths fit
        assert!(Value::U64(u64::MAX).is_integer_can_fit_in_64_bits());
        assert!(Value::I64(i64::MIN).is_integer_can_fit_in_64_bits());
        assert!(Value::U32(100).is_integer_can_fit_in_64_bits());
        assert!(Value::I32(-100).is_integer_can_fit_in_64_bits());
        assert!(Value::U16(100).is_integer_can_fit_in_64_bits());
        assert!(Value::I16(-100).is_integer_can_fit_in_64_bits());
        assert!(Value::U8(100).is_integer_can_fit_in_64_bits());
        assert!(Value::I8(-100).is_integer_can_fit_in_64_bits());

        // U128 within u64 range
        assert!(Value::U128(u64::MAX as u128).is_integer_can_fit_in_64_bits());
        // U128 out of range
        assert!(!Value::U128(u128::MAX).is_integer_can_fit_in_64_bits());

        // I128 within i64 range
        assert!(Value::I128(i64::MAX as i128).is_integer_can_fit_in_64_bits());
        assert!(Value::I128(i64::MIN as i128).is_integer_can_fit_in_64_bits());
        // I128 out of range
        assert!(!Value::I128(i128::MAX).is_integer_can_fit_in_64_bits());
        assert!(!Value::I128(i128::MIN).is_integer_can_fit_in_64_bits());

        // Non-integers
        assert!(!Value::Float(1.0).is_integer_can_fit_in_64_bits());
        assert!(!Value::Text("x".into()).is_integer_can_fit_in_64_bits());
    }

    #[test]
    fn as_integer_all_variants() {
        assert_eq!(Value::U128(42).as_integer::<u64>(), Some(42u64));
        assert_eq!(Value::I128(-42).as_integer::<i64>(), Some(-42i64));
        assert_eq!(Value::U64(42).as_integer::<u64>(), Some(42u64));
        assert_eq!(Value::I64(-42).as_integer::<i64>(), Some(-42i64));
        assert_eq!(Value::U32(42).as_integer::<u32>(), Some(42u32));
        assert_eq!(Value::I32(-42).as_integer::<i32>(), Some(-42i32));
        assert_eq!(Value::U16(42).as_integer::<u16>(), Some(42u16));
        assert_eq!(Value::I16(-42).as_integer::<i16>(), Some(-42i16));
        assert_eq!(Value::U8(42).as_integer::<u8>(), Some(42u8));
        assert_eq!(Value::I8(-42).as_integer::<i8>(), Some(-42i8));
        assert_eq!(Value::Bool(true).as_integer::<u8>(), None);
        // Overflow: large u128 to u8
        assert_eq!(Value::U128(300).as_integer::<u8>(), None);
    }

    #[test]
    fn into_integer_error_paths() {
        let val = Value::Bool(true);
        let result: Result<u64, Error> = val.into_integer();
        assert_eq!(
            result,
            Err(Error::StructureError("value is not an integer".to_string()))
        );

        // Overflow: I128 to u8
        let val = Value::I128(300);
        let result: Result<u8, Error> = val.into_integer();
        assert_eq!(result, Err(Error::IntegerSizeError));

        // Overflow: U128 to i8
        let val = Value::U128(300);
        let result: Result<i8, Error> = val.into_integer();
        assert_eq!(result, Err(Error::IntegerSizeError));
    }

    #[test]
    fn to_integer_error_path() {
        let val = Value::Text("hello".into());
        let result: Result<u64, Error> = val.to_integer();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("value is not an integer"));
    }
}

mod broad_integer_conversion {
    use super::*;

    #[test]
    fn float_positive_to_integer() {
        let val = Value::Float(42.9);
        let result: Result<u64, Error> = val.to_integer_broad_conversion();
        assert_eq!(result, Ok(42)); // truncates
    }

    #[test]
    fn float_negative_to_integer() {
        let val = Value::Float(-42.9);
        let result: Result<i64, Error> = val.to_integer_broad_conversion();
        assert_eq!(result, Ok(-42)); // truncates
    }

    #[test]
    fn float_zero_to_integer() {
        // 0.0 is neither > 0 nor < 0, should error
        let val = Value::Float(0.0);
        let result: Result<u64, Error> = val.to_integer_broad_conversion();
        assert_eq!(result, Err(Error::IntegerSizeError));
    }

    #[test]
    fn bool_false_to_integer() {
        let val = Value::Bool(false);
        let result: Result<u64, Error> = val.to_integer_broad_conversion();
        assert_eq!(result, Ok(0));
    }

    #[test]
    fn bool_true_to_integer() {
        let val = Value::Bool(true);
        let result: Result<u64, Error> = val.to_integer_broad_conversion();
        assert_eq!(result, Ok(1));
    }

    #[test]
    fn text_to_integer() {
        let val = Value::Text("123".to_string());
        let result: Result<u64, Error> = val.to_integer_broad_conversion();
        assert_eq!(result, Ok(123));
    }

    #[test]
    fn text_non_numeric() {
        let val = Value::Text("abc".to_string());
        let result: Result<u64, Error> = val.to_integer_broad_conversion();
        assert_eq!(result, Err(Error::IntegerSizeError));
    }

    #[test]
    fn non_convertible_type() {
        let val = Value::Bytes(vec![1, 2, 3]);
        let result: Result<u64, Error> = val.to_integer_broad_conversion();
        assert!(matches!(result, Err(Error::StructureError(_))));
    }
}

mod has_data_larger_than_tests {
    use super::*;

    #[test]
    fn integer_types() {
        assert!(Value::U128(1).has_data_larger_than(15).is_some());
        assert!(Value::U128(1).has_data_larger_than(16).is_none());
        assert!(Value::I128(1).has_data_larger_than(15).is_some());
        assert!(Value::I128(1).has_data_larger_than(16).is_none());
        assert!(Value::U64(1).has_data_larger_than(7).is_some());
        assert!(Value::U64(1).has_data_larger_than(8).is_none());
        assert!(Value::I64(1).has_data_larger_than(7).is_some());
        assert!(Value::I64(1).has_data_larger_than(8).is_none());
        assert!(Value::U32(1).has_data_larger_than(3).is_some());
        assert!(Value::U32(1).has_data_larger_than(4).is_none());
        assert!(Value::I32(1).has_data_larger_than(3).is_some());
        assert!(Value::I32(1).has_data_larger_than(4).is_none());
        assert!(Value::U16(1).has_data_larger_than(1).is_some());
        assert!(Value::U16(1).has_data_larger_than(2).is_none());
        assert!(Value::I16(1).has_data_larger_than(1).is_some());
        assert!(Value::I16(1).has_data_larger_than(2).is_none());
        assert!(Value::U8(1).has_data_larger_than(0).is_some());
        assert!(Value::U8(1).has_data_larger_than(1).is_none());
        assert!(Value::I8(1).has_data_larger_than(0).is_some());
        assert!(Value::I8(1).has_data_larger_than(1).is_none());
    }

    #[test]
    fn bytes_types() {
        assert!(Value::Bytes(vec![0; 5]).has_data_larger_than(4).is_some());
        assert!(Value::Bytes(vec![0; 5]).has_data_larger_than(5).is_none());
        assert!(Value::Bytes20([0; 20]).has_data_larger_than(19).is_some());
        assert!(Value::Bytes20([0; 20]).has_data_larger_than(20).is_none());
        assert!(Value::Bytes32([0; 32]).has_data_larger_than(31).is_some());
        assert!(Value::Bytes32([0; 32]).has_data_larger_than(32).is_none());
        assert!(Value::Bytes36([0; 36]).has_data_larger_than(35).is_some());
        assert!(Value::Bytes36([0; 36]).has_data_larger_than(36).is_none());
        assert!(Value::Identifier([0; 32])
            .has_data_larger_than(31)
            .is_some());
        assert!(Value::Identifier([0; 32])
            .has_data_larger_than(32)
            .is_none());
    }

    #[test]
    fn other_types() {
        assert!(Value::Float(1.0).has_data_larger_than(7).is_some());
        assert!(Value::Float(1.0).has_data_larger_than(8).is_none());
        assert!(Value::Bool(true).has_data_larger_than(0).is_some());
        assert!(Value::Bool(true).has_data_larger_than(1).is_none());
        assert!(Value::Null.has_data_larger_than(0).is_some());
        assert!(Value::Null.has_data_larger_than(1).is_none());
        assert!(Value::Text("abc".into()).has_data_larger_than(2).is_some());
        assert!(Value::Text("abc".into()).has_data_larger_than(3).is_none());
        assert!(Value::EnumU8(vec![1]).has_data_larger_than(0).is_some());
        assert!(Value::EnumU8(vec![1]).has_data_larger_than(1).is_none());
    }

    #[test]
    fn enum_string() {
        let v = Value::EnumString(vec!["hello".into(), "hi".into()]);
        assert!(v.has_data_larger_than(4).is_some()); // "hello" is 5 > 4
        assert!(v.has_data_larger_than(5).is_none()); // max is 5, not > 5
        let v_empty = Value::EnumString(vec![]);
        assert!(v_empty.has_data_larger_than(0).is_none());
    }

    #[test]
    fn array_and_map() {
        let v = Value::Array(vec![Value::Text("long_string_here".into()), Value::U8(1)]);
        assert!(v.has_data_larger_than(5).is_some());
        assert!(v.has_data_larger_than(100).is_none());

        let v = Value::Map(vec![(
            Value::Text("key".into()),
            Value::Text("long_value".into()),
        )]);
        assert!(v.has_data_larger_than(5).is_some());
        assert!(v.has_data_larger_than(100).is_none());
    }
}

// ===========================================================================
// From / TryFrom impls
// ===========================================================================

mod from_impls {
    use super::*;

    #[test]
    fn from_option_none() {
        let v: Value = None.into();
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn from_option_some() {
        let v: Value = Some(Value::U64(5)).into();
        assert_eq!(v, Value::U64(5));
    }

    #[test]
    fn from_char() {
        let v: Value = 'z'.into();
        assert_eq!(v, Value::Text("z".to_string()));
    }

    #[test]
    fn from_ref_string() {
        let s = String::from("hello");
        let v: Value = (&s).into();
        assert_eq!(v, Value::Text("hello".to_string()));
    }

    #[test]
    fn from_vec_str() {
        let v: Value = vec!["a", "b"].into();
        assert_eq!(
            v,
            Value::Array(vec![Value::Text("a".into()), Value::Text("b".into()),])
        );
    }

    #[test]
    fn from_slice_str() {
        let items: &[&str] = &["x", "y"];
        let v: Value = items.into();
        assert_eq!(
            v,
            Value::Array(vec![Value::Text("x".into()), Value::Text("y".into()),])
        );
    }

    #[test]
    fn from_btreemap_string_value() {
        let mut m = BTreeMap::new();
        m.insert("k".to_string(), Value::U32(7));
        let v: Value = m.into();
        assert!(v.is_map());
    }

    #[test]
    fn from_ref_btreemap_string_value() {
        let mut m = BTreeMap::new();
        m.insert("k".to_string(), Value::U32(7));
        let v: Value = (&m).into();
        assert!(v.is_map());
    }

    #[test]
    fn from_btreemap_option_values() {
        let mut m: BTreeMap<String, Option<String>> = BTreeMap::new();
        m.insert("present".to_string(), Some("val".to_string()));
        m.insert("absent".to_string(), None);
        let v: Value = m.into();
        assert!(v.is_map());
    }

    #[test]
    fn from_str_value_array_to_map() {
        let v: Value = [("a", Value::U8(1)), ("b", Value::U8(2))].into();
        assert!(v.is_map());
    }

    #[test]
    fn from_string_value_array_to_map() {
        let v: Value = [
            ("a".to_string(), Value::U8(1)),
            ("b".to_string(), Value::U8(2)),
        ]
        .into();
        assert!(v.is_map());
    }

    #[test]
    fn from_value_pair_array_to_map() {
        let v: Value = [(Value::U8(1), Value::U8(2))].into();
        assert!(v.is_map());
    }

    #[test]
    fn empty_array_from() {
        let v: Value = ([] as [(&str, Value); 0]).into();
        assert_eq!(v, Value::Map(vec![]));
    }

    #[test]
    fn try_from_value_to_string() {
        let v = Value::Text("hello".to_string());
        let s: String = v.try_into().unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn try_from_value_to_vec_u8() {
        let v = Value::Bytes(vec![1, 2, 3]);
        let b: Vec<u8> = v.try_into().unwrap();
        assert_eq!(b, vec![1, 2, 3]);
    }

    #[test]
    fn try_from_value_integer_types() {
        let result: Result<u64, Error> = Value::U32(42).try_into();
        assert_eq!(result, Ok(42u64));
        let result: Result<i32, Error> = Value::I16(-5).try_into();
        assert_eq!(result, Ok(-5i32));
    }

    #[test]
    fn from_btreemap_ref_value() {
        let mut m = BTreeMap::new();
        let val = Value::U32(7);
        m.insert("k".to_string(), &val);
        let v: Value = m.into();
        assert!(v.is_map());
    }
}

// ===========================================================================
// bytes conversions
// ===========================================================================

mod bytes_conversions {
    use super::*;

    #[test]
    fn is_any_bytes_type() {
        assert!(Value::Bytes(vec![]).is_any_bytes_type());
        assert!(Value::Bytes20([0; 20]).is_any_bytes_type());
        assert!(Value::Bytes32([0; 32]).is_any_bytes_type());
        assert!(Value::Bytes36([0; 36]).is_any_bytes_type());
        assert!(Value::Identifier([0; 32]).is_any_bytes_type());
        assert!(!Value::U64(0).is_any_bytes_type());
    }

    #[test]
    fn into_bytes_from_typed_bytes() {
        assert_eq!(Value::Bytes20([1; 20]).into_bytes().unwrap().len(), 20);
        assert_eq!(Value::Bytes32([1; 32]).into_bytes().unwrap().len(), 32);
        assert_eq!(Value::Bytes36([1; 36]).into_bytes().unwrap().len(), 36);
        assert_eq!(Value::Identifier([1; 32]).into_bytes().unwrap().len(), 32);
    }

    #[test]
    fn into_bytes_from_array() {
        let v = Value::Array(vec![Value::U8(1), Value::U8(2), Value::U8(3)]);
        assert_eq!(v.into_bytes(), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn to_bytes_from_typed_bytes() {
        assert_eq!(Value::Bytes20([2; 20]).to_bytes().unwrap().len(), 20);
        assert_eq!(Value::Bytes32([2; 32]).to_bytes().unwrap().len(), 32);
        assert_eq!(Value::Bytes36([2; 36]).to_bytes().unwrap().len(), 36);
        assert_eq!(Value::Identifier([2; 32]).to_bytes().unwrap().len(), 32);
    }

    #[test]
    fn to_bytes_from_array() {
        let v = Value::Array(vec![Value::U8(10), Value::U8(20)]);
        assert_eq!(v.to_bytes(), Ok(vec![10, 20]));
    }

    #[test]
    fn to_binary_data_from_typed_bytes() {
        assert_eq!(
            Value::Bytes20([3; 20]).to_binary_data().unwrap(),
            BinaryData::new(vec![3; 20])
        );
        assert_eq!(
            Value::Bytes32([3; 32]).to_binary_data().unwrap(),
            BinaryData::new(vec![3; 32])
        );
        assert_eq!(
            Value::Bytes36([3; 36]).to_binary_data().unwrap(),
            BinaryData::new(vec![3; 36])
        );
        assert_eq!(
            Value::Identifier([3; 32]).to_binary_data().unwrap(),
            BinaryData::new(vec![3; 32])
        );
    }

    #[test]
    fn to_binary_data_from_array() {
        let v = Value::Array(vec![Value::U8(5), Value::U8(6)]);
        assert_eq!(v.to_binary_data(), Ok(BinaryData::new(vec![5, 6])));
    }

    #[test]
    fn as_bytes_slice_from_typed_bytes() {
        assert_eq!(Value::Bytes20([4; 20]).as_bytes_slice().unwrap().len(), 20);
        assert_eq!(Value::Bytes32([4; 32]).as_bytes_slice().unwrap().len(), 32);
        assert_eq!(Value::Bytes36([4; 36]).as_bytes_slice().unwrap().len(), 36);
        assert_eq!(
            Value::Identifier([4; 32]).as_bytes_slice().unwrap().len(),
            32
        );
    }

    #[test]
    fn as_bytes_slice_error() {
        assert!(Value::U64(1).as_bytes_slice().is_err());
    }
}

// ===========================================================================
// float / text / bool / null / array / map accessors
// ===========================================================================

mod accessors {
    use super::*;

    #[test]
    fn float_from_integers() {
        assert_eq!(Value::U128(5).as_float(), Some(5.0));
        assert_eq!(Value::I128(-5).as_float(), Some(-5.0));
        assert_eq!(Value::U32(5).as_float(), Some(5.0));
        assert_eq!(Value::I32(-5).as_float(), Some(-5.0));
        assert_eq!(Value::U16(5).as_float(), Some(5.0));
        assert_eq!(Value::I16(-5).as_float(), Some(-5.0));
        assert_eq!(Value::U8(5).as_float(), Some(5.0));
        assert_eq!(Value::I8(-5).as_float(), Some(-5.0));
        assert_eq!(Value::Text("x".into()).as_float(), None);
    }

    #[test]
    fn into_float_from_integers() {
        assert_eq!(Value::U128(5).into_float(), Ok(5.0));
        assert_eq!(Value::I128(-5).into_float(), Ok(-5.0));
        assert_eq!(Value::U32(5).into_float(), Ok(5.0));
        assert_eq!(Value::I32(-5).into_float(), Ok(-5.0));
        assert_eq!(Value::U16(5).into_float(), Ok(5.0));
        assert_eq!(Value::I16(-5).into_float(), Ok(-5.0));
        assert_eq!(Value::U8(5).into_float(), Ok(5.0));
        assert_eq!(Value::I8(-5).into_float(), Ok(-5.0));
        assert!(Value::Null.into_float().is_err());
    }

    #[test]
    fn to_float_from_integers() {
        assert_eq!(Value::U128(5).to_float(), Ok(5.0));
        assert_eq!(Value::I128(-5).to_float(), Ok(-5.0));
        assert_eq!(Value::U32(5).to_float(), Ok(5.0));
        assert_eq!(Value::I32(-5).to_float(), Ok(-5.0));
        assert_eq!(Value::U16(5).to_float(), Ok(5.0));
        assert_eq!(Value::I16(-5).to_float(), Ok(-5.0));
        assert_eq!(Value::U8(5).to_float(), Ok(5.0));
        assert_eq!(Value::I8(-5).to_float(), Ok(-5.0));
        assert!(Value::Null.to_float().is_err());
    }

    #[test]
    fn text_accessors() {
        let v = Value::Text("hello".into());
        assert!(v.is_text());
        assert_eq!(v.as_text(), Some("hello"));
        assert_eq!(v.to_str(), Ok("hello"));
        assert_eq!(v.to_text(), Ok("hello".into()));
        assert_eq!(v.as_str(), Some("hello"));

        assert!(!Value::U64(1).is_text());
        assert_eq!(
            Value::U64(1).to_str(),
            Err(Error::StructureError("value is not a string".into()))
        );
        assert_eq!(
            Value::U64(1).to_text(),
            Err(Error::StructureError("value is not a string".into()))
        );
        assert_eq!(
            Value::U64(1).into_text(),
            Err(Error::StructureError("value is not a string".into()))
        );
    }

    #[test]
    fn text_mut() {
        let mut v = Value::Text("hello".into());
        v.as_text_mut().unwrap().push_str(" world");
        assert_eq!(v.as_text(), Some("hello world"));
        assert_eq!(Value::U64(1).as_text_mut(), None);
    }

    #[test]
    fn bool_accessors() {
        let v = Value::Bool(true);
        assert!(v.is_bool());
        assert_eq!(v.as_bool(), Some(true));
        assert_eq!(v.to_bool(), Ok(true));
        assert_eq!(v.into_bool(), Ok(true));

        assert!(!Value::Null.is_bool());
        assert_eq!(Value::Null.as_bool(), None);
    }

    #[test]
    fn null_check() {
        assert!(Value::Null.is_null());
        assert!(!Value::U64(0).is_null());
    }

    #[test]
    fn array_accessors() {
        let v = Value::Array(vec![Value::U8(1)]);
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v.to_array_slice().unwrap().len(), 1);
        assert_eq!(v.to_array_ref().unwrap().len(), 1);
        assert_eq!(v.to_array_owned().unwrap().len(), 1);
        assert_eq!(v.as_slice().unwrap().len(), 1);
    }

    #[test]
    fn array_error_paths() {
        let v = Value::Bool(true);
        assert!(v.to_array_slice().is_err());
        assert!(v.to_array_ref().is_err());
        assert!(v.to_array_owned().is_err());
        assert!(v.as_slice().is_err());
        assert!(Value::Bool(true).into_array().is_err());
    }

    #[test]
    fn array_mut() {
        let mut v = Value::Array(vec![Value::U8(1)]);
        v.as_array_mut().unwrap().push(Value::U8(2));
        assert_eq!(v.as_array().unwrap().len(), 2);

        v.to_array_mut().unwrap().push(Value::U8(3));
        assert_eq!(v.as_array().unwrap().len(), 3);
    }

    #[test]
    fn to_array_mut_error() {
        let mut v = Value::Bool(true);
        assert!(v.to_array_mut().is_err());
    }

    #[test]
    fn map_accessors() {
        let v = Value::Map(vec![(Value::Text("k".into()), Value::U8(1))]);
        assert!(v.is_map());
        assert_eq!(v.as_map().unwrap().len(), 1);
        assert_eq!(v.to_map().unwrap().len(), 1);
        assert_eq!(v.to_map_ref().unwrap().len(), 1);
    }

    #[test]
    fn map_error_paths() {
        let v = Value::Bool(true);
        assert!(v.to_map().is_err());
        assert!(v.to_map_ref().is_err());
        assert!(v.into_map().is_err());
    }

    #[test]
    fn map_mut() {
        let mut v = Value::Map(vec![(Value::Text("k".into()), Value::U8(1))]);
        v.as_map_mut()
            .unwrap()
            .push((Value::Text("k2".into()), Value::U8(2)));
        assert_eq!(v.as_map().unwrap().len(), 2);

        v.to_map_mut()
            .unwrap()
            .push((Value::Text("k3".into()), Value::U8(3)));
        assert_eq!(v.as_map().unwrap().len(), 3);

        let map = v.as_map_mut_ref().unwrap();
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn map_mut_error() {
        let mut v = Value::Bool(true);
        assert!(v.to_map_mut().is_err());
        assert!(v.as_map_mut_ref().is_err());
    }
}

// ===========================================================================
// display.rs
// ===========================================================================

mod display_tests {
    use super::*;

    #[test]
    fn display_all_variants() {
        // Test string_representation for all variants
        let cases = vec![
            (Value::U128(42), "(u128)42"),
            (Value::I128(-42), "(i128)-42"),
            (Value::U64(42), "(u64)42"),
            (Value::I64(-42), "(i64)-42"),
            (Value::U32(42), "(u32)42"),
            (Value::I32(-42), "(i32)-42"),
            (Value::U16(42), "(u16)42"),
            (Value::I16(-42), "(i16)-42"),
            (Value::U8(42), "(u8)42"),
            (Value::I8(-42), "(i8)-42"),
            (Value::Float(3.14), "float 3.14"),
            (Value::Bool(true), "bool true"),
            (Value::Bool(false), "bool false"),
            (Value::Null, "Null"),
            (Value::Text("short".into()), "string short"),
        ];

        for (val, expected) in cases {
            let s = format!("{}", val);
            assert_eq!(s, expected, "Display mismatch for {:?}", val);
        }
    }

    #[test]
    fn display_long_text_truncates() {
        let long_str = "a".repeat(30);
        let val = Value::Text(long_str);
        let s = format!("{}", val);
        assert!(s.contains("[...(30)]"));
    }

    #[test]
    fn display_bytes() {
        let val = Value::Bytes(vec![0xAB, 0xCD]);
        let s = format!("{}", val);
        assert_eq!(s, "bytes abcd");
    }

    #[test]
    fn display_array() {
        let val = Value::Array(vec![Value::U8(1), Value::U8(2)]);
        let s = format!("{}", val);
        assert!(s.starts_with("array of ["));
    }

    #[test]
    fn display_map() {
        let val = Value::Map(vec![(Value::Text("k".into()), Value::U8(1))]);
        let s = format!("{}", val);
        assert!(s.starts_with("Map {"));
    }

    #[test]
    fn display_enum_variants() {
        assert_eq!(format!("{}", Value::EnumU8(vec![1])), "enum u8");
        assert_eq!(
            format!("{}", Value::EnumString(vec!["a".into()])),
            "enum string"
        );
    }

    #[test]
    fn non_qualified_string_representation() {
        // Test non_qualified_string_representation for key variants
        let val = Value::Float(3.14);
        assert_eq!(val.non_qualified_string_representation(), "3.14");

        let val = Value::Text("hello".into());
        assert_eq!(val.non_qualified_string_representation(), "hello");

        let val = Value::Bool(true);
        assert_eq!(val.non_qualified_string_representation(), "true");

        let val = Value::U64(42);
        assert_eq!(val.non_qualified_string_representation(), "42");

        let val = Value::Null;
        assert_eq!(val.non_qualified_string_representation(), "Null");
    }
}

// ===========================================================================
// eq.rs
// ===========================================================================

mod eq_tests {
    use super::*;

    #[test]
    fn partial_eq_integers() {
        assert!(Value::U64(42) == 42u64);
        assert!(Value::I64(-5) == -5i64);
        assert!(Value::U32(10) == 10u32);
        assert!(Value::I32(-10) == -10i32);
        assert!(Value::U16(5) == 5u16);
        assert!(Value::I16(-5) == -5i16);
        assert!(Value::U8(3) == 3u8);
        assert!(Value::I8(-3) == -3i8);
        assert!(Value::U128(100) == 100u128);
        assert!(Value::I128(-100) == -100i128);

        assert!(Value::Text("x".into()) != 42u64);
    }

    #[test]
    fn partial_eq_ref_integers() {
        let v = Value::U64(42);
        assert!(&v == &42u64);
        let v = Value::Text("x".into());
        assert!(&v != &42u64);
    }

    #[test]
    fn partial_eq_string() {
        let s = String::from("hello");
        assert!(Value::Text("hello".into()) == s);
        assert!(Value::U64(1) != s);
    }

    #[test]
    fn partial_eq_ref_string() {
        let v = Value::Text("hello".into());
        let s = String::from("hello");
        assert!(&v == &s);
        let v = Value::U64(1);
        assert!(&v != &s);
    }

    #[test]
    fn partial_eq_str() {
        assert!(Value::Text("hi".into()) == "hi");
        assert!(Value::U64(1) != "hi");
    }

    #[test]
    fn partial_eq_ref_str() {
        let v = Value::Text("hi".into());
        assert!(&v == &"hi");
        let v = Value::U64(1);
        assert!(&v != &"hi");
    }

    #[test]
    fn partial_eq_float() {
        assert!(Value::Float(3.14) == 3.14f64);
        assert!(Value::U64(1) != 3.14f64);
    }

    #[test]
    fn partial_eq_ref_float() {
        let v = Value::Float(3.14);
        assert!(&v == &3.14f64);
        let v = Value::U64(1);
        assert!(&v != &3.14f64);
    }

    #[test]
    fn partial_eq_vec_u8() {
        let bytes = vec![1u8, 2, 3];
        assert!(Value::Bytes(vec![1, 2, 3]) == bytes);
        assert!(Value::U64(1) != bytes);
    }

    #[test]
    fn partial_eq_ref_vec_u8() {
        let v = Value::Bytes(vec![1, 2, 3]);
        let bytes = vec![1u8, 2, 3];
        assert!(&v == &bytes);
        let v = Value::U64(1);
        assert!(&v != &bytes);
    }

    #[test]
    fn partial_eq_byte_arrays() {
        assert!(Value::Bytes20([1; 20]) == [1u8; 20]);
        assert!(&Value::Bytes20([1; 20]) == &[1u8; 20]);
        assert!(Value::Bytes32([1; 32]) == [1u8; 32]);
        assert!(&Value::Bytes32([1; 32]) == &[1u8; 32]);
        assert!(Value::Bytes36([1; 36]) == [1u8; 36]);
        assert!(&Value::Bytes36([1; 36]) == &[1u8; 36]);
    }

    #[test]
    fn equal_underlying_data_bytes() {
        // Same content, different variant
        let a = Value::Bytes(vec![1; 32]);
        let b = Value::Identifier([1; 32]);
        assert!(a.equal_underlying_data(&b));
        assert!(b.equal_underlying_data(&a));

        // Different content
        let c = Value::Bytes(vec![2; 32]);
        assert!(!a.equal_underlying_data(&c));
    }

    #[test]
    fn equal_underlying_data_integers() {
        let a = Value::U64(42);
        let b = Value::I128(42);
        assert!(a.equal_underlying_data(&b));

        let c = Value::U32(42);
        assert!(a.equal_underlying_data(&c));
    }

    #[test]
    fn equal_underlying_data_fallback() {
        // Non-bytes, non-integer: falls back to PartialEq
        let a = Value::Text("hi".into());
        let b = Value::Text("hi".into());
        assert!(a.equal_underlying_data(&b));

        let c = Value::Text("bye".into());
        assert!(!a.equal_underlying_data(&c));
    }

    #[test]
    fn equal_underlying_data_u128_overflow() {
        // U128 > i128::MAX should fail as_i128_unified
        let a = Value::U128(u128::MAX);
        let b = Value::U64(0);
        assert!(!a.equal_underlying_data(&b));
    }
}

// ===========================================================================
// string_encoding.rs
// ===========================================================================

mod string_encoding_tests {
    use platform_value::string_encoding::{decode, encode, Encoding, ALL_ENCODINGS};

    #[test]
    fn encode_decode_roundtrip_base58() {
        let data = vec![1, 2, 3, 4, 5];
        let encoded = encode(&data, Encoding::Base58);
        let decoded = decode(&encoded, Encoding::Base58).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_decode_roundtrip_base64() {
        let data = vec![10, 20, 30, 40, 50];
        let encoded = encode(&data, Encoding::Base64);
        let decoded = decode(&encoded, Encoding::Base64).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_decode_roundtrip_hex() {
        let data = vec![0xAB, 0xCD, 0xEF];
        let encoded = encode(&data, Encoding::Hex);
        assert_eq!(encoded, "abcdef");
        let decoded = decode(&encoded, Encoding::Hex).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_invalid_input() {
        assert!(decode("not_valid_hex!!!", Encoding::Hex).is_err());
        assert!(decode("!!!not_base64!!!", Encoding::Base64).is_err());
        // Base58 invalid chars
        assert!(decode("0OIl", Encoding::Base58).is_err());
    }

    #[test]
    fn encoding_display() {
        assert_eq!(format!("{}", Encoding::Base58), "Base58");
        assert_eq!(format!("{}", Encoding::Base64), "Base64");
        assert_eq!(format!("{}", Encoding::Hex), "Hex");
    }

    #[test]
    fn all_encodings_constant() {
        assert_eq!(ALL_ENCODINGS.len(), 3);
    }
}

// ===========================================================================
// value_map.rs
// ===========================================================================

mod value_map_tests {
    use super::*;

    #[test]
    fn sort_by_keys() {
        let mut map: ValueMap = vec![
            (Value::Text("b".into()), Value::U8(2)),
            (Value::Text("a".into()), Value::U8(1)),
        ];
        map.sort_by_keys();
        assert_eq!(map[0].0, Value::Text("a".into()));
    }

    #[test]
    fn sort_by_keys_and_inner_maps() {
        let inner_map = Value::Map(vec![
            (Value::Text("y".into()), Value::U8(2)),
            (Value::Text("x".into()), Value::U8(1)),
        ]);
        let mut map: ValueMap = vec![
            (Value::Text("b".into()), inner_map),
            (Value::Text("a".into()), Value::U8(0)),
        ];
        map.sort_by_keys_and_inner_maps();
        assert_eq!(map[0].0, Value::Text("a".into()));
        // inner map should also be sorted
        if let Value::Map(inner) = &map[1].1 {
            assert_eq!(inner[0].0, Value::Text("x".into()));
        }
    }

    #[test]
    fn sort_by_lexicographical_byte_ordering() {
        let mut map: ValueMap = vec![
            (Value::Text("bb".into()), Value::U8(2)),
            (Value::Text("a".into()), Value::U8(1)),
        ];
        map.sort_by_lexicographical_byte_ordering_keys();
        // "a" (len 1) < "bb" (len 2)
        assert_eq!(map[0].0, Value::Text("a".into()));
    }

    #[test]
    fn sort_by_lexicographical_byte_ordering_and_inner_maps() {
        let inner_map = Value::Map(vec![
            (Value::Text("yy".into()), Value::U8(2)),
            (Value::Text("x".into()), Value::U8(1)),
        ]);
        let mut map: ValueMap = vec![
            (Value::Text("bb".into()), inner_map),
            (Value::Text("a".into()), Value::U8(0)),
        ];
        map.sort_by_lexicographical_byte_ordering_keys_and_inner_maps();
        assert_eq!(map[0].0, Value::Text("a".into()));
    }

    #[test]
    fn get_key_and_optional_key() {
        let map: ValueMap = vec![
            (Value::Text("a".into()), Value::U8(1)),
            (Value::U8(42), Value::U8(99)), // non-text key
        ];
        assert_eq!(*map.get_key("a").unwrap(), Value::U8(1));
        assert!(map.get_key("missing").is_err());
        assert_eq!(*map.get_optional_key("a").unwrap(), Value::U8(1));
        assert!(map.get_optional_key("missing").is_none());
    }

    #[test]
    fn get_key_mut_and_insert() {
        let mut map: ValueMap = vec![(Value::Text("a".into()), Value::U8(1))];
        *map.get_key_mut("a").unwrap() = Value::U8(99);
        assert_eq!(*map.get_key("a").unwrap(), Value::U8(99));
        assert!(map.get_key_mut("missing").is_err());
    }

    #[test]
    fn get_key_mut_or_insert() {
        let mut map: ValueMap = vec![(Value::Text("a".into()), Value::U8(1))];
        // Existing key
        let v = map.get_key_mut_or_insert("a", Value::U8(99));
        assert_eq!(*v, Value::U8(1));
        // New key
        let v = map.get_key_mut_or_insert("b", Value::U8(42));
        assert_eq!(*v, Value::U8(42));
    }

    #[test]
    fn get_key_by_value_mut_or_insert() {
        let mut map: ValueMap = vec![(Value::Text("a".into()), Value::U8(1))];
        let key = Value::Text("a".into());
        let v = map.get_key_by_value_mut_or_insert(&key, Value::U8(99));
        assert_eq!(*v, Value::U8(1));
        let new_key = Value::Text("b".into());
        let v = map.get_key_by_value_mut_or_insert(&new_key, Value::U8(42));
        assert_eq!(*v, Value::U8(42));
    }

    #[test]
    fn insert_string_key_value() {
        let mut map: ValueMap = vec![];
        map.insert_string_key_value("key".to_string(), Value::U8(1));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn remove_key() {
        let mut map: ValueMap = vec![
            (Value::Text("a".into()), Value::U8(1)),
            (Value::U8(42), Value::U8(99)), // non-text key
        ];
        assert_eq!(map.remove_key("a").unwrap(), Value::U8(1));
        assert!(map.remove_key("missing").is_err());
    }

    #[test]
    fn remove_optional_key() {
        let mut map: ValueMap = vec![(Value::Text("a".into()), Value::U8(1))];
        assert_eq!(map.remove_optional_key("a"), Some(Value::U8(1)));
        assert_eq!(map.remove_optional_key("missing"), None);
    }

    #[test]
    fn remove_optional_key_if_null() {
        let mut map: ValueMap = vec![
            (Value::Text("a".into()), Value::Null),
            (Value::Text("b".into()), Value::U8(1)),
        ];
        map.remove_optional_key_if_null("a");
        assert_eq!(map.len(), 1);
        // "b" is not null so it stays
        map.remove_optional_key_if_null("b");
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn remove_optional_key_if_empty_array() {
        let mut map: ValueMap = vec![
            (Value::Text("a".into()), Value::Array(vec![])),
            (Value::Text("b".into()), Value::Array(vec![Value::U8(1)])),
            (Value::Text("c".into()), Value::U8(1)),
        ];
        map.remove_optional_key_if_empty_array("a");
        assert_eq!(map.len(), 2);
        // "b" has non-empty array, stays
        map.remove_optional_key_if_empty_array("b");
        assert_eq!(map.len(), 2);
        // "c" is not an array, stays
        map.remove_optional_key_if_empty_array("c");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn remove_optional_key_value() {
        let mut map: ValueMap = vec![(Value::Text("a".into()), Value::U8(1))];
        let key = Value::Text("a".into());
        assert_eq!(map.remove_optional_key_value(&key), Some(Value::U8(1)));
        let missing = Value::Text("missing".into());
        assert_eq!(map.remove_optional_key_value(&missing), None);
    }

    #[test]
    fn from_btree_map() {
        let mut btree = BTreeMap::new();
        btree.insert("a".to_string(), Value::U8(1));
        btree.insert("b".to_string(), Value::U8(2));
        let map = ValueMap::from_btree_map(btree);
        assert_eq!(map.len(), 2);
    }
}

// ===========================================================================
// inner_value.rs - map operations
// ===========================================================================

mod inner_value_tests {
    use super::*;

    fn make_map() -> Value {
        platform_value!({
            "name": "test",
            "count": 42,
            "active": true,
            "data": [1, 2, 3],
            "nested": {
                "inner_key": "inner_val"
            }
        })
    }

    #[test]
    fn has_key() {
        let v = make_map();
        assert!(v.has("name").unwrap());
        assert!(!v.has("nonexistent").unwrap());
    }

    #[test]
    fn get_and_get_value() {
        let v = make_map();
        assert!(v.get("name").unwrap().is_some());
        assert!(v.get("nonexistent").unwrap().is_none());
        assert!(v.get_value("name").is_ok());
    }

    #[test]
    fn get_mut_and_set() {
        let mut v = make_map();
        let inner = v.get_mut("count").unwrap().unwrap();
        assert!(inner.is_integer());

        v.set_value("count", Value::U64(99)).unwrap();
        let c: u64 = v.get_integer("count").unwrap();
        assert_eq!(c, 99);
    }

    #[test]
    fn set_into_value() {
        let mut v = platform_value!({ "x": 1 });
        v.set_into_value("x", 42u32).unwrap();
        let x: u32 = v.get_integer("x").unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn set_into_binary_data() {
        let mut v = platform_value!({ "x": 1 });
        v.set_into_binary_data("data", vec![1, 2, 3]).unwrap();
        assert!(v.get("data").unwrap().is_some());
    }

    #[test]
    fn insert_and_insert_at_end() {
        let mut v = platform_value!({ "a": 1 });
        v.insert("b".to_string(), Value::U8(2)).unwrap();
        v.insert_at_end("c".to_string(), Value::U8(3)).unwrap();
        assert!(v.has("b").unwrap());
        assert!(v.has("c").unwrap());
    }

    #[test]
    fn remove_and_remove_many() {
        let mut v = platform_value!({ "a": 1, "b": 2, "c": 3 });
        let removed = v.remove("a").unwrap();
        assert!(removed == 1i32);
        v.remove_many(&["b", "c"]).unwrap();
        assert!(!v.has("b").unwrap());
    }

    #[test]
    fn remove_optional_value() {
        let mut v = platform_value!({ "a": 1 });
        assert!(v.remove_optional_value("a").unwrap().is_some());
        assert!(v.remove_optional_value("missing").unwrap().is_none());
    }

    #[test]
    fn remove_optional_value_if_null() {
        let mut v = Value::Map(vec![
            (Value::Text("a".into()), Value::Null),
            (Value::Text("b".into()), Value::U8(1)),
        ]);
        v.remove_optional_value_if_null("a").unwrap();
        // b should still be there
        assert!(v.has("b").unwrap());
    }

    #[test]
    fn remove_optional_value_if_empty_array() {
        let mut v = Value::Map(vec![
            (Value::Text("a".into()), Value::Array(vec![])),
            (Value::Text("b".into()), Value::U8(1)),
        ]);
        v.remove_optional_value_if_empty_array("a").unwrap();
        assert!(v.has("b").unwrap());
    }

    #[test]
    fn get_typed_values() {
        let v = make_map();
        let count: u64 = v.get_integer("count").unwrap();
        assert_eq!(count, 42);

        let opt_count: Option<u64> = v.get_optional_integer("count").unwrap();
        assert_eq!(opt_count, Some(42));

        let name = v.get_str("name").unwrap();
        assert_eq!(name, "test");

        let opt_name = v.get_optional_str("name").unwrap();
        assert_eq!(opt_name, Some("test"));

        let active = v.get_bool("active").unwrap();
        assert!(active);

        let opt_active = v.get_optional_bool("active").unwrap();
        assert_eq!(opt_active, Some(true));

        let missing_int: Option<u64> = v.get_optional_integer("nonexistent").unwrap();
        assert_eq!(missing_int, None);
    }

    #[test]
    fn get_array() {
        let v = make_map();
        let arr = v.get_array("data").unwrap();
        assert_eq!(arr.len(), 3);

        let opt_arr = v.get_optional_array("data").unwrap();
        assert!(opt_arr.is_some());

        let missing_arr = v.get_optional_array("nonexistent").unwrap();
        assert!(missing_arr.is_none());
    }

    #[test]
    fn get_array_slice_and_ref() {
        let v = make_map();
        let slice = v.get_array_slice("data").unwrap();
        assert_eq!(slice.len(), 3);

        let arr_ref = v.get_array_ref("data").unwrap();
        assert_eq!(arr_ref.len(), 3);

        let opt_slice = v.get_optional_array_slice("data").unwrap();
        assert!(opt_slice.is_some());
    }

    #[test]
    fn remove_integer() {
        let mut v = platform_value!({ "x": 42 });
        let x: u64 = v.remove_integer("x").unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn remove_optional_integer() {
        let mut v = platform_value!({ "x": 42 });
        let x: Option<u64> = v.remove_optional_integer("x").unwrap();
        assert_eq!(x, Some(42));
        let y: Option<u64> = v.remove_optional_integer("missing").unwrap();
        assert_eq!(y, None);
    }

    #[test]
    fn remove_optional_integer_null_value() {
        let mut v = Value::Map(vec![(Value::Text("x".into()), Value::Null)]);
        let x: Option<u64> = v.remove_optional_integer("x").unwrap();
        assert_eq!(x, None);
    }

    #[test]
    fn remove_bytes_and_binary_data() {
        let mut v = Value::Map(vec![(
            Value::Text("data".into()),
            Value::Bytes(vec![1, 2, 3]),
        )]);
        let data = v.remove_bytes("data").unwrap();
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn remove_optional_bytes() {
        let mut v = Value::Map(vec![(
            Value::Text("data".into()),
            Value::Bytes(vec![1, 2, 3]),
        )]);
        let data = v.remove_optional_bytes("data").unwrap();
        assert!(data.is_some());
        let missing = v.remove_optional_bytes("missing").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn remove_binary_data() {
        let mut v = Value::Map(vec![(Value::Text("data".into()), Value::Bytes(vec![1, 2]))]);
        let bd = v.remove_binary_data("data").unwrap();
        assert_eq!(bd, BinaryData::new(vec![1, 2]));
    }

    #[test]
    fn remove_optional_binary_data() {
        let mut v = Value::Map(vec![(Value::Text("data".into()), Value::Bytes(vec![1, 2]))]);
        assert!(v.remove_optional_binary_data("data").unwrap().is_some());
        assert!(v.remove_optional_binary_data("missing").unwrap().is_none());
    }

    #[test]
    fn remove_array() {
        let mut v = platform_value!({ "arr": [1, 2, 3] });
        let arr = v.remove_array("arr").unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn remove_optional_array() {
        let mut v = platform_value!({ "arr": [1] });
        assert!(v.remove_optional_array("arr").unwrap().is_some());
        assert!(v.remove_optional_array("missing").unwrap().is_none());
    }

    #[test]
    fn get_optional_bool_null() {
        let v = Value::Map(vec![(Value::Text("x".into()), Value::Null)]);
        let result = v.get_optional_bool("x").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_integer_null() {
        let v = Value::Map(vec![(Value::Text("x".into()), Value::Null)]);
        let result: Option<u64> = v.get_optional_integer("x").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn insert_in_map_replaces_existing() {
        let mut map: ValueMap = vec![(Value::Text("k".into()), Value::U8(1))];
        Value::insert_in_map(&mut map, "k", Value::U8(99));
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].1, Value::U8(99));
    }

    #[test]
    fn insert_in_map_string_value_inserts_sorted() {
        let mut map: ValueMap = vec![
            (Value::Text("a".into()), Value::U8(1)),
            (Value::Text("c".into()), Value::U8(3)),
        ];
        Value::insert_in_map_string_value(&mut map, "b".to_string(), Value::U8(2));
        assert_eq!(map.len(), 3);
        assert_eq!(map[1].0, Value::Text("b".into()));
    }

    #[test]
    fn push_to_map_string_value() {
        let mut map: ValueMap = vec![(Value::Text("a".into()), Value::U8(1))];
        // Replace existing
        Value::push_to_map_string_value(&mut map, "a".to_string(), Value::U8(99));
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].1, Value::U8(99));
        // Add new
        Value::push_to_map_string_value(&mut map, "b".to_string(), Value::U8(2));
        assert_eq!(map.len(), 2);
    }
}

// ===========================================================================
// inner_value_at_path.rs
// ===========================================================================

mod inner_value_at_path_tests {
    use super::*;

    #[test]
    fn get_value_at_path() {
        let v = platform_value!({
            "a": { "b": { "c": 42 } }
        });
        let c = v.get_value_at_path("a.b.c").unwrap();
        assert!(c == &42i32);
    }

    #[test]
    fn get_optional_value_at_path_missing() {
        let v = platform_value!({ "a": { "b": 1 } });
        assert!(v.get_optional_value_at_path("a.missing").unwrap().is_none());
    }

    #[test]
    fn get_mut_value_at_path() {
        let mut v = platform_value!({ "a": { "b": 1 } });
        let b = v.get_mut_value_at_path("a.b").unwrap();
        *b = Value::U64(99);
        assert_eq!(*v.get_value_at_path("a.b").unwrap(), Value::U64(99));
    }

    #[test]
    fn get_optional_mut_value_at_path() {
        let mut v = platform_value!({ "a": { "b": 1 } });
        assert!(v
            .get_optional_mut_value_at_path("a.missing")
            .unwrap()
            .is_none());
        assert!(v.get_optional_mut_value_at_path("a.b").unwrap().is_some());
    }

    #[test]
    fn remove_value_at_path() {
        let mut v = platform_value!({ "a": { "b": 42 } });
        let removed = v.remove_value_at_path("a.b").unwrap();
        assert!(removed == 42i32);
    }

    #[test]
    fn remove_optional_value_at_path() {
        let mut v = platform_value!({ "a": { "b": 42 } });
        assert!(v.remove_optional_value_at_path("a.b").unwrap().is_some());
        assert!(v
            .remove_optional_value_at_path("a.missing")
            .unwrap()
            .is_none());
    }

    #[test]
    fn remove_optional_value_at_path_missing_intermediate() {
        let mut v = platform_value!({ "a": { "b": 42 } });
        assert!(v.remove_optional_value_at_path("a.x.y").unwrap().is_none());
    }

    #[test]
    fn get_integer_at_path() {
        let v = platform_value!({ "a": { "b": 42 } });
        let val: u64 = v.get_integer_at_path("a.b").unwrap();
        assert_eq!(val, 42);
    }

    #[test]
    fn get_optional_integer_at_path() {
        let v = platform_value!({ "a": { "b": 42 } });
        let val: Option<u64> = v.get_optional_integer_at_path("a.b").unwrap();
        assert_eq!(val, Some(42));
        let missing: Option<u64> = v.get_optional_integer_at_path("a.missing").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn set_value_at_full_path() {
        let mut v = platform_value!({ "root": {} });
        v.set_value_at_full_path("root.x.y", Value::U8(1)).unwrap();
        let val = v.get_value_at_path("root.x.y").unwrap();
        assert_eq!(*val, Value::U8(1));
    }

    #[test]
    fn set_value_at_path() {
        let mut v = platform_value!({ "root": { "inner": {} } });
        v.set_value_at_path("root.inner", "key", Value::U8(42))
            .unwrap();
        let val = v.get_value_at_path("root.inner.key").unwrap();
        assert_eq!(*val, Value::U8(42));
    }

    #[test]
    fn remove_values_at_paths() {
        let mut v = platform_value!({ "a": 1, "b": 2 });
        let removed = v.remove_values_at_paths(vec!["a", "b"]).unwrap();
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn remove_values_matching_paths() {
        let mut v = platform_value!({
            "items": [
                { "x": 1 },
                { "x": 2 }
            ]
        });
        let removed = v.remove_values_matching_path("items[].x").unwrap();
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn get_value_at_path_with_array_index() {
        let v = platform_value!({
            "arr": [10, 20, 30]
        });
        let val = v.get_value_at_path("arr[1]").unwrap();
        assert!(val == &20i32);
    }

    #[test]
    fn get_optional_value_at_path_with_array_index() {
        let v = platform_value!({
            "arr": [10, 20]
        });
        let val = v.get_optional_value_at_path("arr[0]").unwrap();
        assert!(val.is_some());
        // Out of bounds
        let val = v.get_optional_value_at_path("arr[5]").unwrap();
        assert!(val.is_none());
    }
}

// ===========================================================================
// index.rs
// ===========================================================================

mod index_tests {
    use super::*;

    #[test]
    fn index_map_by_str() {
        let v = platform_value!({ "key": "value" });
        assert_eq!(v["key"], Value::Text("value".into()));
        assert_eq!(v["missing"], Value::Null);
    }

    #[test]
    fn index_array_by_usize() {
        let v = platform_value!([1, 2, 3]);
        assert!(v[0] == 1i32);
        assert!(v[2] == 3i32);
    }

    #[test]
    fn index_non_map_non_array() {
        let v = Value::U64(42);
        assert_eq!(v["key"], Value::Null);
        assert_eq!(v[0], Value::Null);
    }

    #[test]
    fn index_mut_map() {
        let mut v = platform_value!({ "x": 1 });
        v["x"] = Value::U64(99);
        assert_eq!(v["x"], Value::U64(99));
    }

    #[test]
    fn index_mut_insert_new() {
        let mut v = platform_value!({ "x": 1 });
        v["y"] = Value::U64(2);
        assert_eq!(v["y"], Value::U64(2));
    }

    #[test]
    fn index_mut_null_creates_map() {
        let mut v = Value::Null;
        v["key"] = Value::U64(1);
        assert_eq!(v["key"], Value::U64(1));
    }

    #[test]
    fn index_string_key() {
        let v = platform_value!({ "key": "value" });
        let key = String::from("key");
        assert_eq!(v[key.as_str()], Value::Text("value".into()));
    }

    #[test]
    fn index_by_ref() {
        let v = platform_value!({ "key": "value" });
        let key = "key";
        assert_eq!(v[&key], Value::Text("value".into()));
    }
}

// ===========================================================================
// pointer.rs
// ===========================================================================

mod pointer_tests {
    use super::*;

    #[test]
    fn pointer_basic() {
        let v = platform_value!({ "a": { "b": [1, 2, 3] } });
        assert!(v.pointer("/a/b/0").unwrap() == &1i32);
        assert!(v.pointer("/a/b/2").unwrap() == &3i32);
        assert_eq!(v.pointer("/a/c"), None);
    }

    #[test]
    fn pointer_empty_returns_self() {
        let v = platform_value!(42);
        let result = v.pointer("").unwrap();
        assert!(result == &42i32);
    }

    #[test]
    fn pointer_no_leading_slash() {
        let v = platform_value!({ "a": 1 });
        assert_eq!(v.pointer("a"), None);
    }

    #[test]
    fn pointer_mut_basic() {
        let mut v = platform_value!({ "a": { "b": 1 } });
        *v.pointer_mut("/a/b").unwrap() = Value::U64(99);
        assert_eq!(v.pointer("/a/b"), Some(&Value::U64(99)));
    }

    #[test]
    fn pointer_mut_empty_returns_self() {
        let mut v = platform_value!(42);
        let result = v.pointer_mut("").unwrap();
        assert!(result == &42i32);
    }

    #[test]
    fn pointer_mut_no_leading_slash() {
        let mut v = platform_value!({ "a": 1 });
        assert_eq!(v.pointer_mut("a"), None);
    }

    #[test]
    fn pointer_escape_sequences() {
        let v = platform_value!({ "a/b": { "c~d": 42 } });
        assert!(v.pointer("/a~1b/c~0d").unwrap() == &42i32);
    }

    #[test]
    fn take() {
        let mut v = platform_value!({ "x": "y" });
        let taken = v["x"].take();
        assert_eq!(taken, Value::Text("y".into()));
        assert_eq!(v["x"], Value::Null);
    }

    #[test]
    fn pointer_leading_zero_index() {
        let v = platform_value!([1, 2]);
        // "01" has leading zero, should return None
        assert_eq!(v.pointer("/01"), None);
    }
}

// ===========================================================================
// patch operations
// ===========================================================================

mod patch_tests {
    use super::*;

    #[test]
    fn patch_add_replace_remove() {
        let mut doc = platform_value!({ "a": 1 });
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/b", "value": 2 },
            { "op": "replace", "path": "/a", "value": 99 },
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert!(doc["a"] == 99i32);
        assert!(doc["b"] == 2i32);
    }

    #[test]
    fn patch_test_operation() {
        let mut doc = platform_value!({ "a": 1 });
        let p: Patch = from_value(platform_value!([
            { "op": "test", "path": "/a", "value": 1 },
        ]))
        .unwrap();
        assert!(patch(&mut doc, &p).is_ok());
    }

    #[test]
    fn patch_test_fails() {
        let mut doc = platform_value!({ "a": 1 });
        let p: Patch = from_value(platform_value!([
            { "op": "test", "path": "/a", "value": 2 },
        ]))
        .unwrap();
        assert!(patch(&mut doc, &p).is_err());
    }

    #[test]
    fn patch_move_operation() {
        let mut doc = platform_value!({ "a": 1, "b": 2 });
        let p: Patch = from_value(platform_value!([
            { "op": "move", "from": "/a", "path": "/c" },
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert!(doc["c"] == 1i32);
        assert_eq!(doc["a"], Value::Null);
    }

    #[test]
    fn patch_copy_operation() {
        let mut doc = platform_value!({ "a": 1 });
        let p: Patch = from_value(platform_value!([
            { "op": "copy", "from": "/a", "path": "/b" },
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert!(doc["a"] == 1i32);
        assert!(doc["b"] == 1i32);
    }

    #[test]
    fn patch_add_to_array() {
        let mut doc = platform_value!([1, 2, 3]);
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/1", "value": 99 },
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert!(doc[1] == 99i32);
        assert!(doc[3] == 3i32);
    }

    #[test]
    fn patch_add_to_array_end() {
        let mut doc = platform_value!([1, 2]);
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/-", "value": 3 },
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert!(doc[2] == 3i32);
    }

    #[test]
    fn patch_replace_root() {
        let mut doc = platform_value!({ "a": 1 });
        let p: Patch = from_value(platform_value!([
            { "op": "replace", "path": "", "value": 42 },
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert!(doc == 42i32);
    }

    #[test]
    fn merge_basic() {
        let mut doc = platform_value!({ "a": 1, "b": 2 });
        let p = platform_value!({ "a": 99, "c": 3 });
        merge(&mut doc, &p);
        assert!(doc["a"] == 99i32);
        assert!(doc["b"] == 2i32);
        assert!(doc["c"] == 3i32);
    }

    #[test]
    fn merge_remove_null() {
        let mut doc = platform_value!({ "a": 1, "b": 2 });
        let p = platform_value!({ "a": null });
        merge(&mut doc, &p);
        assert_eq!(doc["a"], Value::Null);
    }

    #[test]
    fn merge_non_map_patch_replaces() {
        let mut doc = platform_value!({ "a": 1 });
        let p = platform_value!(42);
        merge(&mut doc, &p);
        assert!(doc == 42i32);
    }

    #[test]
    fn merge_non_map_doc() {
        let mut doc = Value::U64(1);
        let p = platform_value!({ "a": 1 });
        merge(&mut doc, &p);
        assert!(doc.is_map());
    }

    #[test]
    fn diff_basic() {
        let left = platform_value!({ "a": 1, "b": 2 });
        let right = platform_value!({ "a": 1, "b": 3, "c": 4 });
        let p = diff(&left, &right);
        let mut doc = left.clone();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, right);
    }
}

// ===========================================================================
// inner_array_value.rs
// ===========================================================================

mod inner_array_tests {
    use super::*;

    #[test]
    fn push_to_array() {
        let mut v = Value::Array(vec![Value::U8(1)]);
        v.push(Value::U8(2)).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn push_to_non_array_errors() {
        let mut v = Value::U64(1);
        assert!(v.push(Value::U8(1)).is_err());
    }
}

// ===========================================================================
// converter/serde_json.rs (requires json feature)
// ===========================================================================

#[cfg(feature = "json")]
mod json_converter_tests {
    use super::*;
    use serde_json::Value as JsonValue;

    #[test]
    fn value_to_json_via_try_into() {
        let v = Value::U64(42);
        let j: JsonValue = v.try_into().unwrap();
        assert_eq!(j, JsonValue::Number(42.into()));
    }

    #[test]
    fn u128_to_json_as_string() {
        let v = Value::U128(u128::MAX);
        let j: JsonValue = v.try_into().unwrap();
        assert!(j.is_string());
    }

    #[test]
    fn i128_to_json_as_string() {
        let v = Value::I128(i128::MIN);
        let j: JsonValue = v.try_into().unwrap();
        assert!(j.is_string());
    }

    #[test]
    fn bytes_to_json_base64() {
        let v = Value::Bytes(vec![1, 2, 3]);
        let j: JsonValue = v.try_into().unwrap();
        assert!(j.is_string());
    }

    #[test]
    fn bytes20_to_json_base64() {
        let v = Value::Bytes20([1; 20]);
        let j: JsonValue = v.try_into().unwrap();
        assert!(j.is_string());
    }

    #[test]
    fn bytes32_to_json_base64() {
        let v = Value::Bytes32([1; 32]);
        let j: JsonValue = v.try_into().unwrap();
        assert!(j.is_string());
    }

    #[test]
    fn bytes36_to_json_base64() {
        let v = Value::Bytes36([1; 36]);
        let j: JsonValue = v.try_into().unwrap();
        assert!(j.is_string());
    }

    #[test]
    fn identifier_to_json_base58() {
        let v = Value::Identifier([5; 32]);
        let j: JsonValue = v.try_into().unwrap();
        assert!(j.is_string());
    }

    #[test]
    fn enum_u8_to_json_error() {
        let v = Value::EnumU8(vec![1]);
        let result: Result<JsonValue, Error> = v.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn enum_string_to_json_error() {
        let v = Value::EnumString(vec!["a".into()]);
        let result: Result<JsonValue, Error> = v.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn json_to_value_null() {
        let j = JsonValue::Null;
        let v: Value = j.into();
        assert!(v.is_null());
    }

    #[test]
    fn json_to_value_negative() {
        let j = serde_json::json!(-42);
        let v: Value = j.into();
        assert_eq!(v, Value::I64(-42));
    }

    #[test]
    fn json_to_value_float() {
        let j = serde_json::json!(3.14);
        let v: Value = j.into();
        assert!(v.is_float());
    }

    #[test]
    fn json_to_value_small_u8_array_stays_array() {
        // Arrays < 10 elements should stay as arrays, not bytes
        let j = serde_json::json!([1, 2, 3]);
        let v: Value = j.into();
        assert!(v.is_array());
    }

    #[test]
    fn json_to_value_large_u8_array_becomes_bytes() {
        // Arrays >= 10 elements of u8 should become bytes
        let j = serde_json::json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let v: Value = j.into();
        assert!(v.is_bytes());
    }

    #[test]
    fn json_ref_to_value() {
        let j = serde_json::json!({ "key": "value" });
        let v: Value = (&j).into();
        assert!(v.is_map());
    }

    #[test]
    fn try_into_validating_json_u128_overflow() {
        let v = Value::U128(u128::MAX);
        assert!(v.try_into_validating_json().is_err());
    }

    #[test]
    fn try_into_validating_json_i128_overflow() {
        let v = Value::I128(i128::MAX);
        assert!(v.try_into_validating_json().is_err());
        let v = Value::I128(i128::MIN);
        assert!(v.try_into_validating_json().is_err());
    }

    #[test]
    fn try_to_validating_json() {
        let v = Value::U64(42);
        let j = v.try_to_validating_json().unwrap();
        assert_eq!(j, JsonValue::Number(42.into()));
    }

    #[test]
    fn try_into_validating_btree_map_json() {
        let v = platform_value!({ "key": 42 });
        let result = v.try_into_validating_btree_map_json();
        assert!(result.is_ok());
    }

    #[test]
    fn convert_from_serde_json_map() {
        let map = vec![("key".to_string(), serde_json::json!(42))];
        let result: BTreeMap<String, Value> = Value::convert_from_serde_json_map(map);
        assert!(result.contains_key("key"));
    }

    #[test]
    fn from_btreemap_string_json_value() {
        let mut m = BTreeMap::new();
        m.insert("key".to_string(), serde_json::json!(42));
        let v: Value = m.into();
        assert!(v.is_map());
    }

    #[test]
    fn from_ref_btreemap_string_json_value() {
        let mut m = BTreeMap::new();
        m.insert("key".to_string(), serde_json::json!(42));
        let v: Value = (&m).into();
        assert!(v.is_map());
    }

    #[test]
    fn btree_value_json_converter() {
        use platform_value::converter::serde_json::BTreeValueJsonConverter;
        let v = platform_value!({ "key": 42 });
        let btree = v.into_btree_string_map().unwrap();

        let json = btree.to_json_value().unwrap();
        assert!(json.is_object());

        let validating_json = btree.to_validating_json_value().unwrap();
        assert!(validating_json.is_object());

        let json2 = btree.clone().into_json_value().unwrap();
        assert!(json2.is_object());

        let validating_json2 = btree.clone().into_validating_json_value().unwrap();
        assert!(validating_json2.is_object());

        let roundtrip: BTreeMap<String, Value> =
            BTreeValueJsonConverter::from_json_value(json).unwrap();
        assert!(roundtrip.contains_key("key"));
    }

    #[test]
    fn try_to_validating_json_all_types() {
        // Ensure all types produce valid JSON
        let vals = vec![
            Value::U32(1),
            Value::I32(-1),
            Value::U16(1),
            Value::I16(-1),
            Value::U8(1),
            Value::I8(-1),
            Value::Float(1.0),
            Value::Text("hi".into()),
            Value::Bool(true),
            Value::Null,
            Value::Bytes(vec![1, 2]),
            Value::Bytes20([0; 20]),
            Value::Bytes32([0; 32]),
            Value::Bytes36([0; 36]),
            Value::Identifier([0; 32]),
            Value::Array(vec![Value::U8(1)]),
            platform_value!({ "k": 1 }),
        ];
        for v in vals {
            assert!(v.try_to_validating_json().is_ok(), "Failed for {:?}", v);
        }
    }

    #[test]
    fn try_to_validating_json_enum_errors() {
        assert!(Value::EnumU8(vec![1]).try_to_validating_json().is_err());
        assert!(Value::EnumString(vec!["a".into()])
            .try_to_validating_json()
            .is_err());
    }

    #[test]
    fn try_into_validating_json_enum_errors() {
        assert!(Value::EnumU8(vec![1]).try_into_validating_json().is_err());
        assert!(Value::EnumString(vec!["a".into()])
            .try_into_validating_json()
            .is_err());
    }
}

// ===========================================================================
// converter/ciborium.rs (requires cbor feature)
// ===========================================================================

#[cfg(feature = "cbor")]
mod cbor_converter_tests {
    use super::*;
    use ciborium::Value as CborValue;

    #[test]
    fn value_to_cbor_integer_types() {
        let vals: Vec<Value> = vec![
            Value::U128(42),
            Value::I128(-42),
            Value::U64(42),
            Value::I64(-42),
            Value::U32(42),
            Value::I32(-42),
            Value::U16(42),
            Value::I16(-42),
            Value::U8(42),
            Value::I8(-42),
        ];
        for v in vals {
            let cv: CborValue = v.clone().try_into().unwrap();
            assert!(cv.is_integer(), "Failed for {:?}", v);
        }
    }

    #[test]
    fn value_to_cbor_bytes() {
        let v = Value::Bytes(vec![1, 2, 3]);
        let cv: CborValue = v.try_into().unwrap();
        assert!(cv.as_bytes().is_some());
    }

    #[test]
    fn value_to_cbor_typed_bytes() {
        let v = Value::Bytes20([1; 20]);
        let cv: CborValue = v.try_into().unwrap();
        assert!(cv.as_bytes().is_some());

        let v = Value::Bytes32([1; 32]);
        let cv: CborValue = v.try_into().unwrap();
        assert!(cv.as_bytes().is_some());

        let v = Value::Bytes36([1; 36]);
        let cv: CborValue = v.try_into().unwrap();
        assert!(cv.as_bytes().is_some());
    }

    #[test]
    fn value_to_cbor_identifier() {
        let v = Value::Identifier([5; 32]);
        let cv: CborValue = v.try_into().unwrap();
        assert!(cv.as_bytes().is_some());
    }

    #[test]
    fn value_to_cbor_other_types() {
        let cv: CborValue = Value::Float(3.14).try_into().unwrap();
        assert!(cv.as_float().is_some());

        let cv: CborValue = Value::Text("hi".into()).try_into().unwrap();
        assert!(cv.as_text().is_some());

        let cv: CborValue = Value::Bool(true).try_into().unwrap();
        assert_eq!(cv.as_bool(), Some(true));

        let cv: CborValue = Value::Null.try_into().unwrap();
        assert!(cv.is_null());
    }

    #[test]
    fn value_to_cbor_array() {
        let v = Value::Array(vec![Value::U8(1), Value::U8(2)]);
        let cv: CborValue = v.try_into().unwrap();
        assert!(cv.as_array().is_some());
    }

    #[test]
    fn value_to_cbor_map() {
        let v = platform_value!({ "key": 42 });
        let cv: CborValue = v.try_into().unwrap();
        assert!(cv.as_map().is_some());
    }

    #[test]
    fn value_to_cbor_enum_errors() {
        let result: Result<CborValue, Error> = Value::EnumU8(vec![1]).try_into();
        assert!(result.is_err());
        let result: Result<CborValue, Error> = Value::EnumString(vec!["a".into()]).try_into();
        assert!(result.is_err());
    }

    #[test]
    fn cbor_to_value_integer() {
        let cv = CborValue::Integer(42.into());
        let v: Value = cv.try_into().unwrap();
        assert!(v.is_integer());
    }

    #[test]
    fn cbor_to_value_bytes() {
        let cv = CborValue::Bytes(vec![1, 2, 3]);
        let v: Value = cv.try_into().unwrap();
        assert!(v.is_bytes());
    }

    #[test]
    fn cbor_to_value_tag_error() {
        let cv = CborValue::Tag(1, Box::new(CborValue::Null));
        let result: Result<Value, Error> = cv.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn cbor_to_value_null() {
        let cv = CborValue::Null;
        let v: Value = cv.try_into().unwrap();
        assert!(v.is_null());
    }

    #[test]
    fn to_cbor_buffer() {
        let v = platform_value!({ "key": 42 });
        let buf = v.to_cbor_buffer().unwrap();
        assert!(!buf.is_empty());
    }

    #[test]
    fn box_value_to_box_cbor() {
        let bv = Box::new(Value::U64(42));
        let result: Result<Box<CborValue>, Error> = bv.try_into();
        assert!(result.is_ok());
    }

    #[test]
    fn convert_from_cbor_map() {
        let map = vec![("key".to_string(), CborValue::Integer(42.into()))];
        let result: BTreeMap<String, Value> = Value::convert_from_cbor_map(map).unwrap();
        assert!(result.contains_key("key"));
    }

    #[test]
    fn convert_to_cbor_map() {
        let map = vec![("key".to_string(), Value::U64(42))];
        let result: BTreeMap<String, CborValue> = Value::convert_to_cbor_map(map).unwrap();
        assert!(result.contains_key("key"));
    }
}

// ===========================================================================
// value_serialization
// ===========================================================================

mod serde_roundtrip {
    use super::*;

    #[test]
    fn serialize_deserialize_option() {
        let v: Option<u32> = Some(42);
        let pv = to_value(v).unwrap();
        let back: Option<u32> = from_value(pv).unwrap();
        assert_eq!(back, Some(42));

        let none: Option<u32> = None;
        let pv = to_value(none).unwrap();
        assert!(pv.is_null());
    }

    #[test]
    fn serialize_various_types() {
        let pv = to_value(42u8).unwrap();
        assert!(pv.is_integer());

        let pv = to_value(42u16).unwrap();
        assert!(pv.is_integer());

        let pv = to_value(42u32).unwrap();
        assert!(pv.is_integer());

        let pv = to_value(42u64).unwrap();
        assert!(pv.is_integer());

        let pv = to_value(true).unwrap();
        assert!(pv.is_bool());

        let pv = to_value(3.14f64).unwrap();
        assert!(pv.is_float());

        let pv = to_value("hello").unwrap();
        assert!(pv.is_text());
    }

    #[test]
    fn deserialize_integer_types() {
        let u8_val: u8 = from_value(Value::U8(42)).unwrap();
        assert_eq!(u8_val, 42u8);

        let u16_val: u16 = from_value(Value::U16(42)).unwrap();
        assert_eq!(u16_val, 42u16);

        let u32_val: u32 = from_value(Value::U32(42)).unwrap();
        assert_eq!(u32_val, 42u32);

        let u64_val: u64 = from_value(Value::U64(42)).unwrap();
        assert_eq!(u64_val, 42u64);

        let i8_val: i8 = from_value(Value::I8(-42)).unwrap();
        assert_eq!(i8_val, -42i8);

        let i16_val: i16 = from_value(Value::I16(-42)).unwrap();
        assert_eq!(i16_val, -42i16);

        let i32_val: i32 = from_value(Value::I32(-42)).unwrap();
        assert_eq!(i32_val, -42i32);

        let i64_val: i64 = from_value(Value::I64(-42)).unwrap();
        assert_eq!(i64_val, -42i64);
    }

    #[test]
    fn serde_roundtrip_complex_struct() {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Complex {
            name: String,
            age: u32,
            active: bool,
            score: f64,
            tags: Vec<String>,
            data: Option<Vec<u8>>,
        }

        let c = Complex {
            name: "test".into(),
            age: 30,
            active: true,
            score: 9.5,
            tags: vec!["a".into(), "b".into()],
            data: Some(vec![1, 2, 3]),
        };

        let pv = to_value(&c).unwrap();
        let back: Complex = from_value(pv).unwrap();
        assert_eq!(c, back);
    }
}

// ===========================================================================
// replace.rs
// ===========================================================================

mod replace_tests {
    use super::*;

    #[test]
    fn clean_recursive() {
        let v = platform_value!({
            "a": null,
            "b": 42,
            "c": { "d": null, "e": 1 }
        });
        let cleaned = v.clean_recursive().unwrap();
        assert!(cleaned.get_optional_value_at_path("a").unwrap().is_none());
        assert!(cleaned.get_optional_value_at_path("c.d").unwrap().is_none());
        assert!(cleaned.get_optional_value_at_path("c.e").unwrap().is_some());
    }

    #[test]
    fn replace_integer_type_at_path() {
        let mut v = platform_value!({
            "data": { "x": 42 }
        });
        v.replace_integer_type_at_path("data.x", IntegerReplacementType::U16)
            .unwrap();
        let val = v.get_value_at_path("data.x").unwrap();
        assert_eq!(*val, Value::U16(42));
    }

    #[test]
    fn replace_integer_type_at_paths() {
        let mut v = platform_value!({
            "data": { "x": 42, "y": 10 }
        });
        v.replace_integer_type_at_paths(vec!["data.x", "data.y"], IntegerReplacementType::U32)
            .unwrap();
        assert_eq!(*v.get_value_at_path("data.x").unwrap(), Value::U32(42));
        assert_eq!(*v.get_value_at_path("data.y").unwrap(), Value::U32(10));
    }
}

// ===========================================================================
// system_bytes.rs
// ===========================================================================

mod system_bytes_tests {
    use super::*;

    #[test]
    fn into_binary_data() {
        let v = Value::Bytes(vec![1, 2, 3]);
        let bd = v.into_binary_data().unwrap();
        assert_eq!(bd, BinaryData::new(vec![1, 2, 3]));
    }

    #[test]
    fn into_binary_data_from_identifier() {
        let v = Value::Identifier([5; 32]);
        let bd = v.into_binary_data().unwrap();
        assert_eq!(bd.0.len(), 32);
    }

    #[test]
    fn into_binary_data_from_array() {
        let v = Value::Array(vec![Value::U8(1), Value::U8(2)]);
        let bd = v.into_binary_data().unwrap();
        assert_eq!(bd, BinaryData::new(vec![1, 2]));
    }

    #[test]
    fn into_binary_data_error() {
        let v = Value::Bool(true);
        assert!(v.into_binary_data().is_err());
    }

    #[test]
    fn to_identifier_from_bytes32() {
        let v = Value::Bytes32([7; 32]);
        let id = v.to_identifier().unwrap();
        assert_eq!(id.to_buffer(), [7; 32]);
    }

    #[test]
    fn to_identifier_from_identifier() {
        let v = Value::Identifier([7; 32]);
        let id = v.to_identifier().unwrap();
        assert_eq!(id.to_buffer(), [7; 32]);
    }

    #[test]
    fn to_identifier_from_text() {
        // Valid base58 text representing a 32-byte identifier
        let id = Identifier::new([5; 32]);
        let text = bs58::encode(id.to_buffer()).into_string();
        let v = Value::Text(text);
        let result = v.to_identifier().unwrap();
        assert_eq!(result.to_buffer(), [5; 32]);
    }
}

// ===========================================================================
// btreemap_extensions/mod.rs
// ===========================================================================

mod btreemap_extensions_tests {
    use super::*;
    use platform_value::btreemap_extensions::BTreeValueMapHelper;

    fn make_btree() -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("name".to_string(), Value::Text("test".into()));
        m.insert("count".to_string(), Value::U64(42));
        m.insert("active".to_string(), Value::Bool(true));
        m.insert("score".to_string(), Value::Float(9.5));
        m.insert(
            "data".to_string(),
            Value::Bytes(vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
        );
        m.insert("id".to_string(), Value::Identifier([5; 32]));
        m.insert(
            "tags".to_string(),
            Value::Array(vec![Value::Text("a".into()), Value::Text("b".into())]),
        );
        m.insert("null_val".to_string(), Value::Null);
        m
    }

    #[test]
    fn get_string() {
        let m = make_btree();
        assert_eq!(m.get_string("name").unwrap(), "test");
        assert!(m.get_string("missing").is_err());
    }

    #[test]
    fn get_optional_string() {
        let m = make_btree();
        assert_eq!(m.get_optional_string("name").unwrap(), Some("test".into()));
        assert_eq!(m.get_optional_string("missing").unwrap(), None);
    }

    #[test]
    fn get_str() {
        let m = make_btree();
        assert_eq!(m.get_str("name").unwrap(), "test");
        assert!(m.get_str("missing").is_err());
    }

    #[test]
    fn get_optional_str() {
        let m = make_btree();
        assert_eq!(m.get_optional_str("name").unwrap(), Some("test"));
        assert_eq!(m.get_optional_str("missing").unwrap(), None);
    }

    #[test]
    fn get_integer() {
        let m = make_btree();
        let v: u64 = m.get_integer("count").unwrap();
        assert_eq!(v, 42);
        assert!(m.get_integer::<u64>("missing").is_err());
    }

    #[test]
    fn get_optional_integer() {
        let m = make_btree();
        let v: Option<u64> = m.get_optional_integer("count").unwrap();
        assert_eq!(v, Some(42));
        let v: Option<u64> = m.get_optional_integer("missing").unwrap();
        assert_eq!(v, None);
        // Null value should return None
        let v: Option<u64> = m.get_optional_integer("null_val").unwrap();
        assert_eq!(v, None);
    }

    #[test]
    fn get_bool() {
        let m = make_btree();
        assert!(m.get_bool("active").unwrap());
        assert!(m.get_bool("missing").is_err());
    }

    #[test]
    fn get_optional_bool() {
        let m = make_btree();
        assert_eq!(m.get_optional_bool("active").unwrap(), Some(true));
        assert_eq!(m.get_optional_bool("missing").unwrap(), None);
        assert_eq!(m.get_optional_bool("null_val").unwrap(), None);
    }

    #[test]
    fn get_float() {
        let m = make_btree();
        assert_eq!(m.get_float("score").unwrap(), 9.5);
        assert!(m.get_float("missing").is_err());
    }

    #[test]
    fn get_optional_float() {
        let m = make_btree();
        assert_eq!(m.get_optional_float("score").unwrap(), Some(9.5));
        assert_eq!(m.get_optional_float("missing").unwrap(), None);
        assert_eq!(m.get_optional_float("null_val").unwrap(), None);
    }

    #[test]
    fn get_u64() {
        let m = make_btree();
        assert_eq!(m.get_u64("count").unwrap(), 42);
        assert!(m.get_u64("missing").is_err());
    }

    #[test]
    fn get_optional_u64() {
        let m = make_btree();
        assert_eq!(m.get_optional_u64("count").unwrap(), Some(42));
        assert_eq!(m.get_optional_u64("missing").unwrap(), None);
        assert_eq!(m.get_optional_u64("null_val").unwrap(), None);
    }

    #[test]
    fn get_bytes() {
        let m = make_btree();
        assert_eq!(m.get_bytes("data").unwrap().len(), 32);
        assert!(m.get_bytes("missing").is_err());
    }

    #[test]
    fn get_optional_bytes() {
        let m = make_btree();
        assert!(m.get_optional_bytes("data").unwrap().is_some());
        assert!(m.get_optional_bytes("missing").unwrap().is_none());
    }

    #[test]
    fn get_hash256_bytes() {
        let m = make_btree();
        let hash = m.get_hash256_bytes("data").unwrap();
        assert_eq!(hash.len(), 32);
        assert!(m.get_hash256_bytes("missing").is_err());
    }

    #[test]
    fn get_optional_hash256_bytes() {
        let m = make_btree();
        assert!(m.get_optional_hash256_bytes("data").unwrap().is_some());
        assert!(m.get_optional_hash256_bytes("missing").unwrap().is_none());
    }

    #[test]
    fn get_identifier() {
        let m = make_btree();
        let id = m.get_identifier("id").unwrap();
        assert_eq!(id.to_buffer(), [5; 32]);
        assert!(m.get_identifier("missing").is_err());
    }

    #[test]
    fn get_optional_identifier() {
        let m = make_btree();
        assert!(m.get_optional_identifier("id").unwrap().is_some());
        assert!(m.get_optional_identifier("missing").unwrap().is_none());
    }

    #[test]
    fn get_inner_string_array() {
        let m = make_btree();
        let tags: Vec<String> = m.get_inner_string_array("tags").unwrap();
        assert_eq!(tags, vec!["a".to_string(), "b".to_string()]);
        assert!(m.get_inner_string_array::<Vec<String>>("missing").is_err());
    }

    #[test]
    fn get_optional_inner_string_array() {
        let m = make_btree();
        let tags: Option<Vec<String>> = m.get_optional_inner_string_array("tags").unwrap();
        assert!(tags.is_some());
        let missing: Option<Vec<String>> = m.get_optional_inner_string_array("missing").unwrap();
        assert!(missing.is_none());
        let null_val: Option<Vec<String>> = m.get_optional_inner_string_array("null_val").unwrap();
        assert!(null_val.is_none());
    }

    #[test]
    fn get_inner_value_array() {
        let m = make_btree();
        let tags: Vec<&Value> = m.get_inner_value_array("tags").unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn get_binary_data() {
        let m = make_btree();
        let bd = m.get_binary_data("data").unwrap();
        assert_eq!(bd.0.len(), 32);
        assert!(m.get_binary_data("missing").is_err());
    }

    #[test]
    fn get_optional_binary_data() {
        let m = make_btree();
        assert!(m.get_optional_binary_data("data").unwrap().is_some());
        assert!(m.get_optional_binary_data("missing").unwrap().is_none());
    }
}

// ===========================================================================
// btreemap_extensions/equal_underlying_data.rs
// ===========================================================================

mod btree_equal_underlying_data {
    use super::*;
    use platform_value::btreemap_extensions::EqualUnderlyingData;

    #[test]
    fn btree_maps_equal_underlying() {
        let mut a = BTreeMap::new();
        a.insert("x".to_string(), Value::U64(42));
        let mut b = BTreeMap::new();
        b.insert("x".to_string(), Value::I128(42));
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn btree_maps_not_equal_different_keys() {
        let mut a = BTreeMap::new();
        a.insert("x".to_string(), Value::U64(42));
        let mut b = BTreeMap::new();
        b.insert("y".to_string(), Value::U64(42));
        assert!(!a.equal_underlying_data(&b));
    }

    #[test]
    fn btree_maps_not_equal_different_values() {
        let mut a = BTreeMap::new();
        a.insert("x".to_string(), Value::U64(42));
        let mut b = BTreeMap::new();
        b.insert("x".to_string(), Value::U64(43));
        assert!(!a.equal_underlying_data(&b));
    }
}

// ===========================================================================
// btreemap_extensions path and removal helpers
// ===========================================================================

mod btree_path_tests {
    use super::*;
    use platform_value::btreemap_extensions::{
        BTreeValueMapPathHelper, BTreeValueRemoveFromMapHelper,
    };

    #[test]
    fn get_at_path() {
        let mut m = BTreeMap::new();
        m.insert("a".to_string(), platform_value!({ "b": { "c": 42 } }));
        let val = m.get_at_path("a.b.c").unwrap();
        assert!(val == &42i32);
    }

    #[test]
    fn get_optional_at_path() {
        let mut m = BTreeMap::new();
        m.insert("a".to_string(), platform_value!({ "b": 1 }));
        assert!(m.get_optional_at_path("a.b").unwrap().is_some());
        assert!(m.get_optional_at_path("a.x").unwrap().is_none());
    }

    #[test]
    fn remove_integer() {
        let mut m = BTreeMap::new();
        m.insert("x".to_string(), Value::U64(42));
        let val: u64 = m.remove_integer("x").unwrap();
        assert_eq!(val, 42);
        assert!(!m.contains_key("x"));
    }

    #[test]
    fn remove_optional_integer() {
        let mut m = BTreeMap::new();
        m.insert("x".to_string(), Value::U64(42));
        let val: Option<u64> = m.remove_optional_integer("x").unwrap();
        assert_eq!(val, Some(42));
        let val: Option<u64> = m.remove_optional_integer("missing").unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn remove_string() {
        let mut m = BTreeMap::new();
        m.insert("x".to_string(), Value::Text("hello".into()));
        let val = m.remove_string("x").unwrap();
        assert_eq!(val, "hello");
    }

    #[test]
    fn remove_optional_string() {
        let mut m = BTreeMap::new();
        m.insert("x".to_string(), Value::Text("hi".into()));
        assert!(m.remove_optional_string("x").unwrap().is_some());
        assert!(m.remove_optional_string("missing").unwrap().is_none());
    }

    #[test]
    fn remove_hash256_bytes() {
        let mut m = BTreeMap::new();
        m.insert(
            "h".to_string(),
            Value::Bytes(vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
        );
        let hash = m.remove_hash256_bytes("h").unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn remove_bytes() {
        let mut m = BTreeMap::new();
        m.insert("b".to_string(), Value::Bytes(vec![1, 2, 3]));
        let b = m.remove_bytes("b").unwrap();
        assert_eq!(b, vec![1, 2, 3]);
    }
}

// ===========================================================================
// btreemap_extensions mut value helpers
// ===========================================================================

mod btree_mut_value_tests {
    use super::*;
    use platform_value::btreemap_extensions::BTreeMutValueMapHelper;

    #[test]
    fn get_inner_map_in_array_mut() {
        let mut m = BTreeMap::new();
        m.insert(
            "items".to_string(),
            Value::Array(vec![
                platform_value!({ "name": "a" }),
                platform_value!({ "name": "b" }),
            ]),
        );
        let maps: Vec<BTreeMap<String, &mut Value>> =
            m.get_inner_map_in_array_mut("items").unwrap();
        assert_eq!(maps.len(), 2);
    }

    #[test]
    fn get_optional_inner_map_in_array_mut_none() {
        let mut m: BTreeMap<String, Value> = BTreeMap::new();
        let result: Option<Vec<BTreeMap<String, &mut Value>>> =
            m.get_optional_inner_map_in_array_mut("missing").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_inner_map_in_array_mut_missing() {
        let mut m: BTreeMap<String, Value> = BTreeMap::new();
        let result: Result<Vec<BTreeMap<String, &mut Value>>, Error> =
            m.get_inner_map_in_array_mut("missing");
        assert!(result.is_err());
    }
}
