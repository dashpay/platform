use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeOrUndefinedJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::tokens::configuration::change_control_rules::ChangeControlRulesWasm;
use crate::tokens::configuration::perpetual_distribution::TokenPerpetualDistributionWasm;
use crate::tokens::configuration::pre_programmed_distribution::TokenPreProgrammedDistributionWasm;
use crate::utils::IntoWasm;
use dpp::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::{
    TokenDistributionRulesV0Getters, TokenDistributionRulesV0Setters,
};
use dpp::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_DISTRIBUTION_RULES_OPTIONS_TS: &'static str = r#"
export interface TokenDistributionRulesOptions {
    perpetualDistribution?: TokenPerpetualDistribution;
    perpetualDistributionRules: ChangeControlRules;
    preProgrammedDistribution?: TokenPreProgrammedDistribution;
    newTokensDestinationIdentity?: IdentifierLike;
    newTokensDestinationIdentityRules: ChangeControlRules;
    mintingAllowChoosingDestination: boolean;
    mintingAllowChoosingDestinationRules: ChangeControlRules;
    changeDirectPurchasePricingRules: ChangeControlRules;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenDistributionRulesOptions")]
    pub type TokenDistributionRulesOptionsJs;
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenDistributionRules")]
pub struct TokenDistributionRulesWasm(TokenDistributionRules);

impl From<TokenDistributionRulesWasm> for TokenDistributionRules {
    fn from(rules: TokenDistributionRulesWasm) -> Self {
        rules.0
    }
}

impl From<TokenDistributionRules> for TokenDistributionRulesWasm {
    fn from(rules: TokenDistributionRules) -> Self {
        Self(rules)
    }
}

#[wasm_bindgen(js_class = TokenDistributionRules)]
impl TokenDistributionRulesWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenDistributionRulesOptionsJs,
    ) -> WasmDppResult<TokenDistributionRulesWasm> {
        let options_obj = Object::from(JsValue::from(options));

        let js_perpetual_distribution = Reflect::get(&options_obj, &"perpetualDistribution".into())
            .unwrap_or(JsValue::UNDEFINED);
        let perpetual_distribution = if js_perpetual_distribution.is_undefined() {
            None
        } else {
            Some(
                js_perpetual_distribution
                    .to_wasm::<TokenPerpetualDistributionWasm>("TokenPerpetualDistribution")?
                    .clone()
                    .into(),
            )
        };

        let perpetual_distribution_rules =
            Reflect::get(&options_obj, &"perpetualDistributionRules".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument("perpetualDistributionRules is required")
                })?
                .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
                .clone();

        let js_pre_programmed_distribution =
            Reflect::get(&options_obj, &"preProgrammedDistribution".into())
                .unwrap_or(JsValue::UNDEFINED);
        let pre_programmed_distribution = if js_pre_programmed_distribution.is_undefined() {
            None
        } else {
            Some(
                js_pre_programmed_distribution
                    .to_wasm::<TokenPreProgrammedDistributionWasm>(
                        "TokenPreProgrammedDistribution",
                    )?
                    .clone()
                    .into(),
            )
        };

        let js_new_tokens_destination_identity =
            Reflect::get(&options_obj, &"newTokensDestinationIdentity".into())
                .unwrap_or(JsValue::UNDEFINED);
        let new_tokens_destination_identity = if js_new_tokens_destination_identity.is_undefined() {
            None
        } else {
            Some(IdentifierWasm::try_from(&js_new_tokens_destination_identity)?.into())
        };

        let new_tokens_destination_identity_rules =
            Reflect::get(&options_obj, &"newTokensDestinationIdentityRules".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument("newTokensDestinationIdentityRules is required")
                })?
                .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
                .clone();

        let minting_allow_choosing_destination =
            Reflect::get(&options_obj, &"mintingAllowChoosingDestination".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument("mintingAllowChoosingDestination is required")
                })?
                .as_bool()
                .ok_or_else(|| {
                    WasmDppError::invalid_argument(
                        "mintingAllowChoosingDestination must be a boolean",
                    )
                })?;

        let minting_allow_choosing_destination_rules =
            Reflect::get(&options_obj, &"mintingAllowChoosingDestinationRules".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument(
                        "mintingAllowChoosingDestinationRules is required",
                    )
                })?
                .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
                .clone();

        let change_direct_purchase_pricing_rules =
            Reflect::get(&options_obj, &"changeDirectPurchasePricingRules".into())
                .map_err(|_| {
                    WasmDppError::invalid_argument("changeDirectPurchasePricingRules is required")
                })?
                .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
                .clone();

        Ok(TokenDistributionRulesWasm(TokenDistributionRules::V0(
            TokenDistributionRulesV0 {
                perpetual_distribution,
                perpetual_distribution_rules: perpetual_distribution_rules.into(),
                pre_programmed_distribution,
                new_tokens_destination_identity,
                new_tokens_destination_identity_rules: new_tokens_destination_identity_rules.into(),
                minting_allow_choosing_destination,
                minting_allow_choosing_destination_rules: minting_allow_choosing_destination_rules
                    .into(),
                change_direct_purchase_pricing_rules: change_direct_purchase_pricing_rules.into(),
            },
        )))
    }

    #[wasm_bindgen(getter = "perpetualDistribution")]
    pub fn get_perpetual_distribution(&self) -> Option<TokenPerpetualDistributionWasm> {
        self.0
            .perpetual_distribution()
            .map(|perp| perp.clone().into())
    }

    #[wasm_bindgen(getter = "perpetualDistributionRules")]
    pub fn get_perpetual_distribution_rules(&self) -> ChangeControlRulesWasm {
        self.0.perpetual_distribution_rules().clone().into()
    }

    #[wasm_bindgen(getter = "preProgrammedDistribution")]
    pub fn get_pre_programmed_distribution(&self) -> Option<TokenPreProgrammedDistributionWasm> {
        self.0
            .pre_programmed_distribution()
            .map(|pre| pre.clone().into())
    }

    #[wasm_bindgen(getter = "newTokenDestinationIdentity")]
    pub fn get_new_tokens_destination_identity(&self) -> Option<IdentifierWasm> {
        self.0
            .new_tokens_destination_identity()
            .map(|id| IdentifierWasm::from(*id))
    }

    #[wasm_bindgen(getter = "newTokenDestinationIdentityRules")]
    pub fn get_new_tokens_destination_identity_rules(&self) -> ChangeControlRulesWasm {
        self.0
            .new_tokens_destination_identity_rules()
            .clone()
            .into()
    }

    #[wasm_bindgen(getter = "isMintingAllowingChoosingDestination")]
    pub fn is_minting_allowing_choosing_destination(&self) -> bool {
        self.0.minting_allow_choosing_destination()
    }

    #[wasm_bindgen(getter = "mintingAllowChoosingDestinationRules")]
    pub fn get_minting_allow_choosing_destination_rules(&self) -> ChangeControlRulesWasm {
        self.0
            .minting_allow_choosing_destination_rules()
            .clone()
            .into()
    }

    #[wasm_bindgen(getter = "changeDirectPurchasePricingRules")]
    pub fn get_change_direct_purchase_pricing_rules(&self) -> ChangeControlRulesWasm {
        self.0.change_direct_purchase_pricing_rules().clone().into()
    }

    #[wasm_bindgen(setter = "perpetualDistribution")]
    pub fn set_perpetual_distribution(
        &mut self,
        perpetual_distribution: &JsValue,
    ) -> WasmDppResult<()> {
        let perpetual_distribution = match perpetual_distribution.is_undefined() {
            true => None,
            false => Some(
                perpetual_distribution
                    .to_wasm::<TokenPerpetualDistributionWasm>("TokenPerpetualDistribution")?
                    .clone()
                    .into(),
            ),
        };

        self.0.set_perpetual_distribution(perpetual_distribution);
        Ok(())
    }

    #[wasm_bindgen(setter = "perpetualDistributionRules")]
    pub fn set_perpetual_distribution_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0
            .set_perpetual_distribution_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "preProgrammedDistribution")]
    pub fn set_pre_programmed_distribution(&mut self, distribution: &JsValue) -> WasmDppResult<()> {
        let distribution = match distribution.is_undefined() {
            true => None,
            false => Some(
                distribution
                    .to_wasm::<TokenPreProgrammedDistributionWasm>(
                        "TokenPreProgrammedDistribution",
                    )?
                    .clone()
                    .into(),
            ),
        };

        self.0.set_pre_programmed_distribution(distribution);
        Ok(())
    }

    #[wasm_bindgen(setter = "newTokenDestinationIdentity")]
    pub fn set_new_tokens_destination_identity(
        &mut self,
        identifier: IdentifierLikeOrUndefinedJs,
    ) -> WasmDppResult<()> {
        let identifier: Option<dpp::prelude::Identifier> = identifier.try_into()?;
        self.0.set_new_tokens_destination_identity(identifier);
        Ok(())
    }

    #[wasm_bindgen(setter = "newTokenDestinationIdentityRules")]
    pub fn set_new_tokens_destination_identity_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0
            .set_new_tokens_destination_identity_rules(rules.clone().into());
    }

    #[wasm_bindgen(setter = "isMintingAllowingChoosingDestination")]
    pub fn set_is_minting_allowing_choosing_destination(
        &mut self,
        is_minting_allowing_choosing_destination: bool,
    ) {
        self.0
            .set_minting_allow_choosing_destination(is_minting_allowing_choosing_destination);
    }

    #[wasm_bindgen(setter = "mintingAllowChoosingDestinationRules")]
    pub fn set_minting_allow_choosing_destination_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0
            .set_minting_allow_choosing_destination_rules(rules.clone().into());
    }

    #[wasm_bindgen(setter = "changeDirectPurchasePricingRules")]
    pub fn set_change_direct_purchase_pricing_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0
            .set_change_direct_purchase_pricing_rules(rules.clone().into());
    }
}

impl_wasm_type_info!(TokenDistributionRulesWasm, TokenDistributionRules);
