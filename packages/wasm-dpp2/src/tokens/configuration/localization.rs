use crate::error::WasmDppError;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::IntoWasm;
use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use dpp::data_contract::associated_token::token_configuration_localization::accessors::v0::{
    TokenConfigurationLocalizationV0Getters, TokenConfigurationLocalizationV0Setters,
};
use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
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
        #[wasm_bindgen(js_name = "shouldCapitalize")] should_capitalize: bool,
        #[wasm_bindgen(js_name = "singularForm")] singular_form: String,
        #[wasm_bindgen(js_name = "pluralForm")] plural_form: String,
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
    pub fn should_capitalize(&self) -> bool {
        self.0.should_capitalize()
    }

    #[wasm_bindgen(getter = "pluralForm")]
    pub fn plural_form(&self) -> String {
        self.0.plural_form().to_string()
    }

    #[wasm_bindgen(getter = "singularForm")]
    pub fn singular_form(&self) -> String {
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
}

crate::impl_wasm_conversions_inner!(
    TokenConfigurationLocalizationWasm,
    TokenConfigurationLocalization,
    TokenConfigurationLocalization,
    TokenConfigurationLocalizationObjectJs,
    TokenConfigurationLocalizationJSONJs
);

impl TryFrom<&JsValue> for TokenConfigurationLocalizationWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        // First, check if it's already a WASM wrapper
        if let Ok(wasm_localization) =
            value.to_wasm::<TokenConfigurationLocalizationWasm>("TokenConfigurationLocalization")
        {
            return Ok(wasm_localization.clone());
        }

        // Deserialize as a versioned object (with $formatVersion) via the
        // canonical ValueConvertible trait.
        use dpp::serialization::ValueConvertible;
        let pv = serialization::platform_value_from_object(value)?;
        let inner = TokenConfigurationLocalization::from_object(pv)
            .map_err(|e| WasmDppError::serialization(format!("from_object: {}", e)))?;
        Ok(TokenConfigurationLocalizationWasm(inner))
    }
}

impl_wasm_type_info!(
    TokenConfigurationLocalizationWasm,
    TokenConfigurationLocalization
);
