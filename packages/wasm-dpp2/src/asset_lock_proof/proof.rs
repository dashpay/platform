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
use crate::utils::{IntoWasm, get_class_type};
use dpp::prelude::AssetLockProof;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * AssetLockProof serialized as a plain object.
 *
 * Internally-tagged discriminated union — `$type` discriminates the variant and
 * the variant's fields sit alongside it. Mirrors the rs-dpp serde shape (which
 * uses `#[serde(tag = "$type")]` on the enum) and the convention used by
 * `AddressWitness` / `AddressFundsFeeStrategyStep`.
 */
export type AssetLockProofObject =
    | ({ $type: "instant" } & InstantAssetLockProofObject)
    | ({ $type: "chain" } & ChainAssetLockProofObject);

/**
 * AssetLockProof serialized as JSON.
 */
export type AssetLockProofJSON =
    | ({ $type: "instant" } & InstantAssetLockProofJSON)
    | ({ $type: "chain" } & ChainAssetLockProofJSON);
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
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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

    /// Returns the lock type as a lowercase wire-shape string ("instant" or
    /// "chain") — matching the `$type` discriminator emitted by `toObject()` /
    /// `toJSON()`.
    #[wasm_bindgen(getter = "lockType")]
    pub fn lock_type(&self) -> String {
        match self.0 {
            AssetLockProof::Instant(_) => AssetLockProofTypeWasm::Instant.into(),
            AssetLockProof::Chain(_) => AssetLockProofTypeWasm::Chain.into(),
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
crate::impl_wasm_conversions_inner!(
    AssetLockProofWasm,
    AssetLockProof,
    AssetLockProof,
    AssetLockProofObjectJs,
    AssetLockProofJSONJs
);
