use crate::asset_lock_proof::chain::{
    ChainAssetLockProofJSONJs, ChainAssetLockProofObjectJs, ChainAssetLockProofWasm,
};
use crate::asset_lock_proof::instant::{
    InstantAssetLockProofJSONJs, InstantAssetLockProofObjectJs, InstantAssetLockProofWasm,
};
use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::enums::lock_types::AssetLockProofTypeWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_try_from_js_value;
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, get_class_type, try_from_options};
use dpp::prelude::AssetLockProof;
use js_sys::Reflect;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * AssetLockProof serialized as a plain object.
 * Type 0 = Instant, Type 1 = Chain.
 */
export type AssetLockProofObject =
    | ({ type: 0 } & InstantAssetLockProofObject)
    | ({ type: 1 } & ChainAssetLockProofObject);

/**
 * AssetLockProof serialized as JSON.
 * Type 0 = Instant, Type 1 = Chain.
 */
export type AssetLockProofJSON =
    | ({ type: 0 } & InstantAssetLockProofJSON)
    | ({ type: 1 } & ChainAssetLockProofJSON);
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AssetLockProofObject")]
    pub type AssetLockProofObjectJs;

    #[wasm_bindgen(typescript_type = "AssetLockProofJSON")]
    pub type AssetLockProofJSONJs;
}

impl From<AssetLockProofObjectJs> for InstantAssetLockProofObjectJs {
    fn from(value: AssetLockProofObjectJs) -> Self {
        JsValue::from(value).into()
    }
}

impl From<AssetLockProofObjectJs> for ChainAssetLockProofObjectJs {
    fn from(value: AssetLockProofObjectJs) -> Self {
        JsValue::from(value).into()
    }
}

impl From<AssetLockProofJSONJs> for InstantAssetLockProofJSONJs {
    fn from(value: AssetLockProofJSONJs) -> Self {
        JsValue::from(value).into()
    }
}

