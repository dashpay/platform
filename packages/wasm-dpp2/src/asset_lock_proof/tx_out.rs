use crate::error::{WasmDppError, WasmDppResult};
use dpp::dashcore::{ScriptBuf, TxOut};
use js_sys::{Object, Reflect, Uint8Array};
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

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"value".into(), &JsValue::from(self.0.value))
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"scriptPubKey".into(),
            &self.0.script_pubkey.to_hex_string().into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<TxOutWasm> {
        let value = Reflect::get(&js_value, &"value".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_f64()
            .ok_or_else(|| WasmDppError::invalid_argument("value must be a number"))?
            as u64;

        let script_hex = Reflect::get(&js_value, &"scriptPubKey".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("scriptPubKey must be a string"))?;

        let script_pubkey = ScriptBuf::from_hex(&script_hex)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        Ok(TxOutWasm(TxOut {
            value,
            script_pubkey,
        }))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"value".into(), &JsValue::from(self.0.value))
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"scriptPubKey".into(),
            &Uint8Array::from(self.0.script_pubkey.as_bytes()).into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<TxOutWasm> {
        let value = Reflect::get(&js_value, &"value".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_f64()
            .ok_or_else(|| WasmDppError::invalid_argument("value must be a number"))?
            as u64;

        let script_js = Reflect::get(&js_value, &"scriptPubKey".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        let script_pubkey = if script_js.is_string() {
            let hex = script_js
                .as_string()
                .ok_or_else(|| WasmDppError::invalid_argument("scriptPubKey must be a string"))?;
            ScriptBuf::from_hex(&hex).map_err(|e| WasmDppError::serialization(e.to_string()))?
        } else {
            ScriptBuf::from_bytes(Uint8Array::from(script_js).to_vec())
        };

        Ok(TxOutWasm(TxOut {
            value,
            script_pubkey,
        }))
    }
}
