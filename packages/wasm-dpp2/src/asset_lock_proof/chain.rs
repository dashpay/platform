use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::serde_format;
use bincode::serde::{decode_from_slice, encode_to_vec};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use serde::{Deserialize, Serialize};
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
        // Non-human-readable: OutPoint serializes as 36 bytes (Uint8Array)
        serde_format::to_object(&self.0)
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        // Human-readable: OutPoint serializes as "txid:vout" string
        serde_format::to_json(&self.0)
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<ChainAssetLockProofWasm> {
        // Non-human-readable: OutPoint expects 36 bytes (Uint8Array)
        let proof: ChainAssetLockProof = serde_format::from_object(js_value)?;
        Ok(ChainAssetLockProofWasm(proof))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<ChainAssetLockProofWasm> {
        // Human-readable: OutPoint expects "txid:vout" string
        let proof: ChainAssetLockProof = serde_format::from_json(js_value)?;
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
