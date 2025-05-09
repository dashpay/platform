pub mod v0;

use crate::data_contract::associated_token::token_configuration_convention::accessors::v0::{
    TokenConfigurationConventionV0Getters, TokenConfigurationConventionV0Setters,
};
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use std::collections::BTreeMap;

impl TokenConfigurationConventionV0Getters for TokenConfigurationConvention {
    fn singular_form_by_language_code_or_default(&self, language_code: &str) -> &str {
        match self {
            TokenConfigurationConvention::V0(v0) => {
                v0.singular_form_by_language_code_or_default(language_code)
            }
        }
    }

    fn plural_form_by_language_code_or_default(&self, language_code: &str) -> &str {
        match self {
            TokenConfigurationConvention::V0(v0) => {
                v0.plural_form_by_language_code_or_default(language_code)
            }
        }
    }

    fn localizations(&self) -> &BTreeMap<String, TokenConfigurationLocalization> {
        match self {
            TokenConfigurationConvention::V0(v0) => v0.localizations(),
        }
    }

    fn localizations_mut(&mut self) -> &mut BTreeMap<String, TokenConfigurationLocalization> {
        match self {
            TokenConfigurationConvention::V0(v0) => v0.localizations_mut(),
        }
    }

    fn decimals(&self) -> u8 {
        match self {
            TokenConfigurationConvention::V0(v0) => v0.decimals(),
        }
    }
}

impl TokenConfigurationConventionV0Setters for TokenConfigurationConvention {
    fn set_localizations(
        &mut self,
        localizations: BTreeMap<String, TokenConfigurationLocalization>,
    ) {
        match self {
            TokenConfigurationConvention::V0(v0) => v0.set_localizations(localizations),
        }
    }

    fn set_decimals(&mut self, decimals: u8) {
        match self {
            TokenConfigurationConvention::V0(v0) => v0.set_decimals(decimals),
        }
    }
}
