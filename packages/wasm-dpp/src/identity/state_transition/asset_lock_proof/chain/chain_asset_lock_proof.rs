use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    errors::{from_dpp_err, RustConversionError},
    identifier::IdentifierWrapper,
    with_js_error,
};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::util::string_encoding;
use dpp::util::string_encoding::Encoding;

#[wasm_bindgen(js_name=ChainAssetLockProof)]
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
    pub fn new(raw_parameters: JsValue) -> Result<ChainAssetLockProofWasm, JsValue> {
        let parameters: ChainAssetLockProofParams =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;

        let out_point: [u8; 36] = parameters.out_point.try_into().map_err(|_| {
            RustConversionError::Error(String::from("outPoint must be a 36 byte array"))
                .to_js_value()
        })?;

        let chain_asset_lock_proof =
            ChainAssetLockProof::new(parameters.core_chain_locked_height, out_point);

        Ok(ChainAssetLockProofWasm(chain_asset_lock_proof))
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        ChainAssetLockProof::asset_lock_type()
    }

    #[wasm_bindgen(js_name=getCoreChainLockedHeight)]
    pub fn get_core_chain_locked_height(&self) -> u32 {
        self.0.core_chain_locked_height()
    }

    #[wasm_bindgen(js_name=getOutPoint)]
    pub fn get_out_point(&self) -> Buffer {
        Buffer::from_bytes(self.0.out_point().as_slice())
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let js_object = self.to_object()?;

        let out_point_base64 = string_encoding::encode(self.0.out_point(), Encoding::Base64);

        js_sys::Reflect::set(
            &js_object,
            &"outPoint".to_owned().into(),
            &JsValue::from(out_point_base64),
        )?;

        Ok(js_object)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let asset_lock_json =
            serde_json::to_value(self.0.clone()).map_err(|e| from_dpp_err(e.into()))?;

        let asset_lock_json_string =
            serde_json::to_string(&asset_lock_json).map_err(|e| from_dpp_err(e.into()))?;
        let js_object = js_sys::JSON::parse(&asset_lock_json_string)?;

        let out_point = self.get_out_point();

        js_sys::Reflect::set(
            &js_object,
            &"outPoint".to_owned().into(),
            &JsValue::from(out_point),
        )?;

        Ok(js_object)
    }

    #[wasm_bindgen(js_name=createIdentifier)]
    pub fn create_identifier(&self) -> Result<IdentifierWrapper, JsValue> {
        let identifier = self
            .0
            .create_identifier()
            .map_err(|e| from_dpp_err(e.into()))?;
        Ok(identifier.into())
    }
}
