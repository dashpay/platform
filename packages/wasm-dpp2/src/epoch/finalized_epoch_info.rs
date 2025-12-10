use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::{JsValueExt, try_to_u64};
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::Identifier;
use js_sys::{BigInt, Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "FinalizedEpochInfo")]
pub struct FinalizedEpochInfoWasm(FinalizedEpochInfo);

impl From<FinalizedEpochInfo> for FinalizedEpochInfoWasm {
    fn from(info: FinalizedEpochInfo) -> Self {
        FinalizedEpochInfoWasm(info)
    }
}

impl From<FinalizedEpochInfoWasm> for FinalizedEpochInfo {
    fn from(info: FinalizedEpochInfoWasm) -> Self {
        info.0
    }
}

impl FinalizedEpochInfoWasm {
    fn v0(&self) -> &FinalizedEpochInfoV0 {
        match &self.0 {
            FinalizedEpochInfo::V0(v0) => v0,
        }
    }

    fn v0_mut(&mut self) -> &mut FinalizedEpochInfoV0 {
        match &mut self.0 {
            FinalizedEpochInfo::V0(v0) => v0,
        }
    }
}

fn block_proposers_from_js(js_value: &JsValue) -> WasmDppResult<BTreeMap<Identifier, u64>> {
    if js_value.is_undefined() || js_value.is_null() {
        return Ok(BTreeMap::new());
    }

    if !js_value.is_object() {
        return Err(WasmDppError::invalid_argument(
            "blockProposers must be an object",
        ));
    }

    let object = Object::from(js_value.clone());
    let keys = Object::keys(&object);
    let mut map = BTreeMap::new();

    for key in keys.iter() {
        let identifier =
            Identifier::from(IdentifierWasm::try_from(key.clone()).map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "invalid block proposer identifier: {}",
                    err
                ))
            })?);

        let value = Reflect::get(&object, &key).map_err(|err| {
            let message = err.error_message();
            WasmDppError::generic(format!(
                "unable to read block proposer '{}': {}",
                identifier.to_string(Encoding::Base58),
                message
            ))
        })?;

        let credits = try_to_u64(value.clone()).map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "block proposer value for '{}' is not a valid u64: {:#}",
                identifier.to_string(Encoding::Base58),
                err
            ))
        })?;

        map.insert(identifier, credits);
    }

    Ok(map)
}

fn block_proposers_to_js(map: &BTreeMap<Identifier, u64>) -> WasmDppResult<JsValue> {
    let object = Object::new();

    for (identifier, value) in map {
        Reflect::set(
            &object,
            &JsValue::from(identifier.to_string(Encoding::Base58)),
            &BigInt::from(*value).into(),
        )
        .map_err(|err| {
            let message = err.error_message();
            WasmDppError::generic(format!(
                "unable to serialize block proposer '{}': {}",
                identifier.to_string(Encoding::Base58),
                message
            ))
        })?;
    }

    Ok(object.into())
}

