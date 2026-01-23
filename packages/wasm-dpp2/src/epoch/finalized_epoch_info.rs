use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_from_for_extern_type;
use crate::impl_wasm_type_info;
use crate::utils::{get_required_property, try_to_object, try_to_u32, try_to_u64};
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use dpp::prelude::Identifier;
use js_sys::{BigInt, Map, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

#[wasm_bindgen(typescript_custom_section)]
const FINALIZED_EPOCH_INFO_OPTIONS_TS: &'static str = r#"
/**
 * Block proposers mapping: Identifier -> block count (bigint).
 */
export type BlockProposersMap = Map<Identifier, bigint>;

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
    blockProposers: BlockProposersMap;
    feeMultiplierPermille: bigint | number;
    protocolVersion: number;
}

/**
 * FinalizedEpochInfo serialized as a plain object.
 */
export interface FinalizedEpochInfoObject {
    firstBlockTime: bigint;
    firstBlockHeight: bigint;
    totalBlocksInEpoch: bigint;
    firstCoreBlockHeight: number;
    nextEpochStartCoreBlockHeight: number;
    totalProcessingFees: bigint;
    totalDistributedStorageFees: bigint;
    totalCreatedStorageFees: bigint;
    coreBlockRewards: bigint;
    blockProposers: BlockProposersMap;
    feeMultiplierPermille: bigint;
    protocolVersion: number;
}

/**
 * FinalizedEpochInfo serialized as JSON.
 */
export interface FinalizedEpochInfoJSON {
    firstBlockTime: string;
    firstBlockHeight: string;
    totalBlocksInEpoch: string;
    firstCoreBlockHeight: number;
    nextEpochStartCoreBlockHeight: number;
    totalProcessingFees: string;
    totalDistributedStorageFees: string;
    totalCreatedStorageFees: string;
    coreBlockRewards: string;
    blockProposers: Record<string, string>;
    feeMultiplierPermille: string;
    protocolVersion: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "FinalizedEpochInfoOptions")]
    pub type FinalizedEpochInfoOptionsJs;

    #[wasm_bindgen(typescript_type = "BlockProposersMap")]
    pub type BlockProposersMapJs;

    #[wasm_bindgen(typescript_type = "FinalizedEpochInfoObject")]
    pub type FinalizedEpochInfoObjectJs;

    #[wasm_bindgen(typescript_type = "FinalizedEpochInfoJSON")]
    pub type FinalizedEpochInfoJSONJs;
}

impl_from_for_extern_type!(BlockProposersMapJs, Map);

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

fn block_proposers_from_map(js_map: &Map) -> WasmDppResult<BTreeMap<Identifier, u64>> {
    let mut map = BTreeMap::new();

    // Iterate using entries()
    let entries = js_map.entries();
    loop {
        let next = entries.next().map_err(|e| {
            WasmDppError::invalid_argument(format!("Failed to iterate map entries: {:?}", e))
        })?;

        let done = Reflect::get(&next, &JsValue::from_str("done"))
            .map_err(|_| WasmDppError::invalid_argument("Failed to get 'done' property"))?
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        let entry_value = Reflect::get(&next, &JsValue::from_str("value"))
            .map_err(|_| WasmDppError::invalid_argument("Failed to get 'value' property"))?;

        let entry_array = js_sys::Array::from(&entry_value);
        let key = entry_array.get(0);
        let value = entry_array.get(1);

        let identifier: Identifier = IdentifierWasm::try_from(key)
            .map_err(|e| {
                WasmDppError::invalid_argument(format!("Invalid block proposer identifier: {}", e))
            })?
            .into();

        let credits = try_to_u64(value).map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "block proposer value is not a valid u64: {:#}",
                err
            ))
        })?;

        map.insert(identifier, credits);
    }

    Ok(map)
}

fn block_proposers_to_map(map: &BTreeMap<Identifier, u64>) -> BlockProposersMapJs {
    let js_map = Map::new();

    for (identifier, value) in map {
        let identifier_wasm = IdentifierWasm::from(*identifier);
        js_map.set(&identifier_wasm.into(), &BigInt::from(*value).into());
    }

    js_map.into()
}

#[wasm_bindgen(js_class = FinalizedEpochInfo)]
impl FinalizedEpochInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: FinalizedEpochInfoOptionsJs,
    ) -> WasmDppResult<FinalizedEpochInfoWasm> {
        let options_obj = try_to_object(options.into(), "options")?;

        let first_block_time_js = get_required_property(&options_obj, "firstBlockTime")?;
        let first_block_time = try_to_u64(first_block_time_js)?;

        let first_block_height_js = get_required_property(&options_obj, "firstBlockHeight")?;
        let first_block_height = try_to_u64(first_block_height_js)?;

        let total_blocks_in_epoch_js = get_required_property(&options_obj, "totalBlocksInEpoch")?;
        let total_blocks_in_epoch = try_to_u64(total_blocks_in_epoch_js)?;

        let first_core_block_height_js =
            get_required_property(&options_obj, "firstCoreBlockHeight")?;
        let first_core_block_height = try_to_u32(first_core_block_height_js, "firstCoreBlockHeight")?;

        let next_epoch_start_core_block_height_js =
            get_required_property(&options_obj, "nextEpochStartCoreBlockHeight")?;
        let next_epoch_start_core_block_height =
            try_to_u32(next_epoch_start_core_block_height_js, "nextEpochStartCoreBlockHeight")?;

        let total_processing_fees_js = get_required_property(&options_obj, "totalProcessingFees")?;
        let total_processing_fees = try_to_u64(total_processing_fees_js)?;

        let total_distributed_storage_fees_js =
            get_required_property(&options_obj, "totalDistributedStorageFees")?;
        let total_distributed_storage_fees = try_to_u64(total_distributed_storage_fees_js)?;

        let total_created_storage_fees_js =
            get_required_property(&options_obj, "totalCreatedStorageFees")?;
        let total_created_storage_fees = try_to_u64(total_created_storage_fees_js)?;

        let core_block_rewards_js = get_required_property(&options_obj, "coreBlockRewards")?;
        let core_block_rewards = try_to_u64(core_block_rewards_js)?;

        let block_proposers_js = get_required_property(&options_obj, "blockProposers")?;
        if !block_proposers_js.is_instance_of::<Map>() {
            return Err(WasmDppError::invalid_argument(
                "'blockProposers' must be a Map",
            ));
        }
        let block_proposers = block_proposers_from_map(&Map::unchecked_from_js(block_proposers_js))?;

        let fee_multiplier_permille_js =
            get_required_property(&options_obj, "feeMultiplierPermille")?;
        let fee_multiplier_permille = try_to_u64(fee_multiplier_permille_js)?;

        let protocol_version_js = get_required_property(&options_obj, "protocolVersion")?;
        let protocol_version = try_to_u32(protocol_version_js, "protocolVersion")?;

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
    pub fn block_proposers(&self) -> BlockProposersMapJs {
        block_proposers_to_map(self.v0().block_proposers())
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
        block_proposers: BlockProposersMapJs,
    ) -> WasmDppResult<()> {
        let block_proposers_map =
            block_proposers_from_map(&Map::from(JsValue::from(block_proposers)))?;
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

crate::impl_wasm_conversions!(
    FinalizedEpochInfoWasm,
    FinalizedEpochInfo,
    FinalizedEpochInfoObjectJs,
    FinalizedEpochInfoJSONJs
);
impl_wasm_type_info!(FinalizedEpochInfoWasm, FinalizedEpochInfo);
