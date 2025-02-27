#![allow(clippy::from_over_into)]

pub use serde::Serialize;
use wasm_bindgen::prelude::*;

use dpp::metadata::Metadata;
use dpp::util::deserializer::ProtocolVersion;

#[wasm_bindgen(js_name=Metadata)]
#[derive(Clone, Debug)]
pub struct MetadataWasm(Metadata);

impl From<Metadata> for MetadataWasm {
    fn from(v: Metadata) -> Self {
        MetadataWasm(v)
    }
}

impl From<&MetadataWasm> for Metadata {
    fn from(v: &MetadataWasm) -> Self {
        v.0
    }
}

impl Into<Metadata> for MetadataWasm {
    fn into(self) -> Metadata {
        self.0
    }
}

//? probably it should be a separate trait with blanket implementation
#[wasm_bindgen(js_class=Metadata)]
impl MetadataWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        block_height: u64,
        core_chain_locked_height: u32,
        time_ms: u64,
        protocol_version: u32,
    ) -> Result<MetadataWasm, JsValue> {
        let inner = Metadata {
            block_height,
            core_chain_locked_height: core_chain_locked_height as u64,
            time_ms,
            protocol_version: protocol_version as u64 as ProtocolVersion,
        };
        Ok(inner.into())
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

    #[wasm_bindgen(js_name=getBlockHeight)]
    pub fn block_height(&self) -> u64 {
        self.0.block_height
    }

    #[wasm_bindgen(js_name=getCoreChainLockedHeight)]
    pub fn core_chain_locked_height(&self) -> u32 {
        self.0.core_chain_locked_height as u32
    }

    #[wasm_bindgen(js_name=getTimeMs)]
    pub fn time_ms(&self) -> u64 {
        self.0.time_ms
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn protocol_version(&self) -> u32 {
        self.0.protocol_version
    }
}
