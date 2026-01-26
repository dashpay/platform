use crate::error::WasmDppResult;
use crate::utils::{try_from_options_with, try_to_object, try_to_u16, try_to_u32, try_to_u64};
use crate::{impl_wasm_conversions, impl_wasm_type_info};
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::block::extended_epoch_info::v0::{ExtendedEpochInfoV0, ExtendedEpochInfoV0Getters};
use js_sys::BigInt;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const EXTENDED_EPOCH_INFO_OPTIONS_TS: &str = r#"
export interface ExtendedEpochInfoOptions {
    index: number;
    firstBlockTime: bigint;
    firstBlockHeight: bigint;
    firstCoreBlockHeight: number;
    feeMultiplierPermille: bigint;
    protocolVersion: number;
}

/**
 * ExtendedEpochInfo serialized as a plain object.
 */
export interface ExtendedEpochInfoObject {
    index: number;
    firstBlockTime: bigint;
    firstBlockHeight: bigint;
    firstCoreBlockHeight: number;
    feeMultiplierPermille: bigint;
    protocolVersion: number;
}

/**
 * ExtendedEpochInfo serialized as JSON.
 */
export interface ExtendedEpochInfoJSON {
    index: number;
    firstBlockTime: string;
    firstBlockHeight: string;
    firstCoreBlockHeight: number;
    feeMultiplierPermille: string;
    protocolVersion: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ExtendedEpochInfoOptions")]
    pub type ExtendedEpochInfoOptionsJs;

    #[wasm_bindgen(typescript_type = "ExtendedEpochInfoObject")]
    pub type ExtendedEpochInfoObjectJs;

    #[wasm_bindgen(typescript_type = "ExtendedEpochInfoJSON")]
    pub type ExtendedEpochInfoJSONJs;
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "ExtendedEpochInfo")]
pub struct ExtendedEpochInfoWasm(ExtendedEpochInfo);

impl From<ExtendedEpochInfo> for ExtendedEpochInfoWasm {
    fn from(info: ExtendedEpochInfo) -> Self {
        ExtendedEpochInfoWasm(info)
    }
}

impl From<ExtendedEpochInfoWasm> for ExtendedEpochInfo {
    fn from(info: ExtendedEpochInfoWasm) -> Self {
        info.0
    }
}

impl ExtendedEpochInfoWasm {
    fn v0_mut(&mut self) -> &mut ExtendedEpochInfoV0 {
        match &mut self.0 {
            ExtendedEpochInfo::V0(v0) => v0,
        }
    }
}

#[wasm_bindgen(js_class = ExtendedEpochInfo)]
impl ExtendedEpochInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: ExtendedEpochInfoOptionsJs,
    ) -> WasmDppResult<ExtendedEpochInfoWasm> {
        let options_obj = try_to_object(options.into(), "options")?;

        let index = try_from_options_with(&options_obj, "index", |v| try_to_u16(v, "index"))?;

        let first_block_time = try_from_options_with(&options_obj, "firstBlockTime", |v| {
            try_to_u64(v, "firstBlockTime")
        })?;

        let first_block_height = try_from_options_with(&options_obj, "firstBlockHeight", |v| {
            try_to_u64(v, "firstBlockHeight")
        })?;

        let first_core_block_height =
            try_from_options_with(&options_obj, "firstCoreBlockHeight", |v| {
                try_to_u32(v, "firstCoreBlockHeight")
            })?;

        let fee_multiplier_permille =
            try_from_options_with(&options_obj, "feeMultiplierPermille", |v| {
                try_to_u64(v, "feeMultiplierPermille")
            })?;

        let protocol_version = try_from_options_with(&options_obj, "protocolVersion", |v| {
            try_to_u32(v, "protocolVersion")
        })?;

        Ok(ExtendedEpochInfoWasm(ExtendedEpochInfo::V0(
            ExtendedEpochInfoV0 {
                index,
                first_block_time,
                first_block_height,
                first_core_block_height,
                fee_multiplier_permille,
                protocol_version,
            },
        )))
    }

    #[wasm_bindgen(getter = "index")]
    pub fn index(&self) -> u16 {
        self.0.index()
    }

    #[wasm_bindgen(getter = "firstBlockTime")]
    pub fn first_block_time(&self) -> BigInt {
        BigInt::from(self.0.first_block_time())
    }

    #[wasm_bindgen(getter = "firstBlockHeight")]
    pub fn first_block_height(&self) -> BigInt {
        BigInt::from(self.0.first_block_height())
    }

    #[wasm_bindgen(getter = "firstCoreBlockHeight")]
    pub fn first_core_block_height(&self) -> u32 {
        self.0.first_core_block_height()
    }

    #[wasm_bindgen(getter = "feeMultiplierPermille")]
    pub fn fee_multiplier_permille(&self) -> u64 {
        self.0.fee_multiplier_permille()
    }

    #[wasm_bindgen(getter = "feeMultiplier")]
    pub fn fee_multiplier(&self) -> f64 {
        self.0.fee_multiplier_permille() as f64 / 1000.0
    }

    #[wasm_bindgen(getter = "protocolVersion")]
    pub fn protocol_version(&self) -> u32 {
        self.0.protocol_version()
    }

    #[wasm_bindgen(setter = "index")]
    pub fn set_index(&mut self, index: &js_sys::Number) -> WasmDppResult<()> {
        self.v0_mut().index = try_to_u16(index, "index")?;
        Ok(())
    }

    #[wasm_bindgen(setter = "firstBlockTime")]
    pub fn set_first_block_time(
        &mut self,
        #[wasm_bindgen(js_name = "firstBlockTime")] first_block_time: &js_sys::BigInt,
    ) -> WasmDppResult<()> {
        self.v0_mut().first_block_time = try_to_u64(first_block_time, "firstBlockTime")?;
        Ok(())
    }

    #[wasm_bindgen(setter = "firstBlockHeight")]
    pub fn set_first_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "firstBlockHeight")] first_block_height: &js_sys::BigInt,
    ) -> WasmDppResult<()> {
        self.v0_mut().first_block_height = try_to_u64(first_block_height, "firstBlockHeight")?;
        Ok(())
    }

    #[wasm_bindgen(setter = "firstCoreBlockHeight")]
    pub fn set_first_core_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "firstCoreBlockHeight")] first_core_block_height: &js_sys::Number,
    ) -> WasmDppResult<()> {
        self.v0_mut().first_core_block_height = try_to_u32(first_core_block_height, "firstCoreBlockHeight")?;
        Ok(())
    }

    #[wasm_bindgen(setter = "feeMultiplierPermille")]
    pub fn set_fee_multiplier_permille(
        &mut self,
        #[wasm_bindgen(js_name = "feeMultiplierPermille")] fee_multiplier_permille: u64,
    ) {
        self.v0_mut().fee_multiplier_permille = fee_multiplier_permille;
    }

    #[wasm_bindgen(setter = "protocolVersion")]
    pub fn set_protocol_version(
        &mut self,
        #[wasm_bindgen(js_name = "protocolVersion")] protocol_version: u32,
    ) {
        self.v0_mut().protocol_version = protocol_version;
    }
}

impl_wasm_conversions!(
    ExtendedEpochInfoWasm,
    ExtendedEpochInfo,
    ExtendedEpochInfoObjectJs,
    ExtendedEpochInfoJSONJs
);
impl_wasm_type_info!(ExtendedEpochInfoWasm, ExtendedEpochInfo);
