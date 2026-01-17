use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
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

#[wasm_bindgen(typescript_custom_section)]
const FINALIZED_EPOCH_INFO_OPTIONS_TS: &'static str = r#"
export interface FinalizedEpochInfoOptions {
    firstBlockTime: bigint | number;
    firstBlockHeight: bigint | number;
    totalBlocksInEpoch: bigint | number;
    firstCoreBlockHeight: number;
    nextEpochStartCoreBlockHeight: number;
    totalProcessingFees: bigint | number;
    totalDistributedStorageFees: bigint | number;
    totalCreatedStorageFees: bigint | number;
    coreBlockRewards: bigint | number;
    blockProposers: Record<string, bigint | number>;
    feeMultiplierPermille: bigint | number;
    protocolVersion: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "FinalizedEpochInfoOptions")]
    pub type FinalizedEpochInfoOptionsJs;
}

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: FinalizedEpochInfoOptionsJs) -> WasmDppResult<FinalizedEpochInfoWasm> {
        let options_obj = Object::from(JsValue::from(options));

        let first_block_time = try_to_u64(
            Reflect::get(&options_obj, &"firstBlockTime".into())
                .map_err(|_| WasmDppError::invalid_argument("firstBlockTime is required"))?,
        )?;

        let first_block_height = try_to_u64(
            Reflect::get(&options_obj, &"firstBlockHeight".into())
                .map_err(|_| WasmDppError::invalid_argument("firstBlockHeight is required"))?,
        )?;

        let total_blocks_in_epoch = try_to_u64(
            Reflect::get(&options_obj, &"totalBlocksInEpoch".into())
                .map_err(|_| WasmDppError::invalid_argument("totalBlocksInEpoch is required"))?,
        )?;

        let first_core_block_height = Reflect::get(&options_obj, &"firstCoreBlockHeight".into())
            .map_err(|_| WasmDppError::invalid_argument("firstCoreBlockHeight is required"))?
            .as_f64()
            .ok_or_else(|| WasmDppError::invalid_argument("firstCoreBlockHeight must be a number"))?
            as u32;

        let next_epoch_start_core_block_height =
            Reflect::get(&options_obj, &"nextEpochStartCoreBlockHeight".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument("nextEpochStartCoreBlockHeight is required")
                })?
                .as_f64()
                .ok_or_else(|| {
                    WasmDppError::invalid_argument("nextEpochStartCoreBlockHeight must be a number")
                })? as u32;

        let total_processing_fees = try_to_u64(
            Reflect::get(&options_obj, &"totalProcessingFees".into())
                .map_err(|_| WasmDppError::invalid_argument("totalProcessingFees is required"))?,
        )?;

        let total_distributed_storage_fees = try_to_u64(
            Reflect::get(&options_obj, &"totalDistributedStorageFees".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument("totalDistributedStorageFees is required")
                })?,
        )?;

        let total_created_storage_fees = try_to_u64(
            Reflect::get(&options_obj, &"totalCreatedStorageFees".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument("totalCreatedStorageFees is required")
                })?,
        )?;

        let core_block_rewards = try_to_u64(
            Reflect::get(&options_obj, &"coreBlockRewards".into())
                .map_err(|_| WasmDppError::invalid_argument("coreBlockRewards is required"))?,
        )?;

        let block_proposers_js = Reflect::get(&options_obj, &"blockProposers".into())
            .map_err(|_| WasmDppError::invalid_argument("blockProposers is required"))?;
        let block_proposers = block_proposers_from_js(&block_proposers_js)?;

        let fee_multiplier_permille = try_to_u64(
            Reflect::get(&options_obj, &"feeMultiplierPermille".into())
                .map_err(|_| WasmDppError::invalid_argument("feeMultiplierPermille is required"))?,
        )?;

        let protocol_version = Reflect::get(&options_obj, &"protocolVersion".into())
            .map_err(|_| WasmDppError::invalid_argument("protocolVersion is required"))?
            .as_f64()
            .ok_or_else(|| WasmDppError::invalid_argument("protocolVersion must be a number"))?
            as u32;

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
    pub fn set_block_proposers(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<string, number>")] block_proposers: &JsValue,
    ) -> WasmDppResult<()> {
        let block_proposers_map = block_proposers_from_js(block_proposers)?;
        self.v0_mut().block_proposers = block_proposers_map;
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

crate::impl_wasm_conversions!(FinalizedEpochInfoWasm, FinalizedEpochInfo);
impl_wasm_type_info!(FinalizedEpochInfoWasm, FinalizedEpochInfo);
