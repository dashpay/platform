use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockInfoOptions {
    time_ms: u64,
    height: u64,
    core_height: u32,
    epoch_index: u16,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for creating a BlockInfo instance.
 */
export interface BlockInfoOptions {
    timeMs: bigint;
    height: bigint;
    coreHeight: number;
    epochIndex: number;
}

/**
 * BlockInfo serialized as a plain object.
 */
export interface BlockInfoObject {
    timeMs: bigint;
    height: bigint;
    coreHeight: number;
    epochIndex: number;
}

/**
 * BlockInfo serialized as JSON.
 * u64 fields are numbers when within JS safe integer range, strings when exceeding it.
 */
export interface BlockInfoJSON {
    timeMs: number | string;
    height: number | string;
    coreHeight: number;
    epochIndex: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BlockInfoOptions")]
    pub type BlockInfoOptionsJs;

    #[wasm_bindgen(typescript_type = "BlockInfoObject")]
    pub type BlockInfoObjectJs;

    #[wasm_bindgen(typescript_type = "BlockInfoJSON")]
    pub type BlockInfoJSONJs;
}

#[wasm_bindgen(js_name = "BlockInfo")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BlockInfoWasm(BlockInfo);

#[wasm_bindgen(js_class = BlockInfo)]
impl BlockInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: BlockInfoOptionsJs) -> WasmDppResult<BlockInfoWasm> {
        let opts: BlockInfoOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let epoch = Epoch::new(opts.epoch_index)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(BlockInfoWasm(BlockInfo {
            time_ms: opts.time_ms,
            height: opts.height,
            core_height: opts.core_height,
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

impl_wasm_conversions_inner!(
    BlockInfoWasm,
    BlockInfo,
    BlockInfo,
    BlockInfoObjectJs,
    BlockInfoJSONJs
);

impl_wasm_type_info!(BlockInfoWasm, BlockInfo);
