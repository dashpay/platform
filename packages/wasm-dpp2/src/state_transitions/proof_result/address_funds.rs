//! Address-funds-related `StateTransitionProofResult` wrappers.

use super::helpers::js_obj;
use crate::IdentityWasm;
use crate::PartialIdentityWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use js_sys::Map;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

// --- VerifiedAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedAddressInfos")]
#[derive(Clone)]
pub struct VerifiedAddressInfosWasm {
    pub(super) address_infos: Map, // Map<string(hex), { address: PlatformAddress, nonce: number, credits: BigInt } | undefined>
}

#[wasm_bindgen(js_class = VerifiedAddressInfos)]
impl VerifiedAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[("addressInfos", self.address_infos.clone().into())])
    }

    /// Returns a `JSON.stringify`-friendly form: the `Map` is normalised to a
    /// plain object so its entries survive serialisation (otherwise
    /// `JSON.stringify({addressInfos: <Map>})` produces `{"addressInfos":{}}`).
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        crate::serialization::conversions::normalize_js_value_for_json(&self.to_object())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedAddressInfosWasm> {
        let address_infos = super::helpers::read_map_property(&value, "addressInfos")?;
        Ok(VerifiedAddressInfosWasm { address_infos })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedAddressInfosWasm> {
        Self::from_object(value)
    }
}

impl_wasm_type_info!(VerifiedAddressInfosWasm, VerifiedAddressInfos);

// --- VerifiedIdentityFullWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedIdentityFullWithAddressInfos")]
#[derive(Clone)]
pub struct VerifiedIdentityFullWithAddressInfosWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: IdentityWasm,
    pub(super) address_infos: Map,
}

#[wasm_bindgen(js_class = VerifiedIdentityFullWithAddressInfos)]
impl VerifiedIdentityFullWithAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let id = self.identity.to_object()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"identity".into(), &id.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        Ok(obj.into())
    }

    /// Returns a `JSON.stringify`-friendly form: the embedded `Map` is
    /// normalised to a plain object so its entries survive serialisation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let id = self.identity.to_json()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"identity".into(), &id.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        crate::serialization::conversions::normalize_js_value_for_json(&obj.into())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedIdentityFullWithAddressInfosWasm> {
        let identity_val = js_sys::Reflect::get(&value, &"identity".into())
            .map_err(|_| WasmDppError::generic("Missing property: identity"))?;
        let identity: IdentityWasm = crate::serialization::conversions::from_object(identity_val)?;
        let address_infos = super::helpers::read_map_property(&value, "addressInfos")?;
        Ok(VerifiedIdentityFullWithAddressInfosWasm {
            identity,
            address_infos,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedIdentityFullWithAddressInfosWasm> {
        let identity_val = js_sys::Reflect::get(&value, &"identity".into())
            .map_err(|_| WasmDppError::generic("Missing property: identity"))?;
        let identity: IdentityWasm = crate::serialization::conversions::from_json(identity_val)?;
        let address_infos = super::helpers::read_map_property(&value, "addressInfos")?;
        Ok(VerifiedIdentityFullWithAddressInfosWasm {
            identity,
            address_infos,
        })
    }
}

impl_wasm_type_info!(
    VerifiedIdentityFullWithAddressInfosWasm,
    VerifiedIdentityFullWithAddressInfos
);

// --- VerifiedIdentityWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedIdentityWithAddressInfos")]
#[derive(Clone)]
pub struct VerifiedIdentityWithAddressInfosWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "partialIdentity")]
    pub partial_identity: PartialIdentityWasm,
    pub(super) address_infos: Map,
}

#[wasm_bindgen(js_class = VerifiedIdentityWithAddressInfos)]
impl VerifiedIdentityWithAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let pi = self.partial_identity.to_object()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"partialIdentity".into(), &pi.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        Ok(obj.into())
    }

    /// Returns a `JSON.stringify`-friendly form: the embedded `Map` is
    /// normalised to a plain object so its entries survive serialisation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let pi = self.partial_identity.to_json()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"partialIdentity".into(), &pi.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        crate::serialization::conversions::normalize_js_value_for_json(&obj.into())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedIdentityWithAddressInfosWasm> {
        let pi_val = js_sys::Reflect::get(&value, &"partialIdentity".into())
            .map_err(|_| WasmDppError::generic("Missing property: partialIdentity"))?;
        let partial_identity: PartialIdentityWasm =
            crate::serialization::conversions::from_object(pi_val)?;
        let address_infos = super::helpers::read_map_property(&value, "addressInfos")?;
        Ok(VerifiedIdentityWithAddressInfosWasm {
            partial_identity,
            address_infos,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedIdentityWithAddressInfosWasm> {
        let pi_val = js_sys::Reflect::get(&value, &"partialIdentity".into())
            .map_err(|_| WasmDppError::generic("Missing property: partialIdentity"))?;
        let partial_identity: PartialIdentityWasm =
            crate::serialization::conversions::from_json(pi_val)?;
        let address_infos = super::helpers::read_map_property(&value, "addressInfos")?;
        Ok(VerifiedIdentityWithAddressInfosWasm {
            partial_identity,
            address_infos,
        })
    }
}

impl_wasm_type_info!(
    VerifiedIdentityWithAddressInfosWasm,
    VerifiedIdentityWithAddressInfos
);
