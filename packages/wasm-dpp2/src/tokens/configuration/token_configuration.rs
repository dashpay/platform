use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::change_control_rules::ChangeControlRulesWasm;
use crate::tokens::configuration::configuration_convention::TokenConfigurationConventionWasm;
use crate::tokens::configuration::distribution_rules::TokenDistributionRulesWasm;
use crate::tokens::configuration::keeps_history_rules::TokenKeepsHistoryRulesWasm;
use crate::tokens::configuration::marketplace_rules::TokenMarketplaceRulesWasm;
use crate::utils::{try_from_options, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use dpp::data_contract::{GroupContractPosition, TokenConfiguration, TokenContractPosition};
use dpp::prelude::Identifier;
use dpp::tokens::calculate_token_id;
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenConfigurationOptions {
    base_supply: TokenAmount,
    #[serde(default)]
    max_supply: Option<TokenAmount>,
    #[serde(default)]
    is_started_as_paused: bool,
    #[serde(default)]
    is_allowed_transfer_to_frozen_balance: bool,
    #[serde(default)]
    main_control_group: Option<GroupContractPosition>,
    #[serde(default)]
    description: Option<String>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface TokenConfigurationOptions {
    conventions: TokenConfigurationConvention;
    conventionsChangeRules: ChangeControlRules;
    baseSupply: bigint;
    maxSupply?: bigint;
    keepsHistory: TokenKeepsHistoryRules;
    isStartedAsPaused?: boolean;
    isAllowedTransferToFrozenBalance?: boolean;
    maxSupplyChangeRules: ChangeControlRules;
    distributionRules: TokenDistributionRules;
    marketplaceRules: TokenMarketplaceRules;
    manualMintingRules: ChangeControlRules;
    manualBurningRules: ChangeControlRules;
    freezeRules: ChangeControlRules;
    unfreezeRules: ChangeControlRules;
    destroyFrozenFundsRules: ChangeControlRules;
    emergencyActionRules: ChangeControlRules;
    mainControlGroup?: number;
    mainControlGroupCanBeModified: AuthorizedActionTakers;
    description?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenConfigurationOptions")]
    pub type TokenConfigurationOptionsJs;
}

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenConfiguration")]
pub struct TokenConfigurationWasm(TokenConfiguration);

impl From<TokenConfiguration> for TokenConfigurationWasm {
    fn from(configuration: TokenConfiguration) -> Self {
        Self(configuration)
    }
}

impl From<TokenConfigurationWasm> for TokenConfiguration {
    fn from(configuration: TokenConfigurationWasm) -> Self {
        configuration.0
    }
}

impl_try_from_js_value!(TokenConfigurationWasm, "TokenConfiguration");

#[wasm_bindgen(js_class = TokenConfiguration)]
impl TokenConfigurationWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenConfigurationOptionsJs,
    ) -> WasmDppResult<TokenConfigurationWasm> {
        // Extract complex types first (borrows &options)
        let conventions: TokenConfigurationConventionWasm =
            try_from_options(&options, "conventions")?;
        let conventions_change_rules: ChangeControlRulesWasm =
            try_from_options(&options, "conventionsChangeRules")?;
        let keeps_history: TokenKeepsHistoryRulesWasm =
            try_from_options(&options, "keepsHistory")?;
        let max_supply_change_rules: ChangeControlRulesWasm =
            try_from_options(&options, "maxSupplyChangeRules")?;
        let distribution_rules: TokenDistributionRulesWasm =
            try_from_options(&options, "distributionRules")?;
        let marketplace_rules: TokenMarketplaceRulesWasm =
            try_from_options(&options, "marketplaceRules")?;
        let manual_minting_rules: ChangeControlRulesWasm =
            try_from_options(&options, "manualMintingRules")?;
        let manual_burning_rules: ChangeControlRulesWasm =
            try_from_options(&options, "manualBurningRules")?;
        let freeze_rules: ChangeControlRulesWasm = try_from_options(&options, "freezeRules")?;
        let unfreeze_rules: ChangeControlRulesWasm = try_from_options(&options, "unfreezeRules")?;
        let destroy_frozen_funds_rules: ChangeControlRulesWasm =
            try_from_options(&options, "destroyFrozenFundsRules")?;
        let emergency_action_rules: ChangeControlRulesWasm =
            try_from_options(&options, "emergencyActionRules")?;
        let main_control_group_can_be_modified: AuthorizedActionTakersWasm =
            try_from_options(&options, "mainControlGroupCanBeModified")?;

        // Deserialize primitive fields via serde last (consumes options)
        let opts: TokenConfigurationOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(TokenConfigurationWasm(TokenConfiguration::V0(
            TokenConfigurationV0 {
                conventions: conventions.into(),
                conventions_change_rules: conventions_change_rules.into(),
                base_supply: opts.base_supply,
                max_supply: opts.max_supply,
                keeps_history: keeps_history.into(),
                start_as_paused: opts.is_started_as_paused,
                allow_transfer_to_frozen_balance: opts.is_allowed_transfer_to_frozen_balance,
                max_supply_change_rules: max_supply_change_rules.into(),
                distribution_rules: distribution_rules.into(),
                marketplace_rules: marketplace_rules.into(),
                manual_minting_rules: manual_minting_rules.into(),
                manual_burning_rules: manual_burning_rules.into(),
                freeze_rules: freeze_rules.into(),
                unfreeze_rules: unfreeze_rules.into(),
                destroy_frozen_funds_rules: destroy_frozen_funds_rules.into(),
                emergency_action_rules: emergency_action_rules.into(),
                main_control_group: opts.main_control_group,
                main_control_group_can_be_modified: main_control_group_can_be_modified.into(),
                description: opts.description,
            },
        )))
    }

    #[wasm_bindgen(getter = "conventions")]
    pub fn conventions(&self) -> TokenConfigurationConventionWasm {
        self.0.conventions().clone().into()
    }

    #[wasm_bindgen(getter = "conventionsChangeRules")]
    pub fn conventions_change_rules(&self) -> ChangeControlRulesWasm {
        self.0.conventions_change_rules().clone().into()
    }

    #[wasm_bindgen(getter = "baseSupply")]
    pub fn base_supply(&self) -> TokenAmount {
        self.0.base_supply()
    }

    #[wasm_bindgen(getter = "keepsHistory")]
    pub fn keeps_history(&self) -> TokenKeepsHistoryRulesWasm {
        (*self.0.keeps_history()).into()
    }

    #[wasm_bindgen(getter = "isStartedAsPaused")]
    pub fn is_started_as_paused(&self) -> bool {
        self.0.start_as_paused()
    }

    #[wasm_bindgen(getter = "isAllowedTransferToFrozenBalance")]
    pub fn is_allowed_transfer_to_frozen_balance(&self) -> bool {
        self.0.is_allowed_transfer_to_frozen_balance()
    }

    #[wasm_bindgen(getter = "maxSupply")]
    pub fn max_supply(&self) -> Option<TokenAmount> {
        self.0.max_supply()
    }

    #[wasm_bindgen(getter = "maxSupplyChangeRules")]
    pub fn max_supply_change_rules(&self) -> ChangeControlRulesWasm {
        self.0.max_supply_change_rules().clone().into()
    }

    #[wasm_bindgen(getter = "distributionRules")]
    pub fn distribution_rules(&self) -> TokenDistributionRulesWasm {
        self.0.distribution_rules().clone().into()
    }

    #[wasm_bindgen(getter = "marketplaceRules")]
    pub fn marketplace_rules(&self) -> TokenMarketplaceRulesWasm {
        match self.0.clone() {
            TokenConfiguration::V0(v0) => v0.marketplace_rules.clone().into(),
        }
    }

    #[wasm_bindgen(getter = "manualMintingRules")]
    pub fn manual_minting_rules(&self) -> ChangeControlRulesWasm {
        self.0.manual_minting_rules().clone().into()
    }

    #[wasm_bindgen(getter = "manualBurningRules")]
    pub fn manual_burning_rules(&self) -> ChangeControlRulesWasm {
        self.0.manual_burning_rules().clone().into()
    }

    #[wasm_bindgen(getter = "freezeRules")]
    pub fn freeze_rules(&self) -> ChangeControlRulesWasm {
        self.0.freeze_rules().clone().into()
    }

    #[wasm_bindgen(getter = "unfreezeRules")]
    pub fn unfreeze_rules(&self) -> ChangeControlRulesWasm {
        self.0.unfreeze_rules().clone().into()
    }

    #[wasm_bindgen(getter = "destroyFrozenFundsRules")]
    pub fn destroy_frozen_funds_rules(&self) -> ChangeControlRulesWasm {
        self.0.destroy_frozen_funds_rules().clone().into()
    }

    #[wasm_bindgen(getter = "emergencyActionRules")]
    pub fn emergency_action_rules(&self) -> ChangeControlRulesWasm {
        self.0.emergency_action_rules().clone().into()
    }

    #[wasm_bindgen(getter = "mainControlGroup")]
    pub fn main_control_group(&self) -> Option<GroupContractPosition> {
        self.0.main_control_group()
    }

    #[wasm_bindgen(getter = "mainControlGroupCanBeModified")]
    pub fn main_control_group_can_be_modified(&self) -> AuthorizedActionTakersWasm {
        (*self.0.main_control_group_can_be_modified()).into()
    }

    #[wasm_bindgen(getter = "description")]
    pub fn description(&self) -> Option<String> {
        self.0.description().clone()
    }

    #[wasm_bindgen(setter = "conventions")]
    pub fn set_conventions(&mut self, conventions: &TokenConfigurationConventionWasm) {
        self.0.set_conventions(conventions.clone().into())
    }

    #[wasm_bindgen(setter = "conventionsChangeRules")]
    pub fn set_conventions_change_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_conventions_change_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "baseSupply")]
    pub fn set_base_supply(&mut self, base_supply: JsValue) -> WasmDppResult<()> {
        self.0
            .set_base_supply(try_to_u64(&base_supply, "baseSupply")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "keepsHistory")]
    pub fn set_keeps_history(&mut self, keeps_history: &TokenKeepsHistoryRulesWasm) {
        *self.0.keeps_history_mut() = keeps_history.clone().into();
    }

    #[wasm_bindgen(setter = "isStartedAsPaused")]
    pub fn set_is_started_as_paused(&mut self, is_started_as_paused: bool) {
        self.0.set_start_as_paused(is_started_as_paused)
    }

    #[wasm_bindgen(setter = "isAllowedTransferToFrozenBalance")]
    pub fn set_is_allowed_transfer_to_frozen_balance(
        &mut self,
        is_allowed_transfer_to_frozen_balance: bool,
    ) {
        self.0
            .allow_transfer_to_frozen_balance(is_allowed_transfer_to_frozen_balance);
    }

    #[wasm_bindgen(setter = "maxSupply")]
    pub fn set_max_supply(&mut self, max_supply: JsValue) -> WasmDppResult<()> {
        let max_supply = if max_supply.is_undefined() || max_supply.is_null() {
            None
        } else {
            Some(try_to_u64(&max_supply, "maxSupply")?)
        };
        self.0.set_max_supply(max_supply);
        Ok(())
    }

    #[wasm_bindgen(setter = "maxSupplyChangeRules")]
    pub fn set_max_supply_change_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_max_supply_change_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "distributionRules")]
    pub fn set_distribution_rules(&mut self, rules: &TokenDistributionRulesWasm) {
        self.0.set_distribution_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "marketplaceRules")]
    pub fn set_marketplace_rules(&mut self, marketplace_rules: &TokenMarketplaceRulesWasm) {
        self.0 = match self.0.clone() {
            TokenConfiguration::V0(mut v0) => {
                v0.marketplace_rules = marketplace_rules.clone().into();

                TokenConfiguration::V0(v0)
            }
        }
    }

    #[wasm_bindgen(setter = "manualMintingRules")]
    pub fn set_manual_minting_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_manual_minting_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "manualBurningRules")]
    pub fn set_manual_burning_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_manual_burning_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "freezeRules")]
    pub fn set_freeze_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_freeze_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "unfreezeRules")]
    pub fn set_unfreeze_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_unfreeze_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "destroyFrozenFundsRules")]
    pub fn set_destroy_frozen_funds_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_destroy_frozen_funds_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "emergencyActionRules")]
    pub fn set_emergency_action_rules(&mut self, rules: &ChangeControlRulesWasm) {
        self.0.set_emergency_action_rules(rules.clone().into())
    }

    #[wasm_bindgen(setter = "mainControlGroup")]
    pub fn set_main_control_group(&mut self, group: Option<GroupContractPosition>) {
        self.0.set_main_control_group(group)
    }

    #[wasm_bindgen(setter = "mainControlGroupCanBeModified")]
    pub fn set_main_control_group_can_be_modified(
        &mut self,
        authorized_action_taker: &AuthorizedActionTakersWasm,
    ) {
        self.0
            .set_main_control_group_can_be_modified(authorized_action_taker.clone().into())
    }

    #[wasm_bindgen(setter = "description")]
    pub fn set_description(&mut self, description: Option<String>) {
        self.0.set_description(description)
    }

    #[wasm_bindgen(js_name = "calculateTokenId")]
    pub fn calculate_token_id(
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "tokenPos")] token_pos: TokenContractPosition,
    ) -> WasmDppResult<IdentifierWasm> {
        let contract_id: Identifier = contract_id.try_into()?;

        Ok(IdentifierWasm::from(calculate_token_id(
            contract_id.as_bytes(),
            token_pos,
        )))
    }
}

impl_wasm_type_info!(TokenConfigurationWasm, TokenConfiguration);
