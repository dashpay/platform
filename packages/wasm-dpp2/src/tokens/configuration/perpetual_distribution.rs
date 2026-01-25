use crate::impl_wasm_type_info;
use crate::tokens::configuration::distribution_recipient::TokenDistributionRecipientWasm;
use crate::tokens::configuration::reward_distribution_type::RewardDistributionTypeWasm;
use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenPerpetualDistribution")]
pub struct TokenPerpetualDistributionWasm(TokenPerpetualDistribution);

impl From<TokenPerpetualDistributionWasm> for TokenPerpetualDistribution {
    fn from(value: TokenPerpetualDistributionWasm) -> Self {
        value.0
    }
}

impl From<TokenPerpetualDistribution> for TokenPerpetualDistributionWasm {
    fn from(value: TokenPerpetualDistribution) -> Self {
        TokenPerpetualDistributionWasm(value)
    }
}

#[wasm_bindgen(js_class = TokenPerpetualDistribution)]
impl TokenPerpetualDistributionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "distributionType")]
        distribution_type: &RewardDistributionTypeWasm,
        recipient: &TokenDistributionRecipientWasm,
    ) -> Self {
        TokenPerpetualDistributionWasm(TokenPerpetualDistribution::V0(
            TokenPerpetualDistributionV0 {
                distribution_type: distribution_type.clone().into(),
                distribution_recipient: recipient.clone().into(),
            },
        ))
    }

    #[wasm_bindgen(getter = distributionType)]
    pub fn distribution_type(&self) -> RewardDistributionTypeWasm {
        self.0.distribution_type().clone().into()
    }

    #[wasm_bindgen(getter = distributionRecipient)]
    pub fn recipient(&self) -> TokenDistributionRecipientWasm {
        self.0.distribution_recipient().into()
    }

    #[wasm_bindgen(setter = distributionType)]
    pub fn set_distribution_type(
        &mut self,
        #[wasm_bindgen(js_name = "distributionType")]
        distribution_type: &RewardDistributionTypeWasm,
    ) {
        self.0
            .set_distribution_type(distribution_type.clone().into());
    }

    #[wasm_bindgen(setter = distributionRecipient)]
    pub fn set_recipient(
        &mut self,
        #[wasm_bindgen(js_name = "distributionRecipient")]
        distribution_recipient: &TokenDistributionRecipientWasm,
    ) {
        self.0
            .set_distribution_recipient(distribution_recipient.clone().into());
    }
}

impl_wasm_type_info!(TokenPerpetualDistributionWasm, TokenPerpetualDistribution);
