use anyhow::anyhow;
use serde_json;
use serde_wasm_bindgen::Error;
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue};

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
        return Err($crate::errors::RustConversionError::from(String::from($msg)).to_js_value())
    });

    ($err:expr $(,)?) => ({
        use $crate::private::kind::*;
        return Err($crate::errors::RustConversionError::from($err).to_js_value())
    });

    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::errors::RustConversionError::from(format!($fmt, $($arg)*)).to_js_value())
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

pub fn from_js_error(e: JsValue) -> anyhow::Error {
    let message = if e.is_instance_of::<js_sys::Error>() {
        js_sys::Reflect::get(&e, &"message".into())
            .map(|e| {
                e.as_string()
                    .unwrap_or_else(|| String::from("Unknown JS Error: empty error message."))
            })
            .unwrap_or_else(|_| String::from("Unknown JS Error: unable to access error message"))
    } else {
        // TODO: is there a simpler way to to call `toString()`?
        let to_string_value = js_sys::Reflect::get(&e, &JsValue::from_str("toString"))
            .unwrap_or_else(|_| JsValue::undefined());

        let message = if let Some(to_string_function) =
            to_string_value.dyn_ref::<js_sys::Function>()
        {
            to_string_function
                .call0(&e)
                .unwrap_or_else(|_| JsValue::from_str("Unknown JS Error: call toString() failed"))
                .as_string()
                .unwrap()
        } else {
            String::from("Unknown Error: toString() is not a function")
        };

        message
    };

    anyhow!(message)
}
