use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::error::{WasmDppError, WasmDppResult};
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::hash_types::CycleHash;
use dpp::dashcore::hashes::hex::FromHex;
use dpp::dashcore::secp256k1::hashes::hex::Case::Lower;
use dpp::dashcore::secp256k1::hashes::hex::DisplayHex;
use dpp::dashcore::{InstantLock, Txid};
use js_sys::{Array, Object, Reflect};
use std::str::FromStr;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "InstantLock")]
#[derive(Clone)]
pub struct InstantLockWasm(InstantLock);

impl From<InstantLockWasm> for InstantLock {
    fn from(value: InstantLockWasm) -> Self {
        value.0
    }
}

impl From<InstantLock> for InstantLockWasm {
    fn from(value: InstantLock) -> Self {
        InstantLockWasm(value)
    }
}

#[wasm_bindgen(js_class = InstantLock)]
impl InstantLockWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "InstantLock".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "InstantLock".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        version: u8,
        js_inputs: &js_sys::Array,
        txid: String,
        cycle_hash: String,
        bls_signature: String,
    ) -> WasmDppResult<InstantLockWasm> {
        let inputs = OutPointWasm::vec_from_js_value(js_inputs)?;

        Ok(InstantLockWasm(InstantLock {
            version,
            inputs: inputs.iter().map(|input| input.clone().into()).collect(),
            txid: Txid::from_hex(&txid)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
            cyclehash: CycleHash::from_str(&cycle_hash)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
            signature: BLSSignature::from_hex(&bls_signature)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
        }))
    }

    #[wasm_bindgen(getter = "version")]
    pub fn get_version(&self) -> u8 {
        self.0.version
    }

    #[wasm_bindgen(getter = "inputs")]
    pub fn get_inputs(&self) -> Vec<OutPointWasm> {
        self.0.inputs.iter().map(|input| (*input).into()).collect()
    }

    #[wasm_bindgen(getter = "txid")]
    pub fn get_txid(&self) -> String {
        self.0.txid.to_hex()
    }

    #[wasm_bindgen(getter = "cyclehash")]
    pub fn get_cycle_hash(&self) -> String {
        self.0.cyclehash.to_string()
    }

    #[wasm_bindgen(getter = "blsSignature")]
    pub fn get_bls_signature(&self) -> String {
        self.0.signature.to_bytes().to_hex_string(Lower)
    }

    #[wasm_bindgen(setter = "version")]
    pub fn set_version(&mut self, v: u8) {
        self.0.version = v;
    }

    #[wasm_bindgen(setter = "inputs")]
    pub fn set_inputs(&mut self, inputs: &js_sys::Array) -> WasmDppResult<()> {
        let inputs = OutPointWasm::vec_from_js_value(inputs)?;
        self.0.inputs = inputs.iter().map(|input| input.clone().into()).collect();
        Ok(())
    }

    #[wasm_bindgen(setter = "txid")]
    pub fn set_txid(&mut self, txid: String) -> WasmDppResult<()> {
        self.0.txid =
            Txid::from_hex(&txid).map_err(|err| WasmDppError::serialization(err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(setter = "cyclehash")]
    pub fn set_cycle_hash(&mut self, cycle_hash: String) -> WasmDppResult<()> {
        self.0.cyclehash = CycleHash::from_str(&cycle_hash)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(setter = "blsSignature")]
    pub fn set_bls_signature(&mut self, bls_signature: String) -> WasmDppResult<()> {
        self.0.signature = BLSSignature::from_hex(&bls_signature)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"version".into(), &JsValue::from(self.0.version))
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        let inputs_arr = Array::new();
        for input in &self.0.inputs {
            let input_wasm = OutPointWasm::from(*input);
            inputs_arr.push(&input_wasm.to_hex().into());
        }
        Reflect::set(&obj, &"inputs".into(), &inputs_arr)
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Reflect::set(&obj, &"txid".into(), &self.0.txid.to_hex().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(&obj, &"cyclehash".into(), &self.0.cyclehash.to_string().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"blsSignature".into(),
            &self.0.signature.to_bytes().to_hex_string(Lower).into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<InstantLockWasm> {
        let version = Reflect::get(&js_value, &"version".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_f64()
            .ok_or_else(|| WasmDppError::invalid_argument("version must be a number"))?
            as u8;

        let inputs_arr = Reflect::get(&js_value, &"inputs".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        let inputs_array = Array::from(&inputs_arr);
        let mut inputs = Vec::new();
        for i in 0..inputs_array.length() {
            let hex = inputs_array
                .get(i)
                .as_string()
                .ok_or_else(|| WasmDppError::invalid_argument("input must be a hex string"))?;
            let outpoint = OutPointWasm::from_hex(hex)?;
            inputs.push(outpoint.into());
        }

        let txid = Reflect::get(&js_value, &"txid".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("txid must be a string"))?;

        let cyclehash = Reflect::get(&js_value, &"cyclehash".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("cyclehash must be a string"))?;

        let bls_signature = Reflect::get(&js_value, &"blsSignature".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("blsSignature must be a string"))?;

        Ok(InstantLockWasm(InstantLock {
            version,
            inputs,
            txid: Txid::from_hex(&txid)
                .map_err(|e| WasmDppError::serialization(e.to_string()))?,
            cyclehash: CycleHash::from_str(&cyclehash)
                .map_err(|e| WasmDppError::serialization(e.to_string()))?,
            signature: BLSSignature::from_hex(&bls_signature)
                .map_err(|e| WasmDppError::serialization(e.to_string()))?,
        }))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"version".into(), &JsValue::from(self.0.version))
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        let inputs_arr = Array::new();
        for input in &self.0.inputs {
            let input_wasm = OutPointWasm::from(*input);
            inputs_arr.push(&JsValue::from(input_wasm));
        }
        Reflect::set(&obj, &"inputs".into(), &inputs_arr)
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Reflect::set(&obj, &"txid".into(), &self.0.txid.to_hex().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(&obj, &"cyclehash".into(), &self.0.cyclehash.to_string().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"blsSignature".into(),
            &self.0.signature.to_bytes().to_hex_string(Lower).into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<InstantLockWasm> {
        // fromObject accepts the same format as fromJSON for InstantLock
        InstantLockWasm::from_json(js_value)
    }
}
