use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use bincode::serde::{decode_from_slice, encode_to_vec};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use serde::{Deserialize, Serialize};
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

impl_wasm_conversions!(ChainAssetLockProofWasm, ChainAssetLockProof);
impl_wasm_type_info!(ChainAssetLockProofWasm, ChainAssetLockProof);
