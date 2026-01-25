use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_from_for_extern_type;
use crate::impl_wasm_type_info;
use crate::utils::{
    JsMapExt, try_from_options_with, try_to_map, try_to_object, try_to_u32, try_to_u64,
};
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use dpp::prelude::Identifier;
use js_sys::{BigInt, Map};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

#[wasm_bindgen(typescript_custom_section)]
const FINALIZED_EPOCH_INFO_OPTIONS_TS: &str = r#"
/**
 * Block proposers mapping: base58 Identifier string -> block count (bigint).
 */
export type BlockProposersMap = Map<string, bigint>;

export interface FinalizedEpochInfoOptions {
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

    for entry in js_map.entries().into_iter() {
        let entry = entry.map_err(|e| {
            WasmDppError::invalid_argument(format!("Failed to iterate map entries: {:?}", e))
        })?;

        let entry_array = js_sys::Array::from(&entry);
        let key = entry_array.get(0);
        let value = entry_array.get(1);

        let identifier: Identifier = IdentifierWasm::try_from(key)
            .map_err(|e| {
                WasmDppError::invalid_argument(format!("Invalid block proposer identifier: {}", e))
            })?
            .into();

        let credits = try_to_u64(value, "blockProposerCredits")?;

        map.insert(identifier, credits);
    }

    Ok(map)
}

fn block_proposers_to_map(map: &BTreeMap<Identifier, u64>) -> BlockProposersMapJs {
    Map::from_entries(map.iter().map(|(identifier, value)| {
        let key: JsValue = IdentifierWasm::from(*identifier).to_base58().into();
        let value: JsValue = BigInt::from(*value).into();
        (key, value)
    }))
    .into()
}

