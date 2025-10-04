use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use dpp::data_contract::associated_token::token_configuration_localization::accessors::v0::{
    TokenConfigurationLocalizationV0Getters, TokenConfigurationLocalizationV0Setters,
};
use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = TokenConfigurationLocalization)]
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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenConfigurationLocalization".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenConfigurationLocalization".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
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
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let object = Object::new();

        Reflect::set(
            &object,
            &JsValue::from("shouldCapitalize"),
            &JsValue::from(self.0.should_capitalize()),
        )?;
        Reflect::set(
            &object,
            &JsValue::from("pluralForm"),
            &JsValue::from(self.0.plural_form()),
        )?;
        Reflect::set(
            &object,
            &JsValue::from("singularForm"),
            &JsValue::from(self.0.singular_form()),
        )?;

        Ok(object.into())
    }
}
