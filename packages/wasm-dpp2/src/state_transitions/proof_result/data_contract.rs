//! `VerifiedDataContract` proof-result wrapper.

use super::helpers::js_obj;
use crate::DataContractWasm;
use crate::PlatformVersionLikeJs;
use crate::data_contract::{DataContractJSONJs, DataContractObjectJs};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "VerifiedDataContract")]
#[derive(Clone)]
pub struct VerifiedDataContractWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "dataContract")]
    pub data_contract: DataContractWasm,
}

#[wasm_bindgen(js_class = VerifiedDataContract)]
impl VerifiedDataContractWasm {
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(
        &self,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<JsValue> {
        let dc = self.data_contract.to_object(platform_version)?;
        Ok(js_obj(&[("dataContract", dc.into())]))
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(
        &self,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<JsValue> {
        let dc = self.data_contract.to_json(platform_version)?;
        Ok(js_obj(&[("dataContract", dc.into())]))
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(
        value: JsValue,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<VerifiedDataContractWasm> {
        let dc_val = js_sys::Reflect::get(&value, &"dataContract".into())
            .map_err(|_| WasmDppError::generic("Missing property: dataContract"))?;
        let data_contract = DataContractWasm::from_object(
            dc_val.unchecked_into::<DataContractObjectJs>(),
            false,
            platform_version,
        )?;
        Ok(VerifiedDataContractWasm { data_contract })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(
        value: JsValue,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<VerifiedDataContractWasm> {
        let dc_val = js_sys::Reflect::get(&value, &"dataContract".into())
            .map_err(|_| WasmDppError::generic("Missing property: dataContract"))?;
        let data_contract = DataContractWasm::from_json(
            dc_val.unchecked_into::<DataContractJSONJs>(),
            false,
            platform_version,
        )?;
        Ok(VerifiedDataContractWasm { data_contract })
    }
}

impl_wasm_type_info!(VerifiedDataContractWasm, VerifiedDataContract);
