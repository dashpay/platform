use crate::error::WasmDppError;
use crate::impl_try_from_options;
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

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "NetworkLike")]
    pub type NetworkLikeJs;
}

impl TryFrom<NetworkLikeJs> for NetworkWasm {
    type Error = WasmDppError;
    fn try_from(value: NetworkLikeJs) -> Result<Self, Self::Error> {
        let js_value: JsValue = value.into();
        NetworkWasm::try_from(js_value)
    }
}

impl TryFrom<NetworkLikeJs> for Network {
    type Error = WasmDppError;
    fn try_from(value: NetworkLikeJs) -> Result<Self, Self::Error> {
        let wasm: NetworkWasm = value.try_into()?;
        Ok(Network::from(wasm))
    }
}

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
            // Validate that the number is a non-negative integer within u32 range
            if num.fract() != 0.0 || num < 0.0 || num > u32::MAX as f64 {
                return Err(WasmDppError::invalid_argument(format!(
                    "network value must be a non-negative integer, got '{}'",
                    num
                )));
            }

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

impl_try_from_options!(NetworkWasm);

impl NetworkWasm {
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
