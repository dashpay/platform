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
