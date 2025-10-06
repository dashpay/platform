pub mod chain;
pub mod instant;
pub mod outpoint;
mod tx_out;

use crate::asset_lock_proof::chain::ChainAssetLockProofWasm;
use crate::asset_lock_proof::instant::InstantAssetLockProofWasm;

use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::enums::lock_types::AssetLockProofTypeWasm;
use crate::identifier::IdentifierWasm;
use crate::utils::{IntoWasm, WithJsError, get_class_type};
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
    pub fn new(js_asset_lock_proof: &JsValue) -> Result<AssetLockProofWasm, JsValue> {
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
            &_ => Err(JsValue::from("Invalid asset lock proof type.")),
        }
    }

    #[wasm_bindgen(js_name = "createInstantAssetLockProof")]
    pub fn new_instant_asset_lock_proof(
        instant_lock: Vec<u8>,
        transaction: Vec<u8>,
        output_index: u32,
    ) -> Result<AssetLockProofWasm, JsValue> {
        Ok(InstantAssetLockProofWasm::new(instant_lock, transaction, output_index)?.into())
    }

    #[wasm_bindgen(js_name = "createChainAssetLockProof")]
    pub fn new_chain_asset_lock_proof(
        core_chain_locked_height: u32,
        out_point: &OutPointWasm,
    ) -> Result<AssetLockProofWasm, JsValue> {
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
    pub fn create_identifier(&self) -> Result<IdentifierWasm, JsValue> {
        let identifier = self.0.create_identifier().with_js_error()?;

        Ok(identifier.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let json_value = self.0.to_raw_object().with_js_error()?;

        Ok(json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_string(&self) -> Result<String, JsValue> {
        Ok(hex::encode(
            serde_json::to_string(&self.0).map_err(|err| JsValue::from(err.to_string()))?,
        ))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(asset_lock_proof: String) -> Result<AssetLockProofWasm, JsValue> {
        let asset_lock_proof_bytes = hex::decode(&asset_lock_proof)
            .map_err(|e| JsValue::from_str(&format!("Invalid asset lock proof hex: {}", e)))?;

        let json_str = String::from_utf8(asset_lock_proof_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid UTF-8 in asset lock proof: {}", e)))?;

        let asset_lock_proof = serde_json::from_str(&json_str).map_err(|e| {
            JsValue::from_str(&format!("Failed to parse asset lock proof JSON: {}", e))
        })?;

        Ok(AssetLockProofWasm(asset_lock_proof))
    }
}
