use anyhow::Error as AnyhowError;
use dpp::ProtocolError;
use wasm_bindgen::prelude::wasm_bindgen;

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
