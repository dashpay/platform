pub mod chain;
pub mod instant;
pub mod outpoint;
mod tx_out;

use crate::asset_lock_proof::chain::ChainAssetLockProofWasm;
use crate::asset_lock_proof::instant::InstantAssetLockProofWasm;

use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::enums::lock_types::AssetLockProofTypeWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::{IntoWasm, get_class_type};
use dpp::prelude::AssetLockProof;
use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "AssetLockProof")]
#[derive(Clone)]
pub struct AssetLockProofWasm(AssetLockProof);

impl From<AssetLockProofWasm> for AssetLockProof {
    fn from(proof: AssetLockProofWasm) -> Self {
        proof.0
    }
}

impl From<AssetLockProof> for AssetLockProofWasm {
    fn from(proof: AssetLockProof) -> Self {
        AssetLockProofWasm(proof)
    }
}

impl From<ChainAssetLockProofWasm> for AssetLockProofWasm {
    fn from(proof: ChainAssetLockProofWasm) -> Self {
        AssetLockProofWasm(AssetLockProof::Chain(proof.into()))
    }
}

impl From<InstantAssetLockProofWasm> for AssetLockProofWasm {
    fn from(proof: InstantAssetLockProofWasm) -> Self {
        AssetLockProofWasm(AssetLockProof::Instant(proof.into()))
    }
}

impl From<AssetLockProof> for ChainAssetLockProofWasm {
    fn from(proof: AssetLockProof) -> ChainAssetLockProofWasm {
        match proof {
            AssetLockProof::Chain(chain) => ChainAssetLockProofWasm::from(chain),
            _ => panic!("invalid asset lock proof. must contains chain lock"),
        }
    }
}

impl From<AssetLockProof> for InstantAssetLockProofWasm {
    fn from(proof: AssetLockProof) -> InstantAssetLockProofWasm {
        match proof {
            AssetLockProof::Instant(instant) => InstantAssetLockProofWasm::from(instant),
            _ => panic!("invalid asset lock proof. must contains chain lock"),
        }
    }
}

#[wasm_bindgen(js_class = AssetLockProof)]
impl AssetLockProofWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "AssetLockProof".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "AssetLockProof".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(js_asset_lock_proof: &JsValue) -> WasmDppResult<AssetLockProofWasm> {
        match get_class_type(js_asset_lock_proof)?.as_str() {
            "ChainAssetLockProof" => {
                let chain_lock = js_asset_lock_proof
                    .to_wasm::<ChainAssetLockProofWasm>("ChainAssetLockProof")?
                    .clone();

                Ok(AssetLockProofWasm::from(chain_lock))
            }
            "InstantAssetLockProof" => {
                let instant_lock = js_asset_lock_proof
                    .to_wasm::<InstantAssetLockProofWasm>("InstantAssetLockProof")?
                    .clone();

                Ok(AssetLockProofWasm::from(instant_lock))
            }
            &_ => Err(WasmDppError::invalid_argument(
                "Invalid asset lock proof type",
            )),
        }
    }

    #[wasm_bindgen(js_name = "createInstantAssetLockProof")]
    pub fn new_instant_asset_lock_proof(
        instant_lock: Vec<u8>,
        transaction: Vec<u8>,
        output_index: u32,
    ) -> WasmDppResult<AssetLockProofWasm> {
        Ok(InstantAssetLockProofWasm::new(instant_lock, transaction, output_index)?.into())
    }

    #[wasm_bindgen(js_name = "createChainAssetLockProof")]
    pub fn new_chain_asset_lock_proof(
        core_chain_locked_height: u32,
        out_point: &OutPointWasm,
    ) -> WasmDppResult<AssetLockProofWasm> {
        Ok(ChainAssetLockProofWasm::new(core_chain_locked_height, out_point)?.into())
    }

    #[wasm_bindgen(js_name = "getLockType")]
    pub fn get_lock_type(&self) -> String {
        match self.0 {
            AssetLockProof::Chain(_) => AssetLockProofTypeWasm::Chain.into(),
            AssetLockProof::Instant(_) => AssetLockProofTypeWasm::Instant.into(),
        }
    }

    #[wasm_bindgen(js_name = "getInstantLockProof")]
    pub fn get_instant_lock(&self) -> InstantAssetLockProofWasm {
        self.clone().0.into()
    }

    #[wasm_bindgen(js_name = "getChainLockProof")]
    pub fn get_chain_lock(&self) -> ChainAssetLockProofWasm {
        self.clone().0.into()
    }

    #[wasm_bindgen(js_name = "getOutPoint")]
    pub fn get_out_point(&self) -> Option<OutPointWasm> {
        match self.0.out_point() {
            Some(out_point) => Some(OutPointWasm::from(out_point)),
            None => None,
        }
    }

    #[wasm_bindgen(js_name = "createIdentityId")]
    pub fn create_identifier(&self) -> WasmDppResult<IdentifierWasm> {
        let identifier = self.0.create_identifier()?;

        Ok(identifier.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let json_value = self.0.to_raw_object()?;

        json_value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_string(&self) -> WasmDppResult<String> {
        let json = serde_json::to_string(&self.0)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        Ok(hex::encode(json))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(asset_lock_proof: String) -> WasmDppResult<AssetLockProofWasm> {
        let asset_lock_proof_bytes = hex::decode(&asset_lock_proof).map_err(|e| {
            WasmDppError::serialization(format!("Invalid asset lock proof hex: {}", e))
        })?;

        let json_str = String::from_utf8(asset_lock_proof_bytes).map_err(|e| {
            WasmDppError::serialization(format!("Invalid UTF-8 in asset lock proof: {}", e))
        })?;

        let asset_lock_proof: AssetLockProof = serde_json::from_str(&json_str).map_err(|e| {
            WasmDppError::serialization(format!("Failed to parse asset lock proof JSON: {}", e))
        })?;

        Ok(AssetLockProofWasm(asset_lock_proof))
    }
}
