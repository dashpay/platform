use super::network::{NetworkLikeJs, NetworkWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use dpp::dashcore::address::Payload;
use dpp::dashcore::{Address, opcodes};
use dpp::identity::core_script::CoreScript;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::encode;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "CoreScript")]
#[derive(Clone)]
pub struct CoreScriptWasm(CoreScript);

impl From<CoreScriptWasm> for CoreScript {
    fn from(value: CoreScriptWasm) -> Self {
        value.0
    }
}

impl From<CoreScript> for CoreScriptWasm {
    fn from(value: CoreScript) -> Self {
        CoreScriptWasm(value)
    }
}

#[wasm_bindgen(js_class = CoreScript)]
impl CoreScriptWasm {
    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        CoreScriptWasm(CoreScript::from_bytes(bytes))
    }

    #[wasm_bindgen(js_name = "fromP2PKH")]
    pub fn from_p2pkh(key_hash: Vec<u8>) -> Self {
        let mut key_hash_bytes = [0u8; 20];
        let bytes = key_hash.as_slice();
        let len = bytes.len().min(key_hash_bytes.len());
        key_hash_bytes[..len].copy_from_slice(&bytes[..len]);

        CoreScriptWasm(CoreScript::new_p2pkh(key_hash_bytes))
    }

    #[wasm_bindgen(js_name = "fromP2SH")]
    pub fn from_p2sh(script_hash: Vec<u8>) -> Self {
        let mut script_hash_bytes = [0u8; 20];
        let bytes = script_hash.as_slice();
        let len = bytes.len().min(script_hash_bytes.len());
        script_hash_bytes[..len].copy_from_slice(&bytes[..len]);

        let mut bytes = vec![
            opcodes::all::OP_HASH160.to_u8(),
            opcodes::all::OP_PUSHBYTES_20.to_u8(),
        ];
        bytes.extend_from_slice(&script_hash_bytes);
        bytes.push(opcodes::all::OP_EQUAL.to_u8());
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toAddress")]
    pub fn to_address(
        &self,
        network: NetworkLikeJs,
    ) -> WasmDppResult<String> {
        let network_wasm: NetworkWasm = network.try_into()?;

        let payload = Payload::from_script(self.0.as_script())
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        let address = Address::new(network_wasm.into(), payload);

        Ok(address.to_string())
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.0.to_string(Base64)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        encode(self.0.to_bytes().as_slice(), Hex)
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> String {
        encode(self.0.to_bytes().as_slice(), Base64)
    }

    #[wasm_bindgen(js_name = "toASMString")]
    pub fn to_asm_string(&self) -> String {
        self.0.to_asm_string()
    }
}

impl_try_from_options!(CoreScriptWasm, "CoreScript");

impl_wasm_type_info!(CoreScriptWasm, CoreScript);
