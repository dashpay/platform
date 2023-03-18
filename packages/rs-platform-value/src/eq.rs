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
