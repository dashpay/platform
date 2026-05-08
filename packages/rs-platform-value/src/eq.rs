use crate::Value;

macro_rules! implpartialeq {
    ($($t:ty),+ $(,)?) => {
        $(
            impl PartialEq<$t> for Value {
                #[inline]
                fn eq(&self, other: &$t) -> bool {
                    if let Some(i) = self.as_integer::<$t>() {
                        &i == other
                    } else {
                        false
                    }
                }
            }

            impl PartialEq<$t> for &Value {
                #[inline]
                fn eq(&self, other: &$t) -> bool {
                    if let Some(i) = self.as_integer::<$t>() {
                        &i == other
                    } else {
                        false
                    }
                }
            }
        )+
    };
}

implpartialeq! {
    u128,
    u64,
    u32,
    u16,
    u8,
    i128,
    i64,
    i32,
    i16,
    i8,
}

impl PartialEq<String> for Value {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        if let Some(i) = self.as_text() {
            i == other
        } else {
            false
        }
    }
}

impl PartialEq<String> for &Value {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        if let Some(i) = self.as_str() {
            i == other
        } else {
            false
        }
    }
}

impl PartialEq<&str> for Value {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        if let Some(i) = self.as_str() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<&str> for &Value {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        if let Some(i) = self.as_str() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<f64> for Value {
    #[inline]
    fn eq(&self, other: &f64) -> bool {
        if let Some(i) = self.as_float() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<f64> for &Value {
    #[inline]
    fn eq(&self, other: &f64) -> bool {
        if let Some(i) = self.as_float() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<Vec<u8>> for Value {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_bytes_slice() == Ok(other.as_slice())
    }
}
impl PartialEq<Vec<u8>> for &Value {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_bytes_slice() == Ok(other.as_slice())
    }
}

macro_rules! impl_bytes_array_eq {
    ($($n:expr),+ $(,)?) => {$(
        impl PartialEq<[u8; $n]> for Value {
            #[inline]
            fn eq(&self, other: &[u8; $n]) -> bool {
                self.as_bytes_slice() == Ok(other.as_slice())
            }
        }
        impl PartialEq<[u8; $n]> for &Value {
            #[inline]
            fn eq(&self, other: &[u8; $n]) -> bool {
                self.as_bytes_slice() == Ok(other.as_slice())
            }
        }
    )+};
}
impl_bytes_array_eq! { 20, 32, 36 }

impl Value {
    /* -------------------------------------------------------- *
     *  equality on underlying data                             *
     * -------------------------------------------------------- */

    /// Returns `true` when the *data* represented by the two `Value`s
    /// is identical, even if they are stored in different but
    /// compatible variants.
    ///
    /// * All “bytes-like” variants (`Bytes`, `Bytes20`, `Bytes32`,
    ///   `Bytes36`, `Identifier`) compare equal when their byte
    ///   sequences match.
    /// * All integer variants (`U*`, `I*`) compare equal when they
    ///   represent the same numeric value.
    /// * Otherwise falls back to normal `==` (`PartialEq`) behaviour.
    #[inline]
    pub fn equal_underlying_data(&self, other: &Value) -> bool {
        // 1) bytes-like cross-variant equality
        if let (Ok(a), Ok(b)) = (self.as_bytes_slice(), other.as_bytes_slice()) {
            return a == b;
        }

        // 2) integer cross-variant equality
        if let (Some(a), Some(b)) = (self.as_i128_unified(), other.as_i128_unified()) {
            return a == b;
        }

        // 3) default
        self == other
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use crate::Value;

    // ---- PartialEq<integer types> ----

    #[test]
    fn u8_eq() {
        assert_eq!(Value::U8(42), 42u8);
        assert_ne!(Value::U8(42), 43u8);
    }

    #[test]
    fn i8_eq() {
        assert_eq!(Value::I8(-1), -1i8);
        assert_ne!(Value::I8(-1), 0i8);
    }

    #[test]
    fn u16_eq() {
        assert_eq!(Value::U16(1000), 1000u16);
        assert_ne!(Value::U16(1000), 999u16);
    }

    #[test]
    fn i16_eq() {
        assert_eq!(Value::I16(-500), -500i16);
        assert_ne!(Value::I16(-500), 500i16);
    }

    #[test]
    fn u32_eq() {
        assert_eq!(Value::U32(100_000), 100_000u32);
        assert_ne!(Value::U32(100_000), 0u32);
    }

    #[test]
    fn i32_eq() {
        assert_eq!(Value::I32(-100), -100i32);
        assert_ne!(Value::I32(-100), 100i32);
    }

    #[test]
    fn u64_eq() {
        assert_eq!(Value::U64(u64::MAX), u64::MAX);
        assert_ne!(Value::U64(0), 1u64);
    }

    #[test]
    fn i64_eq() {
        assert_eq!(Value::I64(i64::MIN), i64::MIN);
        assert_ne!(Value::I64(0), 1i64);
    }

    #[test]
    fn u128_eq() {
        assert_eq!(Value::U128(u128::MAX), u128::MAX);
        assert_ne!(Value::U128(0), 1u128);
    }

    #[test]
    fn i128_eq() {
        assert_eq!(Value::I128(i128::MIN), i128::MIN);
        assert_ne!(Value::I128(0), 1i128);
    }

    // ---- cross-type integer comparison via as_integer ----

    #[test]
    fn u8_value_eq_u64_type() {
        // Value::U8(10) should equal 10u64 through as_integer
        assert_eq!(Value::U8(10), 10u64);
    }

    #[test]
    fn u64_value_eq_u8_type_when_fits() {
        assert_eq!(Value::U64(200), 200u8);
    }

    #[test]
    fn u64_value_ne_u8_type_when_overflow() {
        // 256 doesn't fit in u8
        assert_ne!(Value::U64(256), 0u8); // as_integer::<u8> returns None
    }

    #[test]
    fn i8_value_eq_i64_type() {
        assert_eq!(Value::I8(-10), -10i64);
    }

    #[test]
    fn non_integer_ne_integer() {
        assert_ne!(Value::Text("hello".to_string()), 0u64);
        assert_ne!(Value::Null, 0i32);
        assert_ne!(Value::Bool(true), 1u8);
    }

    // ---- PartialEq<String> ----

    #[test]
    fn string_eq() {
        let val = Value::Text("hello".to_string());
        assert_eq!(val, "hello".to_string());
        assert_ne!(val, "world".to_string());
    }

    #[test]
    fn non_text_ne_string() {
        assert_ne!(Value::U8(0), "0".to_string());
        assert_ne!(Value::Null, "".to_string());
    }

    // ---- PartialEq<&str> ----

    #[test]
    fn str_ref_eq() {
        let val = Value::Text("test".to_string());
        assert_eq!(val, "test");
        assert_ne!(val, "other");
    }

    #[test]
    fn non_text_ne_str_ref() {
        assert_ne!(Value::Bool(false), "false");
    }

    // ---- PartialEq<f64> ----

    #[test]
    fn float_eq() {
        assert_eq!(Value::Float(3.14), 3.14f64);
        assert_ne!(Value::Float(3.14), 3.15f64);
    }

    #[test]
    fn integer_eq_float_through_as_float() {
        // as_float converts integers to f64, so Value::U64(10) == 10.0f64
        assert_eq!(Value::U64(10), 10.0f64);
    }

    #[test]
    fn non_numeric_ne_float() {
        assert_ne!(Value::Text("3.14".to_string()), 3.14f64);
    }

    // ---- PartialEq<Vec<u8>> ----

    #[test]
    fn bytes_eq_vec_u8() {
        let data = vec![1, 2, 3];
        assert_eq!(Value::Bytes(data.clone()), data);
    }

    #[test]
    fn bytes_ne_vec_u8() {
        assert_ne!(Value::Bytes(vec![1, 2, 3]), vec![1, 2, 4]);
    }

    #[test]
    fn identifier_eq_vec_u8() {
        let id = [42u8; 32];
        assert_eq!(Value::Identifier(id), id.to_vec());
    }

    #[test]
    fn bytes20_eq_vec_u8() {
        let b = [5u8; 20];
        assert_eq!(Value::Bytes20(b), b.to_vec());
    }

    #[test]
    fn non_bytes_ne_vec_u8() {
        assert_ne!(Value::U8(1), vec![1u8]);
    }

    // ---- PartialEq<[u8; 32]> ----

    #[test]
    fn bytes32_eq_array() {
        let b = [0xffu8; 32];
        assert_eq!(Value::Bytes32(b), b);
    }

    #[test]
    fn identifier_eq_array_32() {
        let id = [7u8; 32];
        assert_eq!(Value::Identifier(id), id);
    }

    #[test]
    fn bytes_eq_array_32() {
        let data = [3u8; 32];
        assert_eq!(Value::Bytes(data.to_vec()), data);
    }

    #[test]
    fn non_bytes_ne_array_32() {
        assert_ne!(Value::Null, [0u8; 32]);
    }

    // ---- PartialEq<[u8; 20]> ----

    #[test]
    fn bytes20_eq_array_20() {
        let b = [1u8; 20];
        assert_eq!(Value::Bytes20(b), b);
    }

    // ---- PartialEq<[u8; 36]> ----

    #[test]
    fn bytes36_eq_array_36() {
        let b = [2u8; 36];
        assert_eq!(Value::Bytes36(b), b);
    }

    // ---- PartialEq for &Value ----

    #[test]
    fn ref_value_eq_integer() {
        let val = Value::U64(42);
        assert_eq!(&val, 42u64);
    }

    #[test]
    fn ref_value_eq_string() {
        let val = Value::Text("hi".to_string());
        assert_eq!(&val, "hi".to_string());
    }

    #[test]
    fn ref_value_eq_str_ref() {
        let val = Value::Text("hi".to_string());
        assert_eq!(&val, "hi");
    }

    #[test]
    fn ref_value_eq_float() {
        let val = Value::Float(1.0);
        assert_eq!(&val, 1.0f64);
    }

    #[test]
    fn ref_value_eq_vec_u8() {
        let val = Value::Bytes(vec![10, 20]);
        assert_eq!(&val, vec![10u8, 20]);
    }

    #[test]
    fn ref_value_eq_array_32() {
        let b = [0u8; 32];
        let val = Value::Bytes32(b);
        assert_eq!(&val, b);
    }

    // ---- equal_underlying_data tests ----

    #[test]
    fn equal_underlying_data_bytes_vs_identifier_same_data() {
        let data = [42u8; 32];
        let bytes = Value::Bytes(data.to_vec());
        let ident = Value::Identifier(data);
        assert!(bytes.equal_underlying_data(&ident));
        assert!(ident.equal_underlying_data(&bytes));
    }

    #[test]
    fn equal_underlying_data_bytes_vs_identifier_different_data() {
        let bytes = Value::Bytes(vec![0u8; 32]);
        let ident = Value::Identifier([1u8; 32]);
        assert!(!bytes.equal_underlying_data(&ident));
    }

    #[test]
    fn equal_underlying_data_bytes32_vs_identifier() {
        let data = [99u8; 32];
        let b32 = Value::Bytes32(data);
        let ident = Value::Identifier(data);
        assert!(b32.equal_underlying_data(&ident));
    }

    #[test]
    fn equal_underlying_data_bytes20_vs_bytes() {
        let data = [5u8; 20];
        let b20 = Value::Bytes20(data);
        let bytes = Value::Bytes(data.to_vec());
        assert!(b20.equal_underlying_data(&bytes));
    }

    #[test]
    fn equal_underlying_data_u8_vs_u64_same_value() {
        let a = Value::U8(10);
        let b = Value::U64(10);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_i8_vs_i128_same_value() {
        let a = Value::I8(-5);
        let b = Value::I128(-5);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_u8_vs_u64_different_value() {
        let a = Value::U8(10);
        let b = Value::U64(20);
        assert!(!a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_u16_vs_i32_same_value() {
        let a = Value::U16(100);
        let b = Value::I32(100);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_negative_i8_vs_u64() {
        // negative can't match unsigned
        let a = Value::I8(-1);
        let b = Value::U64(255);
        assert!(!a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_same_variant_same_value() {
        let a = Value::U64(42);
        let b = Value::U64(42);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_fallback_to_partial_eq() {
        // Text vs Text uses default PartialEq
        let a = Value::Text("hello".to_string());
        let b = Value::Text("hello".to_string());
        assert!(a.equal_underlying_data(&b));

        let c = Value::Text("world".to_string());
        assert!(!a.equal_underlying_data(&c));
    }

    #[test]
    fn equal_underlying_data_null_vs_null() {
        assert!(Value::Null.equal_underlying_data(&Value::Null));
    }

    #[test]
    fn equal_underlying_data_different_types_not_equal() {
        // A string vs a number should not be equal
        let a = Value::Text("42".to_string());
        let b = Value::U64(42);
        assert!(!a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_bool_vs_bool() {
        assert!(Value::Bool(true).equal_underlying_data(&Value::Bool(true)));
        assert!(!Value::Bool(true).equal_underlying_data(&Value::Bool(false)));
    }

    #[test]
    fn equal_underlying_data_float_vs_float() {
        assert!(Value::Float(1.5).equal_underlying_data(&Value::Float(1.5)));
        assert!(!Value::Float(1.5).equal_underlying_data(&Value::Float(2.5)));
    }
}
