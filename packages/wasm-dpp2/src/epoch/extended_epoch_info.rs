use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::block::extended_epoch_info::v0::{ExtendedEpochInfoV0, ExtendedEpochInfoV0Getters};
use js_sys::BigInt;
use wasm_bindgen::prelude::wasm_bindgen;

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "ExtendedEpochInfo".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "ExtendedEpochInfo".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        index: u16,
        first_block_time: u64,
        first_block_height: u64,
        first_core_block_height: u32,
        fee_multiplier_permille: u64,
        protocol_version: u32,
    ) -> ExtendedEpochInfoWasm {
        ExtendedEpochInfoWasm(ExtendedEpochInfo::V0(ExtendedEpochInfoV0 {
            index,
            first_block_time,
            first_block_height,
            first_core_block_height,
            fee_multiplier_permille,
            protocol_version,
        }))
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
    pub fn set_index(&mut self, index: u16) {
        self.v0_mut().index = index;
    }

    #[wasm_bindgen(setter = "firstBlockTime")]
    pub fn set_first_block_time(&mut self, first_block_time: u64) {
        self.v0_mut().first_block_time = first_block_time;
    }

    #[wasm_bindgen(setter = "firstBlockHeight")]
    pub fn set_first_block_height(&mut self, first_block_height: u64) {
        self.v0_mut().first_block_height = first_block_height;
    }

    #[wasm_bindgen(setter = "firstCoreBlockHeight")]
    pub fn set_first_core_block_height(&mut self, first_core_block_height: u32) {
        self.v0_mut().first_core_block_height = first_core_block_height;
    }

    #[wasm_bindgen(setter = "feeMultiplierPermille")]
    pub fn set_fee_multiplier_permille(&mut self, fee_multiplier_permille: u64) {
        self.v0_mut().fee_multiplier_permille = fee_multiplier_permille;
    }

    #[wasm_bindgen(setter = "protocolVersion")]
    pub fn set_protocol_version(&mut self, protocol_version: u32) {
        self.v0_mut().protocol_version = protocol_version;
    }
}
