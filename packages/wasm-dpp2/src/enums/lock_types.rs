//! Internal Rust-side discriminator for `AssetLockProof` variants.
//!
//! Not exposed to JS as a wasm-bindgen enum (numeric enums are unidiomatic at
//! the JS / TS boundary; the wire shape uses lowercase strings "instant" /
//! "chain", matching `AssetLockProof::toObject()` / `toJSON()`). JS-facing
//! getters return the lowercase string directly via the `Display` impl below.

use crate::error::WasmDppError;
use wasm_bindgen::JsValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetLockProofTypeWasm {
    Instant,
    Chain,
}

impl AssetLockProofTypeWasm {
    /// Lowercase wire-shape name ("instant" / "chain"), matching the
    /// adjacent-tagged `{ type, data }` shape emitted by `AssetLockProof.toObject()`.
    pub fn as_wire_name(self) -> &'static str {
        match self {
            AssetLockProofTypeWasm::Instant => "instant",
            AssetLockProofTypeWasm::Chain => "chain",
        }
    }
}

impl TryFrom<&JsValue> for AssetLockProofTypeWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        if let Some(s) = value.as_string() {
            return match s.to_lowercase().as_str() {
                "instant" => Ok(AssetLockProofTypeWasm::Instant),
                "chain" => Ok(AssetLockProofTypeWasm::Chain),
                other => Err(WasmDppError::invalid_argument(format!(
                    "unsupported lock type '{}', expected \"instant\" or \"chain\"",
                    other
                ))),
            };
        }
        Err(WasmDppError::invalid_argument(
            "AssetLockProof type must be a string (\"instant\" or \"chain\")",
        ))
    }
}

impl TryFrom<JsValue> for AssetLockProofTypeWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl From<AssetLockProofTypeWasm> for String {
    fn from(value: AssetLockProofTypeWasm) -> Self {
        value.as_wire_name().to_string()
    }
}
