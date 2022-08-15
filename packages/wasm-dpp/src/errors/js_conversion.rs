use serde_json;
use thiserror::Error;
use wasm_bindgen::JsValue;

/// This is a rust-specific errors. In addition to all errors defined in `js-dpp`, the
/// error might be triggered when using JS bindings
#[derive(Error, Debug)]
pub enum RustConversionError {
    #[error(transparent)]
    SerdeConversionError(#[from] serde_json::Error),

    #[error("Conversion Error: {0}")]
    Error(String),
}

impl RustConversionError {
    pub fn to_js_value(self) -> JsValue {
        self.into()
    }
}

impl From<String> for RustConversionError {
    fn from(v: String) -> Self {
        RustConversionError::Error(v)
    }
}

impl Into<JsValue> for RustConversionError {
    fn into(self) -> JsValue {
        JsValue::from(self.to_string())
    }
}

#[macro_export]
macro_rules! with_js_error {
    ($o:expr) => {
        $o.map_err(|e| RustConversionError::from(e).to_js_value())
    };
}

#[macro_export]
macro_rules! bail_js {
    ($msg:literal) => ({
        return Err(RustConversionError::from(String::from($msg)).to_js_value())
    });

    ($err:expr $(,)?) => ({
        use $crate::private::kind::*;
        return Err(RustConversionError::from($err).to_js_value())
    });

    ($fmt:expr, $($arg:tt)*) => {
        return Err(RustConversionError::from(format!($fmt, $($arg)*)).to_js_value())
    };

}
