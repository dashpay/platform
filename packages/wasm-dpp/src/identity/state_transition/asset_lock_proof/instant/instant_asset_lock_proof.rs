use dpp::dashcore::blockdata::{script::Script, transaction::txout::TxOut};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    errors::{from_dpp_err, RustConversionError},
    with_js_error,
};
use dpp::identity::state_transition::asset_lock_proof::instant::{
    InstantAssetLockProof, RawInstantLock,
};

#[derive(Serialize, Deserialize)]
#[serde(remote = "TxOut")]
struct TxOutJS {
    #[serde(rename = "satoshis")]
    value: u64,
    #[serde(rename = "script")]
    script_pubkey: Script,
}

#[derive(Serialize)]
struct TxOutSerdeHelper<'a>(#[serde(with = "TxOutJS")] &'a TxOut);

#[wasm_bindgen(js_name=InstantAssetLockProof)]
pub struct InstantAssetLockProofWasm(InstantAssetLockProof);

impl From<InstantAssetLockProof> for InstantAssetLockProofWasm {
    fn from(v: InstantAssetLockProof) -> Self {
        InstantAssetLockProofWasm(v)
    }
}

#[wasm_bindgen(js_class = InstantAssetLockProof)]
impl InstantAssetLockProofWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<InstantAssetLockProofWasm, JsValue> {
        let raw_instant_lock: RawInstantLock =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;

        let instant_asset_lock_proof: InstantAssetLockProof =
            raw_instant_lock.try_into().map_err(|_| {
                RustConversionError::Error(String::from("object passed is not a raw Instant Lock"))
                    .to_js_value()
            })?;

        Ok(instant_asset_lock_proof.into())
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.0.asset_lock_type()
    }

    #[wasm_bindgen(js_name=getOutputIndex)]
    pub fn get_output_index(&self) -> usize {
        self.0.output_index()
    }

    #[wasm_bindgen(js_name=getOutPoint)]
    pub fn get_out_point(&self) -> Option<Buffer> {
        self.0
            .out_point()
            .map(|out_point| Buffer::from_bytes(out_point.as_slice()))
    }

    #[wasm_bindgen(js_name=getOutput)]
    pub fn get_output(&self) -> Result<JsValue, JsValue> {
        let output = self.0.output().unwrap();
        let output_json_string =
            serde_json::to_string(&TxOutSerdeHelper(output)).map_err(|e| from_dpp_err(e.into()))?;

        let js_object = js_sys::JSON::parse(&output_json_string)?;
        Ok(js_object)
    }
}
