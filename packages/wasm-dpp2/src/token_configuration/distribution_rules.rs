use crate::identifier::IdentifierWASM;
use crate::token_configuration::change_control_rules::ChangeControlRulesWASM;
use crate::token_configuration::perpetual_distribution::TokenPerpetualDistributionWASM;
use crate::token_configuration::pre_programmed_distribution::TokenPreProgrammedDistributionWASM;
use crate::utils::IntoWasm;
use dpp::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::{
    TokenDistributionRulesV0Getters, TokenDistributionRulesV0Setters,
};
use dpp::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::prelude::Identifier;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenDistributionRulesWASM")]
pub struct TokenDistributionRulesWASM(TokenDistributionRules);

impl From<TokenDistributionRulesWASM> for TokenDistributionRules {
    fn from(rules: TokenDistributionRulesWASM) -> Self {
        rules.0
    }
}

impl From<TokenDistributionRules> for TokenDistributionRulesWASM {
    fn from(rules: TokenDistributionRules) -> Self {
        Self(rules)
    }
}

#[wasm_bindgen]
impl TokenDistributionRulesWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenDistributionRulesWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenDistributionRulesWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_perpetual_distribution: &JsValue,
        perpetual_distribution_rules: &ChangeControlRulesWASM,
        js_pre_programmed_distribution: &JsValue,
        js_new_tokens_destination_identity: &JsValue,
        new_tokens_destination_identity_rules: &ChangeControlRulesWASM,
        minting_allow_choosing_destination: bool,
        minting_allow_choosing_destination_rules: &ChangeControlRulesWASM,
        change_direct_purchase_pricing_rules: &ChangeControlRulesWASM,
    ) -> Result<TokenDistributionRulesWASM, JsValue> {
        let perpetual_distribution = match js_perpetual_distribution.is_undefined() {
            true => None,
            false => Some(TokenPerpetualDistribution::from(
                js_perpetual_distribution
                    .to_wasm::<TokenPerpetualDistributionWASM>("TokenPerpetualDistributionWASM")?
                    .clone(),
            )),
        };

        let pre_programmed_distribution = match js_pre_programmed_distribution.is_undefined() {
            true => None,
            false => Some(TokenPreProgrammedDistribution::from(
                js_pre_programmed_distribution
                    .to_wasm::<TokenPreProgrammedDistributionWASM>(
                        "TokenPreProgrammedDistributionWASM",
                    )?
                    .clone(),
            )),
        };

        let new_tokens_destination_identity =
            match js_new_tokens_destination_identity.is_undefined() {
                true => None,
                false => Some(Identifier::from(IdentifierWASM::try_from(
                    js_new_tokens_destination_identity,
                )?)),
            };

        Ok(TokenDistributionRulesWASM(TokenDistributionRules::V0(
            TokenDistributionRulesV0 {
                perpetual_distribution,
                perpetual_distribution_rules: perpetual_distribution_rules.clone().into(),
                pre_programmed_distribution,
                new_tokens_destination_identity,
                new_tokens_destination_identity_rules: new_tokens_destination_identity_rules
                    .clone()
                    .into(),
                minting_allow_choosing_destination,
                minting_allow_choosing_destination_rules: minting_allow_choosing_destination_rules
                    .clone()
                    .into(),
                change_direct_purchase_pricing_rules: change_direct_purchase_pricing_rules
                    .clone()
                    .into(),
            },
        )))
    }

    #[wasm_bindgen(getter = "perpetualDistribution")]
    pub fn get_perpetual_distribution(&self) -> Option<TokenPerpetualDistributionWASM> {
        match self.0.perpetual_distribution() {
            Some(perp) => Some(perp.clone().into()),
            None => None,
        }
    }

    #[wasm_bindgen(getter = "perpetualDistributionRules")]
    pub fn get_perpetual_distribution_rules(&self) -> ChangeControlRulesWASM {
        self.0.perpetual_distribution_rules().clone().into()
    }

    #[wasm_bindgen(getter = "preProgrammedDistribution")]
    pub fn get_pre_programmed_distribution(&self) -> Option<TokenPreProgrammedDistributionWASM> {
        match self.0.pre_programmed_distribution() {
            Some(pre) => Some(pre.clone().into()),
            None => None,
        }
    }

    #[wasm_bindgen(getter = "newTokenDestinationIdentity")]
    pub fn get_new_tokens_destination_identity(&self) -> Option<IdentifierWASM> {
        match self.0.new_tokens_destination_identity().clone() {
            Some(id) => Some(id.clone().into()),
            None => None,
        }
    }

    #[wasm_bindgen(getter = "newTokenDestinationIdentityRules")]
    pub fn get_new_tokens_destination_identity_rules(&self) -> ChangeControlRulesWASM {
        self.0
            .new_tokens_destination_identity_rules()
            .clone()
            .into()
    }

    #[wasm_bindgen(getter = "mintingAllowChoosingDestination")]
    pub fn get_minting_allow_choosing_destination(&self) -> bool {
        self.0.minting_allow_choosing_destination()
    }

    #[wasm_bindgen(getter = "mintingAllowChoosingDestinationRules")]
    pub fn get_minting_allow_choosing_destination_rules(&self) -> ChangeControlRulesWASM {
        self.0
            .minting_allow_choosing_destination_rules()
            .clone()
            .into()
    }

    #[wasm_bindgen(getter = "changeDirectPurchasePricingRules")]
    pub fn get_change_direct_purchase_pricing_rules(&self) -> ChangeControlRulesWASM {
        self.0.change_direct_purchase_pricing_rules().clone().into()
    }

    #[wasm_bindgen(setter = "perpetualDistribution")]
    pub fn set_perpetual_distribution(
        &mut self,
        js_perpetual_distribution: &JsValue,
    ) -> Result<(), JsValue> {
        let perpetual_distribution = match js_perpetual_distribution.is_undefined() {
            true => None,
            false => Some(TokenPerpetualDistribution::from(
                js_perpetual_distribution
                    .to_wasm::<TokenPerpetualDistributionWASM>("TokenPerpetualDistributionWASM")?
                    .clone(),
            )),
        };

        Ok(self.0.set_perpetual_distribution(perpetual_distribution))
    }

    #[wasm_bindgen(setter = "perpetualDistributionRules")]
    pub fn set_perpetual_distribution_rules(&mut self, rules: &ChangeControlRulesWASM) {
        self.0
            .set_perpetual_distribution_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "preProgrammedDistribution")]
    pub fn set_pre_programmed_distribution(
        &mut self,
        js_distribution: &JsValue,
    ) -> Result<(), JsValue> {
        let distribution = match js_distribution.is_undefined() {
            true => None,
            false => Some(TokenPreProgrammedDistribution::from(
                js_distribution
                    .to_wasm::<TokenPreProgrammedDistributionWASM>(
                        "TokenPreProgrammedDistributionWASM",
                    )?
                    .clone(),
            )),
        };

        Ok(self.0.set_pre_programmed_distribution(distribution))
    }

    #[wasm_bindgen(setter = "newTokenDestinationIdentity")]
    pub fn set_new_tokens_destination_identity(
        &mut self,
        js_identifier: &JsValue,
    ) -> Result<(), JsValue> {
        let identifier = match js_identifier.is_undefined() {
            true => None,
            false => Some(Identifier::from(
                IdentifierWASM::try_from(js_identifier)?.clone(),
            )),
        };

        Ok(self.0.set_new_tokens_destination_identity(identifier))
    }

    #[wasm_bindgen(setter = "newTokenDestinationIdentityRules")]
    pub fn set_new_tokens_destination_identity_rules(&mut self, rules: &ChangeControlRulesWASM) {
        self.0
            .set_new_tokens_destination_identity_rules(rules.clone().into());
    }

    #[wasm_bindgen(setter = "mintingAllowChoosingDestination")]
    pub fn set_minting_allow_choosing_destination(&mut self, flag: bool) {
        self.0.set_minting_allow_choosing_destination(flag);
    }

    #[wasm_bindgen(setter = "mintingAllowChoosingDestinationRules")]
    pub fn set_minting_allow_choosing_destination_rules(&mut self, rules: &ChangeControlRulesWASM) {
        self.0
            .set_minting_allow_choosing_destination_rules(rules.clone().into());
    }

    #[wasm_bindgen(setter = "changeDirectPurchasePricingRules")]
    pub fn set_change_direct_purchase_pricing_rules(&mut self, rules: &ChangeControlRulesWASM) {
        self.0
            .set_change_direct_purchase_pricing_rules(rules.clone().into());
    }
}
