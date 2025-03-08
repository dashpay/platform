use crate::data_contract::associated_token::token_configuration_localization::accessors::v0::{
    TokenConfigurationLocalizationV0Getters, TokenConfigurationLocalizationV0Setters,
};
use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;

pub mod v0;

impl TokenConfigurationLocalizationV0Getters for TokenConfigurationLocalization {
    fn should_capitalize(&self) -> bool {
        match self {
            TokenConfigurationLocalization::V0(v0) => v0.should_capitalize,
        }
    }

    fn singular_form(&self) -> &str {
        match self {
            TokenConfigurationLocalization::V0(v0) => v0.singular_form.as_str(),
        }
    }

    fn plural_form(&self) -> &str {
        match self {
            TokenConfigurationLocalization::V0(v0) => v0.plural_form.as_str(),
        }
    }
}

impl TokenConfigurationLocalizationV0Setters for TokenConfigurationLocalization {
    fn set_should_capitalize(&mut self, should_capitalize: bool) {
        match self {
            TokenConfigurationLocalization::V0(v0) => v0.should_capitalize = should_capitalize,
        }
    }

    fn set_singular_form(&mut self, singular_form: String) {
        match self {
            TokenConfigurationLocalization::V0(v0) => v0.singular_form = singular_form,
        }
    }

    fn set_plural_form(&mut self, plural_form: String) {
        match self {
            TokenConfigurationLocalization::V0(v0) => v0.plural_form = plural_form,
        }
    }
}
