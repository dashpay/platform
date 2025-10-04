use crate::enums::network::NetworkWASM;
use dpp::dashcore::address::Payload;
use dpp::dashcore::{Address, opcodes};
use dpp::identity::core_script::CoreScript;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::encode;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "CoreScriptWASM")]
#[derive(Clone)]
pub struct CoreScriptWASM(CoreScript);

impl From<CoreScriptWASM> for CoreScript {
    fn from(value: CoreScriptWASM) -> Self {
        value.0
    }
}

impl From<CoreScript> for CoreScriptWASM {
    fn from(value: CoreScript) -> Self {
        CoreScriptWASM(value)
    }
}

#[wasm_bindgen]
impl CoreScriptWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "CoreScriptWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "CoreScriptWASM".to_string()
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        CoreScriptWASM(CoreScript::from_bytes(bytes))
    }

    #[wasm_bindgen(js_name = "newP2PKH")]
    pub fn new_p2pkh(js_key_hash: Vec<u8>) -> Self {
        let mut key_hash = [0u8; 20];
        let bytes = js_key_hash.as_slice();
        let len = bytes.len().min(32);
        key_hash[..len].copy_from_slice(&bytes[..len]);

        CoreScriptWASM(CoreScript::new_p2pkh(key_hash))
    }

    #[wasm_bindgen(js_name = "newP2SH")]
    pub fn new_p2sh(js_script_hash: Vec<u8>) -> Self {
        let mut script_hash = [0u8; 20];
        let bytes = js_script_hash.as_slice();
        let len = bytes.len().min(32);
        script_hash[..len].copy_from_slice(&bytes[..len]);

        let mut bytes = vec![
            opcodes::all::OP_HASH160.to_u8(),
            opcodes::all::OP_PUSHBYTES_20.to_u8(),
        ];
        bytes.extend_from_slice(&script_hash);
        bytes.push(opcodes::all::OP_EQUAL.to_u8());
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toAddress")]
    pub fn to_address(&self, js_network: &JsValue) -> Result<String, JsValue> {
        let network = NetworkWASM::try_from(js_network.clone())?;

        let payload =
            Payload::from_script(self.0.as_script()).map_err(|e| JsValue::from(e.to_string()))?;

        let address = Address::new(network.into(), payload);

        Ok(address.to_string())
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> String {
        self.0.to_string(Base64)
    }

    #[wasm_bindgen(js_name = "bytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    #[wasm_bindgen(js_name = "hex")]
    pub fn to_hex(&self) -> String {
        encode(self.0.to_bytes().as_slice(), Hex)
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> String {
        encode(self.0.to_bytes().as_slice(), Base64)
    }

    #[wasm_bindgen(js_name = "ASMString")]
    pub fn to_asm_string(&self) -> String {
        self.0.to_asm_string()
    }
}
