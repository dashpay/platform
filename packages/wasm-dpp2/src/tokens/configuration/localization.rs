use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::IntoWasm;
use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use dpp::data_contract::associated_token::token_configuration_localization::accessors::v0::{
    TokenConfigurationLocalizationV0Getters, TokenConfigurationLocalizationV0Setters,
};
use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/**
 * TokenConfigurationLocalization serialized as a plain object.
 */
export interface TokenConfigurationLocalizationObject {
    $formatVersion: string;
    shouldCapitalize: boolean;
    singularForm: string;
    pluralForm: string;
}

/**
 * TokenConfigurationLocalization serialized as JSON.
 */
export interface TokenConfigurationLocalizationJSON {
    $formatVersion: string;
    shouldCapitalize: boolean;
    singularForm: string;
    pluralForm: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenConfigurationLocalizationObject")]
    pub type TokenConfigurationLocalizationObjectJs;

    #[wasm_bindgen(typescript_type = "TokenConfigurationLocalizationJSON")]
    pub type TokenConfigurationLocalizationJSONJs;
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenConfigurationLocalization")]
pub struct TokenConfigurationLocalizationWasm(TokenConfigurationLocalization);

impl From<TokenConfigurationLocalization> for TokenConfigurationLocalizationWasm {
    fn from(configuration: TokenConfigurationLocalization) -> TokenConfigurationLocalizationWasm {
        TokenConfigurationLocalizationWasm(configuration)
    }
}

impl From<TokenConfigurationLocalizationWasm> for TokenConfigurationLocalization {
    fn from(configuration: TokenConfigurationLocalizationWasm) -> TokenConfigurationLocalization {
        configuration.0
    }
}

#[wasm_bindgen(js_class = TokenConfigurationLocalization)]
impl TokenConfigurationLocalizationWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        should_capitalize: bool,
        singular_form: String,
        plural_form: String,
    ) -> TokenConfigurationLocalizationWasm {
        TokenConfigurationLocalizationWasm(TokenConfigurationLocalization::V0(
            TokenConfigurationLocalizationV0 {
                should_capitalize,
                singular_form,
                plural_form,
            },
        ))
    }

    #[wasm_bindgen(getter = "shouldCapitalize")]
    pub fn get_should_capitalize(&self) -> bool {
        self.0.should_capitalize()
    }

    #[wasm_bindgen(getter = "pluralForm")]
    pub fn get_plural_form(&self) -> String {
        self.0.plural_form().to_string()
    }

    #[wasm_bindgen(getter = "singularForm")]
    pub fn get_singular_form(&self) -> String {
        self.0.singular_form().to_string()
    }

    #[wasm_bindgen(setter = "shouldCapitalize")]
    pub fn set_should_capitalize(&mut self, capitalize: bool) {
        self.0.set_should_capitalize(capitalize);
    }

    #[wasm_bindgen(setter = "pluralForm")]
    pub fn set_plural_form(&mut self, plural_form: String) {
        self.0.set_plural_form(plural_form);
    }

    #[wasm_bindgen(setter = "singularForm")]
    pub fn set_singular_form(&mut self, singular_form: String) {
        self.0.set_singular_form(singular_form);
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<TokenConfigurationLocalizationJSONJs> {
        serialization::to_json(&self.0).map(Into::into)
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(
        value: TokenConfigurationLocalizationJSONJs,
    ) -> WasmDppResult<TokenConfigurationLocalizationWasm> {
        serialization::from_json(value.into()).map(TokenConfigurationLocalizationWasm)
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<TokenConfigurationLocalizationObjectJs> {
        serialization::to_object(&self.0).map(Into::into)
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(
        value: TokenConfigurationLocalizationObjectJs,
    ) -> WasmDppResult<TokenConfigurationLocalizationWasm> {
        serialization::from_object(value.into()).map(TokenConfigurationLocalizationWasm)
    }
}

impl TokenConfigurationLocalizationWasm {
    pub(crate) fn from_js_value(
        js_value: &JsValue,
    ) -> WasmDppResult<TokenConfigurationLocalization> {
        // First, check if it's already a WASM wrapper
        if let Ok(wasm_localization) =
            js_value.to_wasm::<TokenConfigurationLocalizationWasm>("TokenConfigurationLocalization")
        {
            return Ok(TokenConfigurationLocalization::from(
                wasm_localization.clone(),
            ));
        }

        // Deserialize as a versioned object (with $format_version)
        serialization::from_object(js_value.clone())
    }
}

impl_wasm_type_info!(
    TokenConfigurationLocalizationWasm,
    TokenConfigurationLocalization
);
