#![allow(clippy::from_over_into)]

pub use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use dpp::metadata::Metadata;

#[wasm_bindgen(js_name=Metadata)]
#[derive(Debug)]
pub struct MetadataWasm(Metadata);

impl std::convert::From<Metadata> for MetadataWasm {
    fn from(v: Metadata) -> Self {
        MetadataWasm(v)
    }
}

impl std::convert::From<&MetadataWasm> for Metadata {
    fn from(v: &MetadataWasm) -> Self {
        v.0.clone()
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
    #[wasm_bindgen(constructor)]
    pub fn new(block_height: JsValue, core_chain_locked_height: JsValue) -> Self {
        // TODO: return an error instead of unwraps
        let block_height = block_height.as_f64().unwrap() as u64;
        let core_chain_locked_height = core_chain_locked_height.as_f64().unwrap() as u64;
        let inner = Metadata {
            block_height,
            core_chain_locked_height,
            ..Default::default()
        };
        inner.into()
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from(object: JsValue) -> Result<MetadataWasm, JsValue> {
        let metadata: Metadata = serde_wasm_bindgen::from_value(object)?;

        Ok(MetadataWasm(metadata))
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.0).unwrap()
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> JsValue {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        self.0.serialize(&serializer).expect("implements Serialize")
    }
}
