use crate::asset_lock_proof::chain::ChainAssetLockProofWasm;
use crate::asset_lock_proof::instant::InstantAssetLockProofWasm;

use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::enums::lock_types::AssetLockProofTypeWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::{IntoWasm, JsValueExt, get_class_type};
use dpp::prelude::AssetLockProof;
use js_sys::{Object, Reflect};
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
        self.0.out_point().map(OutPointWasm::from)
    }

    #[wasm_bindgen(js_name = "createIdentityId")]
    pub fn create_identifier(&self) -> WasmDppResult<IdentifierWasm> {
        let identifier = self.0.create_identifier()?;

        Ok(identifier.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        match &self.0 {
            AssetLockProof::Chain(chain) => {
                ChainAssetLockProofWasm::from(chain.clone()).to_object()
            }
            AssetLockProof::Instant(instant) => {
                InstantAssetLockProofWasm::from(instant.clone()).to_object()
            }
        }
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<AssetLockProofWasm> {
        if let Ok(existing) = js_value.to_wasm::<AssetLockProofWasm>("AssetLockProof") {
            return Ok(existing.clone());
        }

        if let Ok(chain) = js_value.to_wasm::<ChainAssetLockProofWasm>("ChainAssetLockProof") {
            return Ok(AssetLockProofWasm::from(chain.clone()));
        }

        if let Ok(instant) = js_value.to_wasm::<InstantAssetLockProofWasm>("InstantAssetLockProof")
        {
            return Ok(AssetLockProofWasm::from(instant.clone()));
        }

        if js_value.is_object() {
            let object = Object::from(js_value.clone());
            if Reflect::has(&object, &JsValue::from_str("instantLock"))
                .map_err(|e| WasmDppError::invalid_argument(e.error_message()))?
            {
                return InstantAssetLockProofWasm::from_object(js_value)
                    .map(AssetLockProofWasm::from);
            }

            if Reflect::has(&object, &JsValue::from_str("outPoint"))
                .map_err(|e| WasmDppError::invalid_argument(e.error_message()))?
            {
                return ChainAssetLockProofWasm::from_object(js_value)
                    .map(AssetLockProofWasm::from);
            }
        }

        Err(WasmDppError::invalid_argument(
            "Unsupported AssetLockProof object".to_string(),
        ))
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        match &self.0 {
            AssetLockProof::Chain(chain) => ChainAssetLockProofWasm::from(chain.clone()).to_json(),
            AssetLockProof::Instant(instant) => {
                InstantAssetLockProofWasm::from(instant.clone()).to_json()
            }
        }
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<AssetLockProofWasm> {
        AssetLockProofWasm::from_object(js_value)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = bincode::encode_to_vec(&self.0, bincode::config::standard())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(hex::encode(bytes))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(asset_lock_proof: String) -> WasmDppResult<AssetLockProofWasm> {
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