#[wasm_bindgen(js_class = FinalizedEpochInfo)]
impl FinalizedEpochInfoWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "FinalizedEpochInfo".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name(&self) -> String {
        "FinalizedEpochInfo".to_string()
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_block_time: u64,
        first_block_height: u64,
        total_blocks_in_epoch: u64,
        first_core_block_height: u32,
        next_epoch_start_core_block_height: u32,
        total_processing_fees: u64,
        total_distributed_storage_fees: u64,
        total_created_storage_fees: u64,
        core_block_rewards: u64,
        block_proposers: &JsValue,
        fee_multiplier_permille: u64,
        protocol_version: u32,
    ) -> WasmDppResult<FinalizedEpochInfoWasm> {
        let block_proposers = block_proposers_from_js(block_proposers)?;

        Ok(FinalizedEpochInfoWasm(FinalizedEpochInfo::V0(
            FinalizedEpochInfoV0 {
                first_block_time,
                first_block_height,
                total_blocks_in_epoch,
                first_core_block_height,
                next_epoch_start_core_block_height,
                total_processing_fees,
                total_distributed_storage_fees,
                total_created_storage_fees,
                core_block_rewards,
                block_proposers,
                fee_multiplier_permille,
                protocol_version,
            },
        )))
    }

    #[wasm_bindgen(getter = "firstBlockTime")]
    pub fn first_block_time(&self) -> BigInt {
        BigInt::from(self.v0().first_block_time())
    }

    #[wasm_bindgen(getter = "firstBlockHeight")]
    pub fn first_block_height(&self) -> BigInt {
        BigInt::from(self.v0().first_block_height())
    }

    #[wasm_bindgen(getter = "totalBlocksInEpoch")]
    pub fn total_blocks_in_epoch(&self) -> BigInt {
        BigInt::from(self.v0().total_blocks_in_epoch())
    }

    #[wasm_bindgen(getter = "firstCoreBlockHeight")]
    pub fn first_core_block_height(&self) -> u32 {
        self.v0().first_core_block_height()
    }

    #[wasm_bindgen(getter = "nextEpochStartCoreBlockHeight")]
    pub fn next_epoch_start_core_block_height(&self) -> u32 {
        self.v0().next_epoch_start_core_block_height()
    }

    #[wasm_bindgen(getter = "totalProcessingFees")]
    pub fn total_processing_fees(&self) -> BigInt {
        BigInt::from(self.v0().total_processing_fees())
    }

    #[wasm_bindgen(getter = "totalDistributedStorageFees")]
    pub fn total_distributed_storage_fees(&self) -> BigInt {
        BigInt::from(self.v0().total_distributed_storage_fees())
    }

    #[wasm_bindgen(getter = "totalCreatedStorageFees")]
    pub fn total_created_storage_fees(&self) -> BigInt {
        BigInt::from(self.v0().total_created_storage_fees())
    }

    #[wasm_bindgen(getter = "coreBlockRewards")]
    pub fn core_block_rewards(&self) -> BigInt {
        BigInt::from(self.v0().core_block_rewards())
    }

    #[wasm_bindgen(getter = "blockProposers")]
    pub fn block_proposers(&self) -> WasmDppResult<JsValue> {
        block_proposers_to_js(self.v0().block_proposers())
    }

    #[wasm_bindgen(getter = "feeMultiplierPermille")]
    pub fn fee_multiplier_permille(&self) -> u64 {
        self.v0().fee_multiplier_permille()
    }

    #[wasm_bindgen(getter = "feeMultiplier")]
    pub fn fee_multiplier(&self) -> f64 {
        self.v0().fee_multiplier_permille() as f64 / 1000.0
    }

    #[wasm_bindgen(getter = "protocolVersion")]
    pub fn protocol_version(&self) -> u32 {
        self.v0().protocol_version()
    }

    #[wasm_bindgen(setter = "firstBlockTime")]
    pub fn set_first_block_time(&mut self, first_block_time: u64) {
        self.v0_mut().first_block_time = first_block_time;
    }

    #[wasm_bindgen(setter = "firstBlockHeight")]
    pub fn set_first_block_height(&mut self, first_block_height: u64) {
        self.v0_mut().first_block_height = first_block_height;
    }

    #[wasm_bindgen(setter = "totalBlocksInEpoch")]
    pub fn set_total_blocks_in_epoch(&mut self, total_blocks_in_epoch: u64) {
        self.v0_mut().total_blocks_in_epoch = total_blocks_in_epoch;
    }

    #[wasm_bindgen(setter = "firstCoreBlockHeight")]
    pub fn set_first_core_block_height(&mut self, first_core_block_height: u32) {
        self.v0_mut().first_core_block_height = first_core_block_height;
    }

    #[wasm_bindgen(setter = "nextEpochStartCoreBlockHeight")]
    pub fn set_next_epoch_start_core_block_height(
        &mut self,
        next_epoch_start_core_block_height: u32,
    ) {
        self.v0_mut().next_epoch_start_core_block_height = next_epoch_start_core_block_height;
    }

    #[wasm_bindgen(setter = "totalProcessingFees")]
    pub fn set_total_processing_fees(&mut self, total_processing_fees: u64) {
        self.v0_mut().total_processing_fees = total_processing_fees;
    }

    #[wasm_bindgen(setter = "totalDistributedStorageFees")]
    pub fn set_total_distributed_storage_fees(&mut self, total_distributed_storage_fees: u64) {
        self.v0_mut().total_distributed_storage_fees = total_distributed_storage_fees;
    }

    #[wasm_bindgen(setter = "totalCreatedStorageFees")]
    pub fn set_total_created_storage_fees(&mut self, total_created_storage_fees: u64) {
        self.v0_mut().total_created_storage_fees = total_created_storage_fees;
    }

    #[wasm_bindgen(setter = "coreBlockRewards")]
    pub fn set_core_block_rewards(&mut self, core_block_rewards: u64) {
        self.v0_mut().core_block_rewards = core_block_rewards;
    }

    #[wasm_bindgen(setter = "blockProposers")]
    pub fn set_block_proposers(&mut self, js_block_proposers: &JsValue) -> WasmDppResult<()> {
        let block_proposers = block_proposers_from_js(js_block_proposers)?;
        self.v0_mut().block_proposers = block_proposers;
        Ok(())
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
