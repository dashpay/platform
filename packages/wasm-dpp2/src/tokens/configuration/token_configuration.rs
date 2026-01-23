use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::change_control_rules::ChangeControlRulesWasm;
use crate::tokens::configuration::configuration_convention::TokenConfigurationConventionWasm;
use crate::tokens::configuration::distribution_rules::TokenDistributionRulesWasm;
use crate::tokens::configuration::keeps_history_rules::TokenKeepsHistoryRulesWasm;
use crate::tokens::configuration::marketplace_rules::TokenMarketplaceRulesWasm;
use crate::utils::{IntoWasm, get_required_property};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use dpp::data_contract::{GroupContractPosition, TokenConfiguration, TokenContractPosition};
use dpp::prelude::Identifier;
use dpp::tokens::calculate_token_id;
use js_sys::Object;
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
const TS_TYPES: &'static str = r#"
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

#[wasm_bindgen(js_class = TokenConfiguration)]
impl TokenConfigurationWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenConfigurationOptionsJs,
    ) -> WasmDppResult<TokenConfigurationWasm> {
        let options: JsValue = options.into();
        let object = Object::from(options.clone());

        // Extract conventions (required)
        let conventions = get_required_property(&object, "conventions")?
            .to_wasm::<TokenConfigurationConventionWasm>("TokenConfigurationConvention")?
            .clone();

        // Extract conventionsChangeRules (required)
        let conventions_change_rules = get_required_property(&object, "conventionsChangeRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract keepsHistory (required)
        let keeps_history = get_required_property(&object, "keepsHistory")?
            .to_wasm::<TokenKeepsHistoryRulesWasm>("TokenKeepsHistoryRules")?
            .clone();

        // Extract maxSupplyChangeRules (required)
        let max_supply_change_rules = get_required_property(&object, "maxSupplyChangeRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract distributionRules (required)
        let distribution_rules = get_required_property(&object, "distributionRules")?
            .to_wasm::<TokenDistributionRulesWasm>("TokenDistributionRules")?
            .clone();

        // Extract marketplaceRules (required)
        let marketplace_rules = get_required_property(&object, "marketplaceRules")?
            .to_wasm::<TokenMarketplaceRulesWasm>("TokenMarketplaceRules")?
            .clone();

        // Extract manualMintingRules (required)
        let manual_minting_rules = get_required_property(&object, "manualMintingRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract manualBurningRules (required)
        let manual_burning_rules = get_required_property(&object, "manualBurningRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract freezeRules (required)
        let freeze_rules = get_required_property(&object, "freezeRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract unfreezeRules (required)
        let unfreeze_rules = get_required_property(&object, "unfreezeRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract destroyFrozenFundsRules (required)
        let destroy_frozen_funds_rules = get_required_property(&object, "destroyFrozenFundsRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract emergencyActionRules (required)
        let emergency_action_rules = get_required_property(&object, "emergencyActionRules")?
            .to_wasm::<ChangeControlRulesWasm>("ChangeControlRules")?
            .clone();

        // Extract mainControlGroupCanBeModified (required)
        let main_control_group_can_be_modified =
            get_required_property(&object, "mainControlGroupCanBeModified")?
                .to_wasm::<AuthorizedActionTakersWasm>("AuthorizedActionTakers")?
                .clone();

        // Extract simple fields via serde
        let opts: TokenConfigurationOptions = serde_wasm_bindgen::from_value(options)
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
    pub fn get_conventions(&self) -> TokenConfigurationConventionWasm {
        self.0.conventions().clone().into()
    }

    #[wasm_bindgen(getter = "conventionsChangeRules")]
    pub fn get_conventions_change_rules(&self) -> ChangeControlRulesWasm {
        self.0.conventions_change_rules().clone().into()
    }

    #[wasm_bindgen(getter = "baseSupply")]
    pub fn get_base_supply(&self) -> TokenAmount {
        self.0.base_supply()
    }

    #[wasm_bindgen(getter = "keepsHistory")]
    pub fn get_keeps_history(&self) -> TokenKeepsHistoryRulesWasm {
        (*self.0.keeps_history()).into()
    }

    #[wasm_bindgen(getter = "isStartedAsPaused")]
    pub fn is_started_as_paused(&self) -> bool {
        self.0.start_as_paused()
    }

    #[wasm_bindgen(getter = "isAllowedTransferToFrozenBalance")]
    pub fn get_is_allowed_transfer_to_frozen_balance(&self) -> bool {
        self.0.is_allowed_transfer_to_frozen_balance()
    }

    #[wasm_bindgen(getter = "maxSupply")]
    pub fn get_max_supply(&self) -> Option<TokenAmount> {
        self.0.max_supply()
    }

    #[wasm_bindgen(getter = "maxSupplyChangeRules")]
    pub fn get_max_supply_change_rules(&self) -> ChangeControlRulesWasm {
        self.0.max_supply_change_rules().clone().into()
    }

    #[wasm_bindgen(getter = "distributionRules")]
    pub fn get_distribution_rules(&self) -> TokenDistributionRulesWasm {
        self.0.distribution_rules().clone().into()
    }

    #[wasm_bindgen(getter = "marketplaceRules")]
    pub fn get_marketplace_rules(&self) -> TokenMarketplaceRulesWasm {
        match self.0.clone() {
            TokenConfiguration::V0(v0) => v0.marketplace_rules.clone().into(),
        }
    }

    #[wasm_bindgen(getter = "manualMintingRules")]
    pub fn get_manual_minting_rules(&self) -> ChangeControlRulesWasm {
        self.0.manual_minting_rules().clone().into()
    }

    #[wasm_bindgen(getter = "manualBurningRules")]
    pub fn get_manual_burning_rules(&self) -> ChangeControlRulesWasm {
        self.0.manual_burning_rules().clone().into()
    }

    #[wasm_bindgen(getter = "freezeRules")]
    pub fn get_freeze_rules(&self) -> ChangeControlRulesWasm {
        self.0.freeze_rules().clone().into()
    }

    #[wasm_bindgen(getter = "unfreezeRules")]
    pub fn get_unfreeze_rules(&self) -> ChangeControlRulesWasm {
        self.0.unfreeze_rules().clone().into()
    }

    #[wasm_bindgen(getter = "destroyFrozenFundsRules")]
    pub fn get_destroy_frozen_funds_rules(&self) -> ChangeControlRulesWasm {
        self.0.destroy_frozen_funds_rules().clone().into()
    }

    #[wasm_bindgen(getter = "emergencyActionRules")]
    pub fn get_emergency_action_rules(&self) -> ChangeControlRulesWasm {
        self.0.emergency_action_rules().clone().into()
    }

    #[wasm_bindgen(getter = "mainControlGroup")]
    pub fn get_main_control_group(&self) -> Option<GroupContractPosition> {
        self.0.main_control_group()
    }

    #[wasm_bindgen(getter = "mainControlGroupCanBeModified")]
    pub fn get_main_control_group_can_be_modified(&self) -> AuthorizedActionTakersWasm {
        (*self.0.main_control_group_can_be_modified()).into()
    }

    #[wasm_bindgen(getter = "description")]
    pub fn get_description(&self) -> Option<String> {
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
    pub fn set_base_supply(&mut self, base_supply: TokenAmount) {
        self.0.set_base_supply(base_supply)
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
    pub fn set_max_supply(&mut self, max_supply: Option<TokenAmount>) {
        self.0.set_max_supply(max_supply)
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
        contract_id: IdentifierLikeJs,
        token_pos: TokenContractPosition,
    ) -> WasmDppResult<IdentifierWasm> {
        let contract_id: Identifier = contract_id.try_into()?;

        Ok(IdentifierWasm::from(calculate_token_id(
            contract_id.as_bytes(),
            token_pos,
        )))
    }
}

impl_wasm_type_info!(TokenConfigurationWasm, TokenConfiguration);
