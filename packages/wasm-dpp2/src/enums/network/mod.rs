use dpp::dashcore::network::constants::Network;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
#[wasm_bindgen(js_name = "NetworkWASM")]
#[allow(non_camel_case_types)]
pub enum NetworkWASM {
    Mainnet = 0,
    Testnet = 1,
    Devnet = 2,
    Regtest = 3,
}

impl TryFrom<JsValue> for NetworkWASM {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "mainnet" => Ok(NetworkWASM::Mainnet),
                    "testnet" => Ok(NetworkWASM::Testnet),
                    "devnet" => Ok(NetworkWASM::Devnet),
                    "regtest" => Ok(NetworkWASM::Regtest),
                    _ => Err(JsValue::from(format!(
                        "unsupported network name ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(NetworkWASM::Mainnet),
                    1 => Ok(NetworkWASM::Testnet),
                    2 => Ok(NetworkWASM::Devnet),
                    3 => Ok(NetworkWASM::Regtest),
                    _ => Err(JsValue::from(format!(
                        "unsupported network name ({})",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<NetworkWASM> for String {
    fn from(value: NetworkWASM) -> Self {
        match value {
            NetworkWASM::Mainnet => "Mainnet".to_string(),
            NetworkWASM::Testnet => "Testnet".to_string(),
            NetworkWASM::Devnet => "Devnet".to_string(),
            NetworkWASM::Regtest => "Regtest".to_string(),
        }
    }
}

impl From<NetworkWASM> for Network {
    fn from(network: NetworkWASM) -> Self {
        match network {
            NetworkWASM::Mainnet => Network::Dash,
            NetworkWASM::Testnet => Network::Testnet,
            NetworkWASM::Devnet => Network::Devnet,
            NetworkWASM::Regtest => Network::Regtest,
        }
    }
}
