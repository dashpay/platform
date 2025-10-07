use crate::error::{WasmDppError, WasmDppResult};
use crate::utils::IntoWasm;
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
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let object = Object::new();

        Reflect::set(
            &object,
            &JsValue::from("shouldCapitalize"),
            &JsValue::from(self.0.should_capitalize()),
        )
        .map_err(|err| WasmDppError::from_js_value(err))?;
        Reflect::set(
            &object,
            &JsValue::from("pluralForm"),
            &JsValue::from(self.0.plural_form()),
        )
        .map_err(WasmDppError::from_js_value)?;
        Reflect::set(
            &object,
            &JsValue::from("singularForm"),
            &JsValue::from(self.0.singular_form()),
        )
        .map_err(WasmDppError::from_js_value)?;

        Ok(object.into())
    }
}

impl TokenConfigurationLocalizationWasm {
    pub(crate) fn from_js_value(
        js_value: &JsValue,
    ) -> WasmDppResult<TokenConfigurationLocalization> {
        match js_value
            .to_wasm::<TokenConfigurationLocalizationWasm>("TokenConfigurationLocalization")
        {
            Ok(wasm_localization) => Ok(TokenConfigurationLocalization::from(
                wasm_localization.clone(),
            )),
            Err(err) => {
                let message = err.message();

                if message.contains("constructor name mismatch") {
                    localization_from_plain_js_value(js_value)
                } else {
                    Err(err)
                }
            }
        }
    }
}

fn localization_from_plain_js_value(
    js_value: &JsValue,
) -> WasmDppResult<TokenConfigurationLocalization> {
    if !js_value.is_object() {
        return Err(WasmDppError::invalid_argument(
            "TokenConfigurationLocalization must be an object",
        ));
    }

    let should_capitalize_value = Reflect::get(js_value, &JsValue::from_str("shouldCapitalize"))
        .map_err(WasmDppError::from_js_value)?;
    let should_capitalize = should_capitalize_value.as_bool().ok_or_else(|| {
        WasmDppError::invalid_argument(
            "TokenConfigurationLocalization.shouldCapitalize must be a boolean",
        )
    })?;

    let singular_form_value = Reflect::get(js_value, &JsValue::from_str("singularForm"))
        .map_err(WasmDppError::from_js_value)?;
    let singular_form = singular_form_value.as_string().ok_or_else(|| {
        WasmDppError::invalid_argument(
            "TokenConfigurationLocalization.singularForm must be a string",
        )
    })?;

    let plural_form_value = Reflect::get(js_value, &JsValue::from_str("pluralForm"))
        .map_err(WasmDppError::from_js_value)?;
    let plural_form = plural_form_value.as_string().ok_or_else(|| {
        WasmDppError::invalid_argument("TokenConfigurationLocalization.pluralForm must be a string")
    })?;

    Ok(TokenConfigurationLocalization::V0(
        TokenConfigurationLocalizationV0 {
            should_capitalize,
            singular_form,
            plural_form,
        },
    ))
}
