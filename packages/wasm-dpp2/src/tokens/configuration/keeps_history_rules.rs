use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use dpp::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::{
    TokenKeepsHistoryRulesV0Getters, TokenKeepsHistoryRulesV0Setters,
};
use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenKeepsHistoryRulesOptions {
    #[serde(default)]
    is_keeping_transfer_history: bool,
    #[serde(default)]
    is_keeping_freezing_history: bool,
    #[serde(default)]
    is_keeping_minting_history: bool,
    #[serde(default)]
    is_keeping_burning_history: bool,
    #[serde(default)]
    is_keeping_direct_pricing_history: bool,
    #[serde(default)]
    is_keeping_direct_purchase_history: bool,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface TokenKeepsHistoryRulesOptions {
    isKeepingTransferHistory?: boolean;
    isKeepingFreezingHistory?: boolean;
    isKeepingMintingHistory?: boolean;
    isKeepingBurningHistory?: boolean;
    isKeepingDirectPricingHistory?: boolean;
    isKeepingDirectPurchaseHistory?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenKeepsHistoryRulesOptions")]
    pub type TokenKeepsHistoryRulesOptionsJs;
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "TokenKeepsHistoryRules")]
pub struct TokenKeepsHistoryRulesWasm(TokenKeepsHistoryRules);

impl From<TokenKeepsHistoryRulesWasm> for TokenKeepsHistoryRules {
    fn from(rules: TokenKeepsHistoryRulesWasm) -> Self {
        rules.0
    }
}

impl From<TokenKeepsHistoryRules> for TokenKeepsHistoryRulesWasm {
    fn from(rules: TokenKeepsHistoryRules) -> Self {
        Self(rules)
    }
}

#[wasm_bindgen(js_class = TokenKeepsHistoryRules)]
impl TokenKeepsHistoryRulesWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenKeepsHistoryRulesOptionsJs,
    ) -> WasmDppResult<TokenKeepsHistoryRulesWasm> {
        let opts: TokenKeepsHistoryRulesOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(TokenKeepsHistoryRulesWasm(TokenKeepsHistoryRules::V0(
            TokenKeepsHistoryRulesV0 {
                keeps_transfer_history: opts.is_keeping_transfer_history,
                keeps_freezing_history: opts.is_keeping_freezing_history,
                keeps_minting_history: opts.is_keeping_minting_history,
                keeps_burning_history: opts.is_keeping_burning_history,
                keeps_direct_pricing_history: opts.is_keeping_direct_pricing_history,
                keeps_direct_purchase_history: opts.is_keeping_direct_purchase_history,
            },
        )))
    }

    #[wasm_bindgen(getter = "isKeepingTransferHistory")]
    pub fn is_keeping_transfer_history(&self) -> bool {
        self.0.keeps_transfer_history()
    }

    #[wasm_bindgen(getter = "isKeepingFreezingHistory")]
    pub fn is_keeping_freezing_history(&self) -> bool {
        self.0.keeps_freezing_history()
    }

    #[wasm_bindgen(getter = "isKeepingMintingHistory")]
    pub fn is_keeping_minting_history(&self) -> bool {
        self.0.keeps_minting_history()
    }

    #[wasm_bindgen(getter = "isKeepingBurningHistory")]
    pub fn is_keeping_burning_history(&self) -> bool {
        self.0.keeps_burning_history()
    }

    #[wasm_bindgen(getter = "isKeepingDirectPricingHistory")]
    pub fn is_keeping_direct_pricing_history(&self) -> bool {
        self.0.keeps_direct_pricing_history()
    }

    #[wasm_bindgen(getter = "isKeepingDirectPurchaseHistory")]
    pub fn is_keeping_direct_purchase_history(&self) -> bool {
        self.0.keeps_direct_purchase_history()
    }

    #[wasm_bindgen(setter = "isKeepingTransferHistory")]
    pub fn set_is_keeping_transfer_history(&mut self, is_keeping_transfer_history: bool) {
        self.0
            .set_keeps_transfer_history(is_keeping_transfer_history);
    }

    #[wasm_bindgen(setter = "isKeepingFreezingHistory")]
    pub fn set_is_keeping_freezing_history(&mut self, is_keeping_freezing_history: bool) {
        self.0
            .set_keeps_freezing_history(is_keeping_freezing_history);
    }

    #[wasm_bindgen(setter = "isKeepingMintingHistory")]
    pub fn set_is_keeping_minting_history(&mut self, is_keeping_minting_history: bool) {
        self.0.set_keeps_minting_history(is_keeping_minting_history);
    }

    #[wasm_bindgen(setter = "isKeepingBurningHistory")]
    pub fn set_is_keeping_burning_history(&mut self, is_keeping_burning_history: bool) {
        self.0.set_keeps_burning_history(is_keeping_burning_history);
    }

    #[wasm_bindgen(setter = "isKeepingDirectPricingHistory")]
    pub fn set_is_keeping_direct_pricing_history(
        &mut self,
        is_keeping_direct_pricing_history: bool,
    ) {
        self.0
            .set_keeps_direct_pricing_history(is_keeping_direct_pricing_history);
    }

    #[wasm_bindgen(setter = "isKeepingDirectPurchaseHistory")]
    pub fn set_is_keeping_direct_purchase_history(
        &mut self,
        is_keeping_direct_purchase_history: bool,
    ) {
        self.0
            .set_keeps_direct_purchase_history(is_keeping_direct_purchase_history);
    }
}

impl_wasm_type_info!(TokenKeepsHistoryRulesWasm, TokenKeepsHistoryRules);
