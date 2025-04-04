use crate::data_contract::associated_token::token_configuration_localization::accessors::v0::{
    TokenConfigurationLocalizationV0Getters, TokenConfigurationLocalizationV0Setters,
};
use crate::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;

impl TokenConfigurationLocalizationV0Getters for TokenConfigurationLocalizationV0 {
    fn should_capitalize(&self) -> bool {
        self.should_capitalize
    }

    fn singular_form(&self) -> &str {
        &self.singular_form
    }

    fn plural_form(&self) -> &str {
        &self.plural_form
    }
}

impl TokenConfigurationLocalizationV0Setters for TokenConfigurationLocalizationV0 {
    fn set_should_capitalize(&mut self, should_capitalize: bool) {
        self.should_capitalize = should_capitalize;
    }

    fn set_singular_form(&mut self, singular_form: String) {
        self.singular_form = singular_form;
    }

    fn set_plural_form(&mut self, plural_form: String) {
        self.plural_form = plural_form;
    }
}
