use crate::error::{WasmDppError, WasmDppResult};
use crate::token_configuration::localization::TokenConfigurationLocalizationWasm;
use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::{
    TokenConfigurationConventionV0Getters, TokenConfigurationConventionV0Setters,
};
use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use js_sys::{Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenConfigurationConvention")]
pub struct TokenConfigurationConventionWasm(TokenConfigurationConvention);

impl From<TokenConfigurationConvention> for TokenConfigurationConventionWasm {
    fn from(convention: TokenConfigurationConvention) -> Self {
        TokenConfigurationConventionWasm(convention)
    }
}

impl From<TokenConfigurationConventionWasm> for TokenConfigurationConvention {
    fn from(convention: TokenConfigurationConventionWasm) -> Self {
        convention.0
    }
}

#[wasm_bindgen(js_class = TokenConfigurationConvention)]
impl TokenConfigurationConventionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenConfigurationConvention".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenConfigurationConvention".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_localizations: &JsValue,
        decimals: u8,
    ) -> WasmDppResult<TokenConfigurationConventionWasm> {
        let localizations: BTreeMap<String, TokenConfigurationLocalization> =
            js_value_to_localizations(js_localizations)?;

        Ok(TokenConfigurationConventionWasm(
            TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
                localizations,
                decimals,
            }),
        ))
    }

    #[wasm_bindgen(getter = "decimals")]
    pub fn decimals(&self) -> u8 {
        self.0.decimals()
    }

    #[wasm_bindgen(getter = "localizations")]
    pub fn localizations(&self) -> WasmDppResult<JsValue> {
        let object = Object::new();

        for (key, value) in &self.0.localizations().clone() {
            Reflect::set(
                &object,
                &JsValue::from(key.clone()),
                &TokenConfigurationLocalizationWasm::from(value.clone()).into(),
            )
            .map_err(|err| WasmDppError::from_js_value(err))?;
        }

        Ok(object.into())
    }

    #[wasm_bindgen(setter = "decimals")]
    pub fn set_decimals(&mut self, decimals: u8) {
        self.0.set_decimals(decimals)
    }

    #[wasm_bindgen(setter = "localizations")]
    pub fn set_localizations(&mut self, js_localizations: &JsValue) -> WasmDppResult<()> {
        let localizations: BTreeMap<String, TokenConfigurationLocalization> =
            js_value_to_localizations(js_localizations)?;

        self.0.set_localizations(localizations);
        Ok(())
    }
}

fn js_value_to_localizations(
    js_localizations: &JsValue,
) -> WasmDppResult<BTreeMap<String, TokenConfigurationLocalization>> {
    let js_object = Object::from(js_localizations.clone());
    let mut localizations = BTreeMap::new();

    for key in Object::keys(&js_object) {
        let key_str = key
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("Localization key must be string"))?;

        let js_value = Reflect::get(&js_object, &key).map_err(WasmDppError::from_js_value)?;

        let localization = TokenConfigurationLocalizationWasm::from_js_value(&js_value)?;

        localizations.insert(key_str, localization);
    }

    Ok(localizations)
}
