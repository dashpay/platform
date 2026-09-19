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

/// `VerifiedContractModerationListStatus` proof-result wrapper: the target identity's entry on
/// the list a moderation transition edited. The proof holds that one entry, so the other list
/// is unknown: `banned` is undefined unless `list` is `banlist`.
#[wasm_bindgen(js_name = "VerifiedContractModerationListStatus")]
#[derive(Clone)]
pub struct VerifiedContractModerationListStatusWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "contractId")]
    pub contract_id: crate::IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "identityId")]
    pub identity_id: crate::IdentifierWasm,
    /// The list the moderation edited: `banlist` or `suspensions`
    #[wasm_bindgen(getter_with_clone)]
    pub list: String,
    /// When `list` is `banlist`: the identity is on the banlist
    pub banned: Option<bool>,
    /// When `list` is `suspensions`: the block time, in milliseconds, until which the identity
    /// is suspended
    #[wasm_bindgen(js_name = "suspendedUntil")]
    pub suspended_until: Option<u64>,
}

#[wasm_bindgen(js_class = VerifiedContractModerationListStatus)]
impl VerifiedContractModerationListStatusWasm {
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            ("contractId", self.contract_id.into()),
            ("identityId", self.identity_id.into()),
            ("list", JsValue::from_str(&self.list)),
            (
                "banned",
                self.banned
                    .map(JsValue::from_bool)
                    .unwrap_or(JsValue::UNDEFINED),
            ),
            (
                "suspendedUntil",
                self.suspended_until
                    .map(|until| JsValue::from(js_sys::BigInt::from(until)))
                    .unwrap_or(JsValue::UNDEFINED),
            ),
        ]))
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            (
                "contractId",
                JsValue::from_str(&self.contract_id.to_base58()),
            ),
            (
                "identityId",
                JsValue::from_str(&self.identity_id.to_base58()),
            ),
            ("list", JsValue::from_str(&self.list)),
            (
                "banned",
                self.banned
                    .map(JsValue::from_bool)
                    .unwrap_or(JsValue::UNDEFINED),
            ),
            (
                "suspendedUntil",
                self.suspended_until
                    .map(|until| JsValue::from_f64(until as f64))
                    .unwrap_or(JsValue::UNDEFINED),
            ),
        ]))
    }
}

impl_wasm_type_info!(
    VerifiedContractModerationListStatusWasm,
    VerifiedContractModerationListStatus
);
