use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::OutPoint;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProofType;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::utils::WithJsError;
use crate::{
    buffer::Buffer, errors::RustConversionError, identifier::IdentifierWrapper, with_js_error,
};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::platform_value::string_encoding;
use dpp::platform_value::string_encoding::Encoding;

#[wasm_bindgen(js_name=ChainAssetLockProof)]
#[derive(Clone)]
pub struct ChainAssetLockProofWasm(ChainAssetLockProof);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChainAssetLockProofParams {
    core_chain_locked_height: u32,
    out_point: Vec<u8>,
}

impl From<ChainAssetLockProof> for ChainAssetLockProofWasm {
    fn from(v: ChainAssetLockProof) -> Self {
        ChainAssetLockProofWasm(v)
    }
}

impl From<ChainAssetLockProofWasm> for ChainAssetLockProof {
    fn from(v: ChainAssetLockProofWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ChainAssetLockProof)]
impl ChainAssetLockProofWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<ChainAssetLockProofWasm, JsError> {
        let parameters: ChainAssetLockProofParams = serde_wasm_bindgen::from_value(raw_parameters)
            .map_err(|_| JsError::new("invalid raw chain lock proof"))?;

        let out_point: [u8; 36] = parameters
            .out_point
            .try_into()
            .map_err(|_| JsError::new("outPoint must be a 36 byte array"))?;

        let chain_asset_lock_proof =
            ChainAssetLockProof::new(parameters.core_chain_locked_height, out_point);

        Ok(ChainAssetLockProofWasm(chain_asset_lock_proof))
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        AssetLockProofType::Chain as u8
    }

    #[wasm_bindgen(js_name=getCoreChainLockedHeight)]
    pub fn get_core_chain_locked_height(&self) -> u32 {
        self.0.core_chain_locked_height
    }

    #[wasm_bindgen(js_name=setCoreChainLockedHeight)]
    pub fn set_core_chain_locked_height(&mut self, value: u32) {
        self.0.core_chain_locked_height = value;
    }

    #[wasm_bindgen(js_name=getOutPoint)]
    pub fn get_out_point(&self) -> Result<Buffer, JsValue> {
        let mut outpoint_bytes = Vec::new();

        self.0
            .out_point
            .consensus_encode(&mut outpoint_bytes)
            .map_err(|e| e.to_string())?;

        Ok(Buffer::from_bytes_owned(outpoint_bytes))
    }

    #[wasm_bindgen(js_name=setOutPoint)]
    pub fn set_out_point(&mut self, out_point: Vec<u8>) -> Result<(), JsValue> {
        let out_point: [u8; 36] = out_point.try_into().map_err(|_| {
            RustConversionError::Error(String::from("outPoint must be a 36 byte array"))
                .to_js_value()
        })?;

        self.0.out_point = OutPoint::from(out_point);

        Ok(())
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let js_object = self.to_object()?;

        let mut outpoint_bytes = Vec::new();

        self.0
            .out_point
            .consensus_encode(&mut outpoint_bytes)
            .map_err(|e| e.to_string())?;

        let out_point_base64 = string_encoding::encode(&outpoint_bytes, Encoding::Base64);

        js_sys::Reflect::set(
            &js_object,
            &"outPoint".to_owned().into(),
            &JsValue::from(out_point_base64),
        )?;

        Ok(js_object)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let asset_lock_value = self.0.to_cleaned_object().with_js_error()?;

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_object = with_js_error!(asset_lock_value.serialize(&serializer))?;

        let out_point = self.get_out_point()?;

        js_sys::Reflect::set(
            &js_object,
            &"outPoint".to_owned().into(),
            &JsValue::from(out_point),
        )?;

        Ok(js_object)
    }

    #[wasm_bindgen(js_name=createIdentifier)]
    pub fn create_identifier(&self) -> IdentifierWrapper {
        self.0.create_identifier().into()
    }
}