#[wasm_bindgen(js_class = FinalizedEpochInfo)]
impl FinalizedEpochInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: FinalizedEpochInfoOptionsJs,
    ) -> WasmDppResult<FinalizedEpochInfoWasm> {
        let options_obj = try_to_object(options.into(), "options")?;

        let first_block_time = try_from_options_with(&options_obj, "firstBlockTime", |v| {
            try_to_u64(v, "firstBlockTime")
        })?;

        let first_block_height = try_from_options_with(&options_obj, "firstBlockHeight", |v| {
            try_to_u64(v, "firstBlockHeight")
        })?;

        let total_blocks_in_epoch =
            try_from_options_with(&options_obj, "totalBlocksInEpoch", |v| {
                try_to_u64(v, "totalBlocksInEpoch")
            })?;

        let first_core_block_height =
            try_from_options_with(&options_obj, "firstCoreBlockHeight", |v| {
                try_to_u32(v, "firstCoreBlockHeight")
            })?;

        let next_epoch_start_core_block_height =
            try_from_options_with(&options_obj, "nextEpochStartCoreBlockHeight", |v| {
                try_to_u32(v, "nextEpochStartCoreBlockHeight")
            })?;

        let total_processing_fees =
            try_from_options_with(&options_obj, "totalProcessingFees", |v| {
                try_to_u64(v, "totalProcessingFees")
            })?;

        let total_distributed_storage_fees =
            try_from_options_with(&options_obj, "totalDistributedStorageFees", |v| {
                try_to_u64(v, "totalDistributedStorageFees")
            })?;

        let total_created_storage_fees =
            try_from_options_with(&options_obj, "totalCreatedStorageFees", |v| {
                try_to_u64(v, "totalCreatedStorageFees")
            })?;

        let core_block_rewards = try_from_options_with(&options_obj, "coreBlockRewards", |v| {
            try_to_u64(v, "coreBlockRewards")
        })?;

        let block_proposers = try_from_options_with(&options_obj, "blockProposers", |v| {
            if !v.is_instance_of::<Map>() {
                return Err(WasmDppError::invalid_argument(
                    "'blockProposers' must be a Map",
                ));
            }
            block_proposers_from_map(&Map::unchecked_from_js(v))
        })?;

        let fee_multiplier_permille =
            try_from_options_with(&options_obj, "feeMultiplierPermille", |v| {
                try_to_u64(v, "feeMultiplierPermille")
            })?;

        let protocol_version = try_from_options_with(&options_obj, "protocolVersion", |v| {
            try_to_u32(v, "protocolVersion")
        })?;

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
    pub fn set_first_block_time(
        &mut self,
        #[wasm_bindgen(js_name = "firstBlockTime")] first_block_time: u64,
    ) {
        self.v0_mut().first_block_time = first_block_time;
    }

    #[wasm_bindgen(setter = "firstBlockHeight")]
    pub fn set_first_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "firstBlockHeight")] first_block_height: u64,
    ) {
        self.v0_mut().first_block_height = first_block_height;
    }

    #[wasm_bindgen(setter = "totalBlocksInEpoch")]
    pub fn set_total_blocks_in_epoch(
        &mut self,
        #[wasm_bindgen(js_name = "totalBlocksInEpoch")] total_blocks_in_epoch: u64,
    ) {
        self.v0_mut().total_blocks_in_epoch = total_blocks_in_epoch;
    }

    #[wasm_bindgen(setter = "firstCoreBlockHeight")]
    pub fn set_first_core_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "firstCoreBlockHeight")] first_core_block_height: u32,
    ) {
        self.v0_mut().first_core_block_height = first_core_block_height;
    }

    #[wasm_bindgen(setter = "nextEpochStartCoreBlockHeight")]
    pub fn set_next_epoch_start_core_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "nextEpochStartCoreBlockHeight")]
        next_epoch_start_core_block_height: u32,
    ) {
        self.v0_mut().next_epoch_start_core_block_height = next_epoch_start_core_block_height;
    }

    #[wasm_bindgen(setter = "totalProcessingFees")]
    pub fn set_total_processing_fees(
        &mut self,
        #[wasm_bindgen(js_name = "totalProcessingFees")] total_processing_fees: u64,
    ) {
        self.v0_mut().total_processing_fees = total_processing_fees;
    }

    #[wasm_bindgen(setter = "totalDistributedStorageFees")]
    pub fn set_total_distributed_storage_fees(
        &mut self,
        #[wasm_bindgen(js_name = "totalDistributedStorageFees")]
        total_distributed_storage_fees: u64,
    ) {
        self.v0_mut().total_distributed_storage_fees = total_distributed_storage_fees;
    }

    #[wasm_bindgen(setter = "totalCreatedStorageFees")]
    pub fn set_total_created_storage_fees(
        &mut self,
        #[wasm_bindgen(js_name = "totalCreatedStorageFees")] total_created_storage_fees: u64,
    ) {
        self.v0_mut().total_created_storage_fees = total_created_storage_fees;
    }

    #[wasm_bindgen(setter = "coreBlockRewards")]
    pub fn set_core_block_rewards(
        &mut self,
        #[wasm_bindgen(js_name = "coreBlockRewards")] core_block_rewards: u64,
    ) {
        self.v0_mut().core_block_rewards = core_block_rewards;
    }

    #[wasm_bindgen(setter = "blockProposers")]
    pub fn set_block_proposers(
        &mut self,
        #[wasm_bindgen(js_name = "blockProposers")] block_proposers: BlockProposersMapJs,
    ) -> WasmDppResult<()> {
        let block_proposers_map =
            block_proposers_from_map(&try_to_map(block_proposers.into(), "blockProposers")?)?;
        self.v0_mut().block_proposers = block_proposers_map;
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

crate::impl_wasm_conversions!(
    FinalizedEpochInfoWasm,
    FinalizedEpochInfo,
    FinalizedEpochInfoObjectJs,
    FinalizedEpochInfoJSONJs
);
impl_wasm_type_info!(FinalizedEpochInfoWasm, FinalizedEpochInfo);
