use crate::error::{WasmDppError, WasmDppResult};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "BlockInfo")]
#[derive(Clone)]
pub struct BlockInfoWasm(BlockInfo);

#[wasm_bindgen(js_class = BlockInfo)]
impl BlockInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(time_ms: u64, height: u64, core_height: u32, epoch_index: u16) -> WasmDppResult<BlockInfoWasm> {
        let epoch = Epoch::new(epoch_index)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(BlockInfoWasm(BlockInfo {
            time_ms,
            height,
            core_height,
            epoch,
        }))
    }

    #[wasm_bindgen(getter = timeMs)]
    pub fn time_ms(&self) -> u64 {
        self.0.time_ms
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u64 {
        self.0.height
    }

    #[wasm_bindgen(getter = coreHeight)]
    pub fn core_height(&self) -> u32 {
        self.0.core_height
    }

    #[wasm_bindgen(getter = epochIndex)]
    pub fn epoch_index(&self) -> u16 {
        self.0.epoch.index
    }

    /// Serialize to JSON (human-readable format)
    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let json_value = serde_json::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        json_value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Deserialize from JSON
    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<BlockInfoWasm> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        let block_info: BlockInfo = serde_json::from_value(json_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(BlockInfoWasm(block_info))
    }

    /// Serialize to JS object (binary-preserving format)
    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        serde_wasm_bindgen::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Deserialize from JS object
    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<BlockInfoWasm> {
        let block_info: BlockInfo = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(BlockInfoWasm(block_info))
    }
}

impl From<BlockInfo> for BlockInfoWasm {
    fn from(block: BlockInfo) -> Self {
        BlockInfoWasm(block)
    }
}

impl From<BlockInfoWasm> for BlockInfo {
    fn from(wasm: BlockInfoWasm) -> Self {
        wasm.0
    }
}

impl From<&BlockInfoWasm> for BlockInfo {
    fn from(wasm: &BlockInfoWasm) -> Self {
        wasm.0.clone()
    }
}
