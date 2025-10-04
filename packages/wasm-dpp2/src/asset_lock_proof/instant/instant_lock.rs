use crate::asset_lock_proof::outpoint::OutPointWASM;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::hash_types::CycleHash;
use dpp::dashcore::hashes::hex::FromHex;
use dpp::dashcore::secp256k1::hashes::hex::Case::Lower;
use dpp::dashcore::secp256k1::hashes::hex::DisplayHex;
use dpp::dashcore::{InstantLock, Txid};
use std::str::FromStr;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "InstantLockWASM")]
#[derive(Clone)]
pub struct InstantLockWASM(InstantLock);

impl From<InstantLockWASM> for InstantLock {
    fn from(value: InstantLockWASM) -> Self {
        value.0
    }
}

impl From<InstantLock> for InstantLockWASM {
    fn from(value: InstantLock) -> Self {
        InstantLockWASM(value)
    }
}

#[wasm_bindgen]
impl InstantLockWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "InstantLockWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "InstantLockWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        version: u8,
        js_inputs: &js_sys::Array,
        txid: String,
        cycle_hash: String,
        bls_signature: String,
    ) -> Result<InstantLockWASM, JsValue> {
        let inputs = OutPointWASM::vec_from_js_value(js_inputs)?;

        Ok(InstantLockWASM(InstantLock {
            version,
            inputs: inputs.iter().map(|input| input.clone().into()).collect(),
            txid: Txid::from_hex(&txid).map_err(|err| JsValue::from(err.to_string()))?,
            cyclehash: CycleHash::from_str(&cycle_hash)
                .map_err(|err| JsValue::from(err.to_string()))?,
            signature: BLSSignature::from_hex(&bls_signature)
                .map_err(|err| JsValue::from(err.to_string()))?,
        }))
    }

    #[wasm_bindgen(getter = "version")]
    pub fn get_version(&self) -> u8 {
        self.0.version
    }

    #[wasm_bindgen(getter = "inputs")]
    pub fn get_inputs(&self) -> Vec<OutPointWASM> {
        self.0
            .inputs
            .iter()
            .map(|input| input.clone().into())
            .collect()
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
    pub fn set_inputs(&mut self, inputs: &js_sys::Array) -> Result<(), JsValue> {
        let inputs = OutPointWASM::vec_from_js_value(inputs)?;
        self.0.inputs = inputs.iter().map(|input| input.clone().into()).collect();
        Ok(())
    }

    #[wasm_bindgen(setter = "txid")]
    pub fn set_txid(&mut self, txid: String) -> Result<(), JsValue> {
        self.0.txid = Txid::from_hex(&txid).map_err(|err| JsValue::from(err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(setter = "cyclehash")]
    pub fn set_cycle_hash(&mut self, cycle_hash: String) -> Result<(), JsValue> {
        self.0.cyclehash =
            CycleHash::from_str(&cycle_hash).map_err(|err| JsValue::from(err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(setter = "blsSignature")]
    pub fn set_bls_signature(&mut self, bls_signature: String) -> Result<(), JsValue> {
        self.0.signature =
            BLSSignature::from_hex(&bls_signature).map_err(|err| JsValue::from(err.to_string()))?;
        Ok(())
    }
}
