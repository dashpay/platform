//! `VerifiedDocuments` proof-result wrapper.

use super::helpers::js_obj;
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use js_sys::Map;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "VerifiedDocuments")]
#[derive(Clone)]
pub struct VerifiedDocumentsWasm {
    pub(super) documents: Map, // Map<string(base58), DocumentWasm | undefined>
}

#[wasm_bindgen(js_class = VerifiedDocuments)]
impl VerifiedDocumentsWasm {
    #[wasm_bindgen(getter)]
    pub fn documents(&self) -> Map {
        self.documents.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[("documents", self.documents.clone().into())])
    }

    /// Returns a `JSON.stringify`-friendly form: the `Map` is normalised to a
    /// plain object so its entries survive serialisation (otherwise
    /// `JSON.stringify({documents: <Map>})` produces `{"documents":{}}`).
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        crate::serialization::conversions::normalize_js_value_for_json(&self.to_object())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedDocumentsWasm> {
        let documents = super::helpers::read_map_property(&value, "documents")?;
        Ok(VerifiedDocumentsWasm { documents })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedDocumentsWasm> {
        Self::from_object(value)
    }
}

impl_wasm_type_info!(VerifiedDocumentsWasm, VerifiedDocuments);
