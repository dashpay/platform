use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, try_to_array, try_to_fixed_bytes};
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
export type ProTxHashLikeArray = Array<ProTxHash | string | Uint8Array>;
"#;

/// Extern type for flexible ProTxHash input
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ProTxHashLike")]
    pub type ProTxHashLikeJs;

    #[wasm_bindgen(typescript_type = "ProTxHashLike | null")]
    pub type ProTxHashLikeNullableJs;

    #[wasm_bindgen(typescript_type = "ProTxHashLikeArray")]
    pub type ProTxHashLikeArrayJs;
}

impl TryFrom<ProTxHashLikeJs> for ProTxHashWasm {
    type Error = WasmDppError;

    fn try_from(value: ProTxHashLikeJs) -> Result<Self, Self::Error> {
        let js_value: JsValue = value.into();
        ProTxHashWasm::try_from(js_value)
    }
}

impl TryFrom<ProTxHashLikeJs> for ProTxHash {
    type Error = WasmDppError;

    fn try_from(value: ProTxHashLikeJs) -> Result<Self, Self::Error> {
        let wasm: ProTxHashWasm = value.try_into()?;
        Ok(ProTxHash::from(wasm))
    }
}

impl TryFrom<ProTxHashLikeNullableJs> for Option<ProTxHash> {
    type Error = WasmDppError;

    fn try_from(value: ProTxHashLikeNullableJs) -> Result<Self, Self::Error> {
        let js_value: JsValue = value.into();
        if js_value.is_null() || js_value.is_undefined() {
            return Ok(None);
        }
        // Check for empty string
        if let Some(s) = js_value.as_string()
            && s.is_empty()
        {
            return Ok(None);
        }
        ProTxHashWasm::try_from(js_value).map(|w| Some(ProTxHash::from(w)))
    }
}

/// Helper function to convert a JavaScript array of ProTxHashLike values to Vec<ProTxHash>
pub fn pro_tx_hashes_from_js_array(
    array: ProTxHashLikeArrayJs,
) -> Result<Vec<ProTxHash>, WasmDppError> {
    let js_value: JsValue = array.into();
    let js_array = try_to_array(js_value, "proTxHashes")?;
    js_array
        .iter()
        .map(|v| {
            ProTxHashWasm::try_from(v)
                .map(ProTxHash::from)
                .map_err(|err| {
                    WasmDppError::invalid_argument(format!("Invalid ProTxHash: {}", err))
                })
        })
        .collect()
}

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
}

impl_try_from_options!(ProTxHashWasm);

impl TryFrom<JsValue> for ProTxHashWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        // Try as ProTxHash object first
        if let Ok(wasm) = value.to_wasm::<ProTxHashWasm>("ProTxHash") {
            return Ok(wasm.clone());
        }

        // Try as string (hex format)
        if let Some(hex_str) = value.as_string() {
            let hash = ProTxHash::from_str(&hex_str).map_err(|e| {
                WasmDppError::invalid_argument(format!("Invalid ProTxHash hex string: {}", e))
            })?;
            return Ok(ProTxHashWasm(hash));
        }

        // Try as Uint8Array (validates type and 32-byte length)
        if let Ok(arr) = try_to_fixed_bytes::<32>(value, "proTxHash") {
            let raw = sha256d::Hash::from_byte_array(arr);
            let hash = ProTxHash::from_raw_hash(raw);
            return Ok(ProTxHashWasm(hash));
        }

        Err(WasmDppError::invalid_argument(
            "ProTxHash must be a ProTxHash object, hex string, or Uint8Array (32 bytes)",
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

impl_wasm_type_info!(ProTxHashWasm, ProTxHash);
