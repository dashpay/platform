use crate::token_configuration::distribution_recipient::TokenDistributionRecipientWASM;
use crate::token_configuration::reward_distribution_type::RewardDistributionTypeWASM;
use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenPerpetualDistribution")]
pub struct TokenPerpetualDistributionWASM(TokenPerpetualDistribution);

impl From<TokenPerpetualDistributionWASM> for TokenPerpetualDistribution {
    fn from(value: TokenPerpetualDistributionWASM) -> Self {
        value.0
    }
}

impl From<TokenPerpetualDistribution> for TokenPerpetualDistributionWASM {
    fn from(value: TokenPerpetualDistribution) -> Self {
        TokenPerpetualDistributionWASM(value)
    }
}

#[wasm_bindgen(js_class = TokenPerpetualDistribution)]
impl TokenPerpetualDistributionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenPerpetualDistribution".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenPerpetualDistribution".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        distribution_type: &RewardDistributionTypeWASM,
        recipient: &TokenDistributionRecipientWASM,
    ) -> Self {
        TokenPerpetualDistributionWASM(TokenPerpetualDistribution::V0(
            TokenPerpetualDistributionV0 {
                distribution_type: distribution_type.clone().into(),
                distribution_recipient: recipient.clone().into(),
            },
        ))
    }

    #[wasm_bindgen(getter = distributionType)]
    pub fn distribution_type(&self) -> RewardDistributionTypeWASM {
        self.0.distribution_type().clone().into()
    }

    #[wasm_bindgen(getter = distributionRecipient)]
    pub fn recipient(&self) -> TokenDistributionRecipientWASM {
        self.0.distribution_recipient().clone().into()
    }

    #[wasm_bindgen(setter = distributionType)]
    pub fn set_distribution_type(&mut self, distribution_type: &RewardDistributionTypeWASM) {
        self.0
            .set_distribution_type(distribution_type.clone().into());
    }

    #[wasm_bindgen(setter = distributionRecipient)]
    pub fn set_recipient(&mut self, distribution_recipient: &TokenDistributionRecipientWASM) {
        self.0
            .set_distribution_recipient(distribution_recipient.clone().into());
    }
}
