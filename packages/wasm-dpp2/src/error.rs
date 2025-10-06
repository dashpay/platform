use anyhow::Error as AnyhowError;
use dpp::ProtocolError;
use js_sys::Error as JsError;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

/// Structured error returned by wasm-dpp2 APIs.
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WasmDppErrorKind {
    /// Error raised by Dash Platform Protocol.
    Protocol,
    /// Invalid argument provided by the caller.
    InvalidArgument,
    /// Serialization or deserialization failure.
    Serialization,
    /// Type conversion failure.
    Conversion,
    /// Catch-all for other failure modes.
    Generic,
}

/// Structured error returned by wasm-dpp2 APIs.
#[wasm_bindgen]
#[derive(thiserror::Error, Debug, Clone)]
#[error("{message}")]
pub struct WasmDppError {
    kind: WasmDppErrorKind,
    message: String,
    /// Optional numeric error code. `-1` indicates absence.
    code: i32,
}

impl WasmDppError {
    pub(crate) fn new(
        kind: WasmDppErrorKind,
        message: impl Into<String>,
        code: Option<i32>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            code: code.unwrap_or(-1),
        }
    }

    pub(crate) fn protocol(message: impl Into<String>) -> Self {
        Self::new(WasmDppErrorKind::Protocol, message, None)
    }

    #[allow(dead_code)]
    pub(crate) fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(WasmDppErrorKind::InvalidArgument, message, None)
    }

    pub(crate) fn serialization(message: impl Into<String>) -> Self {
        Self::new(WasmDppErrorKind::Serialization, message, None)
    }

    #[allow(dead_code)]
    pub(crate) fn conversion(message: impl Into<String>) -> Self {
        Self::new(WasmDppErrorKind::Conversion, message, None)
    }

    pub(crate) fn generic(message: impl Into<String>) -> Self {
        Self::new(WasmDppErrorKind::Generic, message, None)
    }
}

impl From<ProtocolError> for WasmDppError {
    fn from(error: ProtocolError) -> Self {
        Self::protocol(error.to_string())
    }
}

impl From<AnyhowError> for WasmDppError {
    fn from(error: AnyhowError) -> Self {
        Self::generic(error.to_string())
    }
}

impl From<JsValue> for WasmDppError {
    fn from(value: JsValue) -> Self {
        if value.is_null() || value.is_undefined() {
            return WasmDppError::invalid_argument("JavaScript error: value is null or undefined");
        }

        if let Some(js_error) = value.dyn_ref::<JsError>() {
            return WasmDppError::invalid_argument(js_error.message());
        }

        if let Some(message) = value.as_string() {
            return WasmDppError::invalid_argument(message);
        }

        let message = js_sys::JSON::stringify(&value)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Unknown JavaScript error".to_string());

        WasmDppError::invalid_argument(message)
    }
}

#[wasm_bindgen]
impl WasmDppError {
    /// Returns the structured error kind.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> WasmDppErrorKind {
        self.kind
    }

    /// Backwards-compatible string representation of the kind.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        match self.kind {
            WasmDppErrorKind::Protocol => "Protocol",
            WasmDppErrorKind::InvalidArgument => "InvalidArgument",
            WasmDppErrorKind::Serialization => "Serialization",
            WasmDppErrorKind::Conversion => "Conversion",
            WasmDppErrorKind::Generic => "Generic",
        }
        .to_string()
    }

    /// Human-readable error message.
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// Optional numeric code. `-1` means absent.
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> i32 {
        self.code
    }
}

/// Result alias that uses `WasmDppError` as the error type.
pub type WasmDppResult<T> = Result<T, WasmDppError>;
