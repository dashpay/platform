use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_conversions;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "BlockInfo")]
#[derive(Clone)]
pub struct BlockInfoWasm(BlockInfo);

#[wasm_bindgen(js_class = BlockInfo)]
impl BlockInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        time_ms: u64,
        height: u64,
        core_height: u32,
        epoch_index: u16,
    ) -> WasmDppResult<BlockInfoWasm> {
        let epoch =
            Epoch::new(epoch_index).map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

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
        wasm.0
    }
}

impl_wasm_conversions!(BlockInfoWasm, BlockInfo);
