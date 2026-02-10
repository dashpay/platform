use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::tokens::configuration::localization::TokenConfigurationLocalizationWasm;
use crate::utils::{JsValueExt, try_from_options, try_to_object, try_to_string, try_to_u8};
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

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Record<string, TokenConfigurationLocalization>")]
    pub type TokenConfigurationLocalizationsJs;
}

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        localizations: &JsValue,
        decimals: u8,
    ) -> WasmDppResult<TokenConfigurationConventionWasm> {
        let localizations: BTreeMap<String, TokenConfigurationLocalization> =
            value_to_localizations(localizations)?;

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
    pub fn localizations(&self) -> WasmDppResult<TokenConfigurationLocalizationsJs> {
        let object = Object::new();

        for (key, value) in &self.0.localizations().clone() {
            Reflect::set(
                &object,
                &JsValue::from(key.clone()),
                &TokenConfigurationLocalizationWasm::from(value.clone()).into(),
            )
            .map_err(|err| {
                let message = err.error_message();
                WasmDppError::generic(format!(
                    "unable to serialize localization '{}': {}",
                    key, message
                ))
            })?;
        }

        Ok(JsValue::from(object).into())
    }

    #[wasm_bindgen(setter = "decimals")]
    pub fn set_decimals(&mut self, decimals: &js_sys::Number) -> WasmDppResult<()> {
        self.0.set_decimals(try_to_u8(decimals, "decimals")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "localizations")]
    pub fn set_localizations(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<string, TokenConfigurationLocalization>")]
        localizations: &JsValue,
    ) -> WasmDppResult<()> {
        let localizations: BTreeMap<String, TokenConfigurationLocalization> =
            value_to_localizations(localizations)?;

        self.0.set_localizations(localizations);
        Ok(())
    }
}

fn value_to_localizations(
    localizations_value: &JsValue,
) -> WasmDppResult<BTreeMap<String, TokenConfigurationLocalization>> {
    let js_object = try_to_object(localizations_value.clone(), "localizations")?;
    let mut localizations = BTreeMap::new();

    for key in Object::keys(&js_object) {
        let key_str = try_to_string(&key, "localization key")?;

        let localization: TokenConfigurationLocalizationWasm =
            try_from_options(&js_object.clone().into(), &key_str)?;

        localizations.insert(key_str, localization.into());
    }

    Ok(localizations)
}

impl_try_from_js_value!(
    TokenConfigurationConventionWasm,
    "TokenConfigurationConvention"
);
impl_wasm_type_info!(
    TokenConfigurationConventionWasm,
    TokenConfigurationConvention
);
