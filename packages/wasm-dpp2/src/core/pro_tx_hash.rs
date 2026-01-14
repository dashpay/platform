use crate::error::{WasmDppError, WasmDppResult};
use dpp::dashcore::ProTxHash;
use dpp::dashcore::hashes::{Hash, sha256d};
use std::str::FromStr;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// TypeScript type alias for flexible ProTxHash input
#[wasm_bindgen(typescript_custom_section)]
const PRO_TX_HASH_LIKE_TS: &'static str = r#"
/**
 * Flexible ProTxHash type that accepts ProTxHash object, hex string, or Uint8Array.
 *
 * - Hex string: 64-character hex-encoded hash (reversed byte order, as displayed)
 * - Uint8Array: 32 bytes in internal byte order
 */
export type ProTxHashLike = ProTxHash | string | Uint8Array;
"#;

#[wasm_bindgen(js_name = "ProTxHash")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProTxHashWasm(ProTxHash);

impl ProTxHashWasm {
    /// Create from inner ProTxHash
    pub fn new(inner: ProTxHash) -> Self {
        Self(inner)
    }

    /// Get the inner ProTxHash
    pub fn inner(&self) -> &ProTxHash {
        &self.0
    }

    /// Consume and return the inner ProTxHash
    pub fn into_inner(self) -> ProTxHash {
        self.0
    }

    /// Get as hex string (reversed byte order, as typically displayed)
    pub fn to_hex(&self) -> String {
        self.0.to_string()
    }

    /// Get as raw bytes (internal byte order)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_byte_array()
    }

    /// Create a ProTxHash from a hex string
    pub fn from_hex(hex: &str) -> WasmDppResult<ProTxHashWasm> {
        let hash = ProTxHash::from_str(hex)
            .map_err(|e| WasmDppError::invalid_argument(format!("Invalid ProTxHash hex: {}", e)))?;
        Ok(ProTxHashWasm(hash))
    }

    /// Try to extract a ProTxHash from an options object field.
    ///
    /// This helper reads the specified field from an options object and converts it
    /// to a ProTxHashWasm. Accepts hex string, Uint8Array, or ProTxHash object.
    pub fn try_from_options(options: &JsValue, field_name: &str) -> WasmDppResult<Self> {
        let hash_js =
            js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
                WasmDppError::invalid_argument(format!("Missing '{}' field", field_name))
            })?;

        if hash_js.is_undefined() || hash_js.is_null() {
            return Err(WasmDppError::invalid_argument(format!(
                "'{}' is required",
                field_name
            )));
        }

        ProTxHashWasm::try_from(&hash_js)
    }

    /// Try to extract an optional ProTxHash from an options object field.
    ///
    /// Returns None if the field is undefined, null, or an empty string/array.
    /// Otherwise attempts conversion.
    pub fn try_from_options_optional(
        options: &JsValue,
        field_name: &str,
    ) -> WasmDppResult<Option<Self>> {
        let hash_js =
            js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
                WasmDppError::invalid_argument(format!("Failed to get '{}'", field_name))
            })?;

        if hash_js.is_undefined() || hash_js.is_null() {
            return Ok(None);
        }

        // Check for empty string
        if let Some(s) = hash_js.as_string()
            && s.is_empty()
        {
            return Ok(None);
        }

        ProTxHashWasm::try_from(&hash_js).map(Some)
    }
}

impl TryFrom<JsValue> for ProTxHashWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        // Try as string first (hex format)
        if let Some(hex_str) = value.as_string() {
            let hash = ProTxHash::from_str(&hex_str).map_err(|e| {
                WasmDppError::invalid_argument(format!("Invalid ProTxHash hex string: {}", e))
            })?;
            return Ok(ProTxHashWasm(hash));
        }

        // Try as Uint8Array
        if value.is_object() {
            let bytes = js_sys::Uint8Array::new(&value).to_vec();
            if bytes.len() != 32 {
                return Err(WasmDppError::invalid_argument(format!(
                    "ProTxHash must be exactly 32 bytes, got {} bytes",
                    bytes.len()
                )));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            let raw = sha256d::Hash::from_byte_array(arr);
            let hash = ProTxHash::from_raw_hash(raw);
            return Ok(ProTxHashWasm(hash));
        }

        Err(WasmDppError::invalid_argument(
            "ProTxHash must be a hex string or Uint8Array (32 bytes)",
        ))
    }
}

impl TryFrom<&JsValue> for ProTxHashWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        ProTxHashWasm::try_from(value.clone())
    }
}

impl From<ProTxHashWasm> for ProTxHash {
    fn from(wrapper: ProTxHashWasm) -> Self {
        wrapper.0
    }
}

impl From<ProTxHash> for ProTxHashWasm {
    fn from(hash: ProTxHash) -> Self {
        ProTxHashWasm(hash)
    }
}
