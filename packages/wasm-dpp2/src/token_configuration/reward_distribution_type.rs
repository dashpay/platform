use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType::{BlockBasedDistribution, EpochBasedDistribution, TimeBasedDistribution};
use dpp::prelude::{BlockHeightInterval, EpochInterval, TimestampMillisInterval};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::token_configuration::distribution_function::DistributionFunctionWASM;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "RewardDistributionType")]
pub struct RewardDistributionTypeWASM(RewardDistributionType);

impl From<RewardDistributionType> for RewardDistributionTypeWASM {
    fn from(reward_distribution_type: RewardDistributionType) -> Self {
        Self(reward_distribution_type)
    }
}

impl From<RewardDistributionTypeWASM> for RewardDistributionType {
    fn from(reward_distribution_type: RewardDistributionTypeWASM) -> Self {
        reward_distribution_type.0
    }
}

#[wasm_bindgen(js_class = RewardDistributionType)]
impl RewardDistributionTypeWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "RewardDistributionType".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "RewardDistributionType".to_string()
    }

    #[wasm_bindgen(js_name = "BlockBasedDistribution")]
    pub fn block_based_distribution(
        interval: BlockHeightInterval,
        function: &DistributionFunctionWASM,
    ) -> Self {
        RewardDistributionTypeWASM(BlockBasedDistribution {
            interval,
            function: function.clone().into(),
        })
    }

    #[wasm_bindgen(js_name = "TimeBasedDistribution")]
    pub fn time_based_distribution(
        interval: TimestampMillisInterval,
        function: &DistributionFunctionWASM,
    ) -> Self {
        RewardDistributionTypeWASM(TimeBasedDistribution {
            interval,
            function: function.clone().into(),
        })
    }

    #[wasm_bindgen(js_name = "EpochBasedDistribution")]
    pub fn epoch_based_distribution(
        interval: EpochInterval,
        function: &DistributionFunctionWASM,
    ) -> Self {
        RewardDistributionTypeWASM(EpochBasedDistribution {
            interval,
            function: function.clone().into(),
        })
    }

    #[wasm_bindgen(js_name = "getDistribution")]
    pub fn get_distribution(&self) -> JsValue {
        match self.0.clone() {
            RewardDistributionType::BlockBasedDistribution { interval, function } => {
                JsValue::from(BlockBasedDistributionWASM {
                    interval,
                    function: function.clone().into(),
                })
            }
            RewardDistributionType::TimeBasedDistribution { interval, function } => {
                JsValue::from(TimeBasedDistributionWASM {
                    interval,
                    function: function.clone().into(),
                })
            }
            RewardDistributionType::EpochBasedDistribution { interval, function } => {
                JsValue::from(EpochBasedDistributionWASM {
                    interval,
                    function: function.clone().into(),
                })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "BlockBasedDistribution")]
pub struct BlockBasedDistributionWASM {
    pub interval: BlockHeightInterval,
    function: DistributionFunctionWASM,
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TimeBasedDistribution")]
pub struct TimeBasedDistributionWASM {
    pub interval: TimestampMillisInterval,
    function: DistributionFunctionWASM,
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "EpochBasedDistribution")]
pub struct EpochBasedDistributionWASM {
    pub interval: EpochInterval,
    function: DistributionFunctionWASM,
}

#[wasm_bindgen(js_class = BlockBasedDistribution)]
impl BlockBasedDistributionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "BlockBasedDistribution".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "BlockBasedDistribution".to_string()
    }

    #[wasm_bindgen(getter = "function")]
    pub fn get_function(&self) -> DistributionFunctionWASM {
        self.function.clone()
    }

    #[wasm_bindgen(setter = "function")]
    pub fn set_function(&mut self, function: &DistributionFunctionWASM) {
        self.function = function.clone()
    }
}

#[wasm_bindgen(js_class = TimeBasedDistribution)]
impl TimeBasedDistributionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TimeBasedDistribution".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TimeBasedDistribution".to_string()
    }

    #[wasm_bindgen(getter = "function")]
    pub fn get_function(&self) -> DistributionFunctionWASM {
        self.function.clone()
    }

    #[wasm_bindgen(setter = "function")]
    pub fn set_function(&mut self, function: &DistributionFunctionWASM) {
        self.function = function.clone()
    }
}

#[wasm_bindgen(js_class = EpochBasedDistribution)]
impl EpochBasedDistributionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "EpochBasedDistribution".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "EpochBasedDistribution".to_string()
    }

    #[wasm_bindgen(getter = "function")]
    pub fn get_function(&self) -> DistributionFunctionWASM {
        self.function.clone()
    }

    #[wasm_bindgen(setter = "function")]
    pub fn set_function(&mut self, function: &DistributionFunctionWASM) {
        self.function = function.clone()
    }
}