impl From<AssetLockProofJSONJs> for ChainAssetLockProofJSONJs {
    fn from(value: AssetLockProofJSONJs) -> Self {
        JsValue::from(value).into()
    }
}

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(
            unchecked_param_type = "ChainAssetLockProof | InstantAssetLockProof",
            js_name = "assetLockProof"
        )]
        asset_lock_proof: &JsValue,
    ) -> WasmDppResult<AssetLockProofWasm> {
        match get_class_type(asset_lock_proof)?.as_str() {
            "ChainAssetLockProof" => {
                let chain_lock = asset_lock_proof
                    .to_wasm::<ChainAssetLockProofWasm>("ChainAssetLockProof")?
                    .clone();

                Ok(AssetLockProofWasm::from(chain_lock))
            }
            "InstantAssetLockProof" => {
                let instant_lock = asset_lock_proof
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
        #[wasm_bindgen(js_name = "instantLock")] instant_lock: Vec<u8>,
        transaction: Vec<u8>,
        #[wasm_bindgen(js_name = "outputIndex")] output_index: u32,
    ) -> WasmDppResult<AssetLockProofWasm> {
        Ok(InstantAssetLockProofWasm::constructor(instant_lock, transaction, output_index)?.into())
    }

    #[wasm_bindgen(js_name = "createChainAssetLockProof")]
    pub fn new_chain_asset_lock_proof(
        #[wasm_bindgen(js_name = "coreChainLockedHeight")] core_chain_locked_height: u32,
        #[wasm_bindgen(js_name = "outPoint")] out_point: &OutPointWasm,
    ) -> WasmDppResult<AssetLockProofWasm> {
        Ok(ChainAssetLockProofWasm::constructor(core_chain_locked_height, out_point)?.into())
    }

    #[wasm_bindgen(getter = "lockType")]
    pub fn lock_type(&self) -> AssetLockProofTypeWasm {
        match self.0 {
            AssetLockProof::Chain(_) => AssetLockProofTypeWasm::Chain,
            AssetLockProof::Instant(_) => AssetLockProofTypeWasm::Instant,
        }
    }

    #[wasm_bindgen(getter = "lockTypeName")]
    pub fn lock_type_name(&self) -> String {
        match self.0 {
            AssetLockProof::Chain(_) => AssetLockProofTypeWasm::Chain.into(),
            AssetLockProof::Instant(_) => AssetLockProofTypeWasm::Instant.into(),
        }
    }

    #[wasm_bindgen(getter = "instantLockProof")]
    pub fn instant_lock_proof(&self) -> InstantAssetLockProofWasm {
        self.clone().0.into()
    }

    #[wasm_bindgen(getter = "chainLockProof")]
    pub fn chain_lock_proof(&self) -> ChainAssetLockProofWasm {
        self.clone().0.into()
    }

    #[wasm_bindgen(getter = "outPoint")]
    pub fn out_point(&self) -> Option<OutPointWasm> {
        self.0.out_point().map(OutPointWasm::from)
    }

    #[wasm_bindgen(js_name = "createIdentityId")]
    pub fn create_identifier(&self) -> WasmDppResult<IdentifierWasm> {
        let identifier = self.0.create_identifier()?;

        Ok(identifier.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<AssetLockProofObjectJs> {
        let inner_object: JsValue = match &self.0 {
            AssetLockProof::Chain(chain) => ChainAssetLockProofWasm::from(chain.clone())
                .to_object()?
                .into(),
            AssetLockProof::Instant(instant) => InstantAssetLockProofWasm::from(instant.clone())
                .to_object()?
                .into(),
        };

        // Add type field: 0 = Instant, 1 = Chain
        let proof_type: u8 = match &self.0 {
            AssetLockProof::Instant(_) => 0,
            AssetLockProof::Chain(_) => 1,
        };
        Reflect::set(
            &inner_object,
            &JsValue::from_str("type"),
            &JsValue::from(proof_type),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Ok(inner_object.into())
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(object: AssetLockProofObjectJs) -> WasmDppResult<AssetLockProofWasm> {
        let proof_type: AssetLockProofTypeWasm = try_from_options(&object, "type")?;

        match proof_type {
            AssetLockProofTypeWasm::Instant => {
                InstantAssetLockProofWasm::from_object(object.into()).map(AssetLockProofWasm::from)
            }
            AssetLockProofTypeWasm::Chain => {
                ChainAssetLockProofWasm::from_object(object.into()).map(AssetLockProofWasm::from)
            }
        }
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<AssetLockProofJSONJs> {
        let inner_json: JsValue = match &self.0 {
            AssetLockProof::Chain(chain) => ChainAssetLockProofWasm::from(chain.clone())
                .to_json()?
                .into(),
            AssetLockProof::Instant(instant) => InstantAssetLockProofWasm::from(instant.clone())
                .to_json()?
                .into(),
        };

        // Add type field: 0 = Instant, 1 = Chain
        let proof_type: u8 = match &self.0 {
            AssetLockProof::Instant(_) => 0,
            AssetLockProof::Chain(_) => 1,
        };
        Reflect::set(
            &inner_json,
            &JsValue::from_str("type"),
            &JsValue::from(proof_type),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Ok(inner_json.into())
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(object: AssetLockProofJSONJs) -> WasmDppResult<AssetLockProofWasm> {
        let proof_type: AssetLockProofTypeWasm = try_from_options(&object, "type")?;

        match proof_type {
            AssetLockProofTypeWasm::Instant => {
                InstantAssetLockProofWasm::from_json(object.into()).map(AssetLockProofWasm::from)
            }
            AssetLockProofTypeWasm::Chain => {
                ChainAssetLockProofWasm::from_json(object.into()).map(AssetLockProofWasm::from)
            }
        }
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = bincode::encode_to_vec(&self.0, bincode::config::standard())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(hex::encode(bytes))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(
        #[wasm_bindgen(js_name = "assetLockProof")] asset_lock_proof: String,
    ) -> WasmDppResult<AssetLockProofWasm> {
        let bytes = hex::decode(asset_lock_proof)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        let proof: AssetLockProof = bincode::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?
            .0;
        Ok(AssetLockProofWasm(proof))
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        bincode::encode_to_vec(&self.0, bincode::config::standard())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<AssetLockProofWasm> {
        let proof: AssetLockProof = bincode::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?
            .0;
        Ok(AssetLockProofWasm(proof))
    }
}

impl_try_from_js_value!(AssetLockProofWasm, "AssetLockProof");
impl_try_from_options!(AssetLockProofWasm);
impl_wasm_type_info!(AssetLockProofWasm, AssetLockProof);
