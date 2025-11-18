use dpp::block::block_info::BlockInfo;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "BlockInfo")]
#[derive(Clone)]
pub struct BlockInfoWasm {
    time_ms: u64,
    height: u64,
    core_height: u64,
    epoch_index: u16,
}

#[wasm_bindgen(js_class = BlockInfo)]
impl BlockInfoWasm {
    #[wasm_bindgen(getter = timeMs)]
    pub fn time_ms(&self) -> u64 {
        self.time_ms
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u64 {
        self.height
    }

    #[wasm_bindgen(getter = coreHeight)]
    pub fn core_height(&self) -> u64 {
        self.core_height
    }

    #[wasm_bindgen(getter = epochIndex)]
    pub fn epoch_index(&self) -> u16 {
        self.epoch_index
    }
}

impl From<BlockInfo> for BlockInfoWasm {
    fn from(block: BlockInfo) -> Self {
        Self {
            time_ms: block.time_ms,
            height: block.height,
            core_height: block.core_height as u64,
            epoch_index: block.epoch.index,
        }
    }
}
