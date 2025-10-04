use dpp::dashcore::network::constants::Network;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
#[wasm_bindgen(js_name = "Network")]
#[allow(non_camel_case_types)]
pub enum NetworkWasm {
    Mainnet = 0,
    Testnet = 1,
    Devnet = 2,
    Regtest = 3,
}

impl TryFrom<JsValue> for NetworkWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "mainnet" => Ok(NetworkWasm::Mainnet),
                    "testnet" => Ok(NetworkWasm::Testnet),
                    "devnet" => Ok(NetworkWasm::Devnet),
                    "regtest" => Ok(NetworkWasm::Regtest),
                    _ => Err(JsValue::from(format!(
                        "unsupported network name ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(NetworkWasm::Mainnet),
                    1 => Ok(NetworkWasm::Testnet),
                    2 => Ok(NetworkWasm::Devnet),
                    3 => Ok(NetworkWasm::Regtest),
                    _ => Err(JsValue::from(format!(
                        "unsupported network name ({})",
                        enum_val
                    ))),
                },
            },
        }
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
