use dpp::dashcore::anyhow::bail;
use serde_json;
use serde_wasm_bindgen::Error;
use thiserror::Error;
use wasm_bindgen::JsValue;
use web_sys::console::log_1;

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

impl From<RustConversionError> for JsValue {
    fn from(err: RustConversionError) -> Self {
        Self::from(err.to_string())
    }
}

impl From<serde_wasm_bindgen::Error> for RustConversionError {
    fn from(err: Error) -> Self {
        Self::from(err.to_string())
    }
}

#[macro_export]
macro_rules! with_js_error {
    ($o:expr) => {
        $o.map_err(|e| $crate::errors::RustConversionError::from(e).to_js_value())
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

#[macro_export]
macro_rules! console_log {
    ($msg:literal) => {{
        web_sys::console::log_1(&format!($msg).into())
    }};

    ($msg:expr $(,)?) => {{
        web_sys::console::log_1(&format!("{}", $msg).into())
    }};

    ($fmt:expr, $($arg:tt)*) => {
        web_sys::console::log_1(&format!($fmt, $($arg)*).into())
    };
}
