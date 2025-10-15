use dpp::block::block_info::BlockInfo;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "BlockInfo")]
#[derive(Clone)]
pub struct BlockInfoWasm {
    #[wasm_bindgen(getter = "timeMs")]
    pub time_ms: u64,
    #[wasm_bindgen(getter)]
    pub height: u64,
    #[wasm_bindgen(getter = "coreHeight")]
    pub core_height: u64,
    #[wasm_bindgen(getter = "epochIndex")]
    pub epoch_index: u16,
}

impl From<BlockInfo> for BlockInfoWasm {
    fn from(block: BlockInfo) -> Self {
        Self {
            time_ms: block.time_ms,
            height: block.height as u64,
            core_height: block.core_height as u64,
            epoch_index: block.epoch.index,
        }
    }
}
