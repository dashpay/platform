use crate::error::{WasmDppError, WasmDppResult};
use dpp::dashcore::{ScriptBuf, TxOut};
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "TxOut")]
#[derive(Clone)]
pub struct TxOutWasm(TxOut);

impl From<TxOut> for TxOutWasm {
    fn from(value: TxOut) -> Self {
        TxOutWasm(value)
    }
}

impl From<TxOutWasm> for TxOut {
    fn from(value: TxOutWasm) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_class = TxOut)]
impl TxOutWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TxOut".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TxOut".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(value: u64, script_pubkey: JsValue) -> WasmDppResult<TxOutWasm> {
        let tx_out: WasmDppResult<TxOut> = match script_pubkey.is_array() {
            true => Ok(TxOut {
                value,
                script_pubkey: ScriptBuf::from_bytes(Uint8Array::from(script_pubkey).to_vec()),
            }),
            false => match script_pubkey.is_string() {
                true => {
                    let hex = script_pubkey.as_string().ok_or_else(|| {
                        WasmDppError::invalid_argument("Script pubkey must be string")
                    })?;

                    let script = ScriptBuf::from_hex(&hex)
                        .map_err(|err| WasmDppError::serialization(err.to_string()))?;

                    Ok(TxOut {
                        value,
                        script_pubkey: script,
                    })
                }
                false => Err(WasmDppError::invalid_argument("Invalid script pubkey")),
            },
        };

        Ok(TxOutWasm(tx_out?))
    }

    #[wasm_bindgen(getter = "value")]
    pub fn get_value(&self) -> u64 {
        self.0.value
    }

    #[wasm_bindgen(getter = "scriptPubKeyHex")]
    pub fn get_script_pubkey_hex(&self) -> String {
        self.0.script_pubkey.to_hex_string()
    }

    #[wasm_bindgen(getter = "scriptPubKeyBytes")]
    pub fn get_script_pubkey_bytes(&self) -> Vec<u8> {
        self.0.script_pubkey.to_bytes()
    }

    #[wasm_bindgen(setter = "value")]
    pub fn set_value(&mut self, value: u64) {
        self.0.value = value;
    }

    #[wasm_bindgen(setter = "scriptPubKeyHex")]
    pub fn set_script_pubkey_hex(&mut self, script_pubkey_hex: String) -> WasmDppResult<()> {
        self.0.script_pubkey = ScriptBuf::from_hex(&script_pubkey_hex)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(setter = "scriptPubKeyBytes")]
    pub fn set_script_pubkey_bytes(&mut self, script_pubkey_bytes: Vec<u8>) {
        self.0.script_pubkey = ScriptBuf::from_bytes(script_pubkey_bytes);
    }

    #[wasm_bindgen(js_name = "getScriptPubKeyASM")]
    pub fn get_script_pubkey_asm(&self) -> String {
        self.0.script_pubkey.to_asm_string()
    }
}
