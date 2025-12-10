use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::{JsValueExt, js_value_to_vec_u8};
use bincode::serde::{decode_from_slice, encode_to_vec};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use js_sys::{Object, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "ChainAssetLockProof")]
#[derive(Clone, Serialize, Deserialize)]
pub struct ChainAssetLockProofWasm(ChainAssetLockProof);

impl From<ChainAssetLockProofWasm> for ChainAssetLockProof {
    fn from(chain_lock: ChainAssetLockProofWasm) -> Self {
        chain_lock.0
    }
}

impl From<ChainAssetLockProof> for ChainAssetLockProofWasm {
    fn from(chain_lock: ChainAssetLockProof) -> Self {
        ChainAssetLockProofWasm(chain_lock)
    }
}

#[wasm_bindgen(js_class = ChainAssetLockProof)]
impl ChainAssetLockProofWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "ChainAssetLockProof".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "ChainAssetLockProof".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        core_chain_locked_height: u32,
        out_point: &OutPointWasm,
    ) -> WasmDppResult<ChainAssetLockProofWasm> {
        Ok(ChainAssetLockProofWasm(ChainAssetLockProof {
            core_chain_locked_height,
            out_point: out_point.clone().into(),
        }))
    }

    #[wasm_bindgen(setter = "coreChainLockedHeight")]
    pub fn set_core_chain_locked_height(&mut self, core_chain_locked_height: u32) {
        self.0.core_chain_locked_height = core_chain_locked_height;
    }

    #[wasm_bindgen(setter = "outPoint")]
    pub fn set_out_point(&mut self, outpoint: &OutPointWasm) {
        self.0.out_point = outpoint.clone().into();
    }

    #[wasm_bindgen(getter = "coreChainLockedHeight")]
    pub fn get_core_chain_locked_height(&self) -> u32 {
        self.0.core_chain_locked_height
    }

    #[wasm_bindgen(getter = "outPoint")]
    pub fn get_out_point(&self) -> OutPointWasm {
        self.0.out_point.into()
    }

    #[wasm_bindgen(js_name = "createIdentityId")]
    pub fn create_identifier(&self) -> IdentifierWasm {
        let identifier = self.0.create_identifier();

        identifier.into()
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        // Use default serializer (non-human-readable) - bytes stay as Uint8Array
        serde_wasm_bindgen::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        // Serialize to serde_json::Value first (human-readable, so outPoint becomes base64)
        // then convert to JS value
        let json_value = serde_json::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        json_value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<ChainAssetLockProofWasm> {
        let (core_chain_locked_height, out_point) = parse_chain_asset_lock_proof_fields(js_value)?;
        let out_point_wasm = OutPointWasm::from_bytes(out_point);
        Ok(ChainAssetLockProofWasm(ChainAssetLockProof {
            core_chain_locked_height,
            out_point: out_point_wasm.into(),
        }))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<ChainAssetLockProofWasm> {
        // Convert JS value to serde_json::Value, then deserialize (human-readable)
        let json_value: serde_json::Value = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        let proof: ChainAssetLockProof = serde_json::from_value(json_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(ChainAssetLockProofWasm(proof))
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        encode_to_vec(&self.0, bincode::config::standard())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ChainAssetLockProofWasm> {
        let proof: ChainAssetLockProof = decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?
            .0;
        Ok(ChainAssetLockProofWasm(proof))
    }
}

fn parse_chain_asset_lock_proof_fields(js_value: JsValue) -> WasmDppResult<(u32, Vec<u8>)> {
    let object = js_value.dyn_into::<Object>().map_err(|_| {
        WasmDppError::invalid_argument("ChainAssetLockProof expects an object".to_string())
    })?;

    let height_js =
        Reflect::get(&object, &JsValue::from_str("coreChainLockedHeight")).map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "unable to read coreChainLockedHeight: {}",
                err.error_message()
            ))
        })?;
    let out_point_js = Reflect::get(&object, &JsValue::from_str("outPoint")).map_err(|err| {
        WasmDppError::invalid_argument(format!("unable to read outPoint: {}", err.error_message()))
    })?;

    let core_chain_locked_height = height_js.as_f64().ok_or_else(|| {
        WasmDppError::invalid_argument("coreChainLockedHeight must be a number".to_string())
    })? as u32;

    let out_point = js_value_to_vec_u8(&out_point_js)?;
    if out_point.len() != 36 {
        return Err(WasmDppError::invalid_argument(
            "outPoint must contain exactly 36 bytes".to_string(),
        ));
    }

    Ok((core_chain_locked_height, out_point))
}
