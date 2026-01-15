use crate::error::{WasmDppError, WasmDppResult};
use dpp::dashcore::Network;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// TypeScript type alias for flexible network input
#[wasm_bindgen(typescript_custom_section)]
const NETWORK_LIKE_TS: &'static str = r#"
/**
 * Flexible network type that accepts Network enum, string names, or numeric values.
 *
 * String values (case-insensitive): "mainnet", "testnet", "devnet", "regtest"
 * Numeric values: 0 (mainnet), 1 (testnet), 2 (devnet), 3 (regtest)
 */
export type NetworkLike = Network | "mainnet" | "testnet" | "devnet" | "regtest" | 0 | 1 | 2 | 3;
"#;

#[wasm_bindgen(js_name = "Network")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum NetworkWasm {
    Mainnet = 0,
    Testnet = 1,
    Devnet = 2,
    Regtest = 3,
}

impl TryFrom<JsValue> for NetworkWasm {
    type Error = WasmDppError;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        // Handle string input
        if let Some(enum_val) = value.as_string() {
            return match enum_val.to_lowercase().as_str() {
                "mainnet" => Ok(NetworkWasm::Mainnet),
                "testnet" => Ok(NetworkWasm::Testnet),
                "devnet" => Ok(NetworkWasm::Devnet),
                "regtest" => Ok(NetworkWasm::Regtest),
                _ => Err(WasmDppError::invalid_argument(format!(
                    "unsupported network name '{}'. Expected: mainnet, testnet, devnet, or regtest",
                    enum_val
                ))),
            };
        }

        // Handle numeric enum value (Network.Mainnet = 0, Testnet = 1, etc.)
        if let Some(num) = value.as_f64() {
            return match num as u32 {
                0 => Ok(NetworkWasm::Mainnet),
                1 => Ok(NetworkWasm::Testnet),
                2 => Ok(NetworkWasm::Devnet),
                3 => Ok(NetworkWasm::Regtest),
                _ => Err(WasmDppError::invalid_argument(format!(
                    "unsupported network value '{}'. Expected: 0 (mainnet), 1 (testnet), 2 (devnet), or 3 (regtest)",
                    num
                ))),
            };
        }

        Err(WasmDppError::invalid_argument(
            "network must be a string ('mainnet', 'testnet', 'devnet', 'regtest') or Network enum value",
        ))
    }
}

impl From<NetworkWasm> for String {
    fn from(value: NetworkWasm) -> Self {
        match value {
            NetworkWasm::Mainnet => "Mainnet".to_string(),
            NetworkWasm::Testnet => "Testnet".to_string(),
            NetworkWasm::Devnet => "Devnet".to_string(),
            NetworkWasm::Regtest => "Regtest".to_string(),
        }
    }
}

impl TryFrom<&JsValue> for NetworkWasm {
    type Error = WasmDppError;
    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        NetworkWasm::try_from(value.clone())
    }
}

impl From<NetworkWasm> for Network {
    fn from(network: NetworkWasm) -> Self {
        match network {
            NetworkWasm::Mainnet => Network::Dash,
            NetworkWasm::Testnet => Network::Testnet,
            NetworkWasm::Devnet => Network::Devnet,
            NetworkWasm::Regtest => Network::Regtest,
        }
    }
}

impl From<Network> for NetworkWasm {
    fn from(network: Network) -> Self {
        match network {
            Network::Dash => NetworkWasm::Mainnet,
            Network::Testnet => NetworkWasm::Testnet,
            Network::Devnet => NetworkWasm::Devnet,
            Network::Regtest => NetworkWasm::Regtest,
            _ => unreachable!("Unknown network variant"),
        }
    }
}

impl NetworkWasm {
    /// Try to extract a Network from an options object field.
    ///
    /// This helper reads the specified field from an options object and converts it
    /// to a NetworkWasm. Accepts Network enum or string names.
    pub fn try_from_options(options: &JsValue, field_name: &str) -> WasmDppResult<Self> {
        let network_js =
            js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
                WasmDppError::invalid_argument(format!("Missing '{}' field", field_name))
            })?;

        if network_js.is_undefined() || network_js.is_null() {
            return Err(WasmDppError::invalid_argument(format!(
                "'{}' is required",
                field_name
            )));
        }

        NetworkWasm::try_from(&network_js)
    }

    /// Try to extract an optional Network from an options object field.
    ///
    /// Returns None if the field is undefined or null, otherwise attempts conversion.
    pub fn try_from_options_optional(
        options: &JsValue,
        field_name: &str,
    ) -> WasmDppResult<Option<Self>> {
        let network_js =
            js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
                WasmDppError::invalid_argument(format!("Failed to get '{}'", field_name))
            })?;

        if network_js.is_undefined() || network_js.is_null() {
            return Ok(None);
        }

        NetworkWasm::try_from(&network_js).map(Some)
    }

    /// Get the network name as a lowercase string (for compatibility with existing code)
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkWasm::Mainnet => "mainnet",
            NetworkWasm::Testnet => "testnet",
            NetworkWasm::Devnet => "devnet",
            NetworkWasm::Regtest => "regtest",
        }
    }
}
