use crate::impl_wasm_type_info;
use crate::tokens::configuration::distribution_function::DistributionFunctionWasm;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType::{BlockBasedDistribution, EpochBasedDistribution, TimeBasedDistribution};
use dpp::prelude::{BlockHeightInterval, EpochInterval, TimestampMillisInterval};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const REWARD_DISTRIBUTION_TS: &'static str = r#"
/**
 * Union type for reward distribution variants.
 */
export type RewardDistributionValue =
    | BlockBasedDistribution
    | TimeBasedDistribution
    | EpochBasedDistribution;
"#;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "RewardDistributionType")]
pub struct RewardDistributionTypeWasm(RewardDistributionType);

impl From<RewardDistributionType> for RewardDistributionTypeWasm {
    fn from(reward_distribution_type: RewardDistributionType) -> Self {
        Self(reward_distribution_type)
    }
}

impl From<RewardDistributionTypeWasm> for RewardDistributionType {
    fn from(reward_distribution_type: RewardDistributionTypeWasm) -> Self {
        reward_distribution_type.0
    }
}

#[wasm_bindgen(js_class = RewardDistributionType)]
impl RewardDistributionTypeWasm {
    #[wasm_bindgen(js_name = "BlockBasedDistribution")]
    pub fn block_based_distribution(
        interval: BlockHeightInterval,
        function: &DistributionFunctionWasm,
    ) -> Self {
        RewardDistributionTypeWasm(BlockBasedDistribution {
            interval,
            function: function.clone().into(),
        })
    }

    #[wasm_bindgen(js_name = "TimeBasedDistribution")]
    pub fn time_based_distribution(
        interval: TimestampMillisInterval,
        function: &DistributionFunctionWasm,
    ) -> Self {
        RewardDistributionTypeWasm(TimeBasedDistribution {
            interval,
            function: function.clone().into(),
        })
    }

    #[wasm_bindgen(js_name = "EpochBasedDistribution")]
    pub fn epoch_based_distribution(
        interval: EpochInterval,
        function: &DistributionFunctionWasm,
    ) -> Self {
        RewardDistributionTypeWasm(EpochBasedDistribution {
            interval,
            function: function.clone().into(),
        })
    }

    #[wasm_bindgen(getter = "distribution")]
    pub fn distribution(&self) -> JsValue {
        match self.0.clone() {
            RewardDistributionType::BlockBasedDistribution { interval, function } => {
                JsValue::from(BlockBasedDistributionWasm {
                    interval,
                    function: function.clone().into(),
                })
            }
            RewardDistributionType::TimeBasedDistribution { interval, function } => {
                JsValue::from(TimeBasedDistributionWasm {
                    interval,
                    function: function.clone().into(),
                })
            }
            RewardDistributionType::EpochBasedDistribution { interval, function } => {
                JsValue::from(EpochBasedDistributionWasm {
                    interval,
                    function: function.clone().into(),
                })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "BlockBasedDistribution")]
pub struct BlockBasedDistributionWasm {
    pub interval: BlockHeightInterval,
    function: DistributionFunctionWasm,
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TimeBasedDistribution")]
pub struct TimeBasedDistributionWasm {
    pub interval: TimestampMillisInterval,
    function: DistributionFunctionWasm,
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "EpochBasedDistribution")]
pub struct EpochBasedDistributionWasm {
    pub interval: EpochInterval,
    function: DistributionFunctionWasm,
}

#[wasm_bindgen(js_class = BlockBasedDistribution)]
impl BlockBasedDistributionWasm {
    #[wasm_bindgen(getter = "function")]
    pub fn get_function(&self) -> DistributionFunctionWasm {
        self.function.clone()
    }

    #[wasm_bindgen(setter = "function")]
    pub fn set_function(&mut self, function: &DistributionFunctionWasm) {
        self.function = function.clone()
    }
}

#[wasm_bindgen(js_class = TimeBasedDistribution)]
impl TimeBasedDistributionWasm {
    #[wasm_bindgen(getter = "function")]
    pub fn get_function(&self) -> DistributionFunctionWasm {
        self.function.clone()
    }

    #[wasm_bindgen(setter = "function")]
    pub fn set_function(&mut self, function: &DistributionFunctionWasm) {
        self.function = function.clone()
    }
}

#[wasm_bindgen(js_class = EpochBasedDistribution)]
impl EpochBasedDistributionWasm {
    #[wasm_bindgen(getter = "function")]
    pub fn get_function(&self) -> DistributionFunctionWasm {
        self.function.clone()
    }

    #[wasm_bindgen(setter = "function")]
    pub fn set_function(&mut self, function: &DistributionFunctionWasm) {
        self.function = function.clone()
    }
}

impl_wasm_type_info!(RewardDistributionTypeWasm, RewardDistributionType);
impl_wasm_type_info!(BlockBasedDistributionWasm, BlockBasedDistribution);
impl_wasm_type_info!(TimeBasedDistributionWasm, TimeBasedDistribution);
impl_wasm_type_info!(EpochBasedDistributionWasm, EpochBasedDistribution);
