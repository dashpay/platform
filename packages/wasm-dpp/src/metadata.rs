#![allow(clippy::from_over_into)]

pub use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use dpp::metadata::Metadata;

#[wasm_bindgen(js_name=Metadata)]
pub struct MetadataWasm(Metadata);

impl std::convert::From<Metadata> for MetadataWasm {
    fn from(v: Metadata) -> Self {
        MetadataWasm(v)
    }
}
impl std::convert::Into<Metadata> for MetadataWasm {
    fn into(self) -> Metadata {
        self.0
    }
}

//? probably it should be a separate trait with blanket implementation
#[wasm_bindgen(js_class=Metadata)]
impl MetadataWasm {
    #[wasm_bindgen(js_name=default)]
    pub fn new() -> Self {
        MetadataWasm(Metadata::default())
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from(object: JsValue) -> Self {
        let i: Metadata = serde_json::from_str(&object.as_string().unwrap()).unwrap();
        MetadataWasm(i)
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> JsValue {
        JsValue::from_serde(&self.0).unwrap()
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> JsValue {
        JsValue::from_serde(&self.0).unwrap()
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }
}
