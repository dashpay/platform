use crate::consensus::basic::data_contract::{
    DecimalsOverLimitError, InvalidTokenLanguageCodeError, InvalidTokenNameCharacterError,
    InvalidTokenNameLengthError,
};
use crate::consensus::basic::token::MissingDefaultLocalizationError;
use crate::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_configuration_localization::accessors::v0::TokenConfigurationLocalizationV0Getters;
use crate::validation::SimpleConsensusValidationResult;
use once_cell::sync::Lazy;
use regex::Regex;

static LANG_CODE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z]{2,3}(-[a-zA-Z]{4})?(-[a-zA-Z]{2}|\d{3})?(-[a-zA-Z0-9]{4,8})*(-[a-wy-zA-WY-Z0-9](-[a-zA-Z0-9]{2,8})+)*(-x(-[a-zA-Z0-9]{1,8})+)?$").unwrap()
});

impl TokenConfigurationConvention {
    #[inline(always)]
    pub(super) fn validate_localizations_v0(&self) -> SimpleConsensusValidationResult {
        let english_localization = match self {
            TokenConfigurationConvention::V0(v0) => v0.localizations.get("en"),
        };

        // If there is no English localization, return an error
        if english_localization.is_none() {
            return SimpleConsensusValidationResult::new_with_error(
                MissingDefaultLocalizationError::new().into(),
            );
        }

        // Max decimals is defined as 16
        if self.decimals() > 16 {
            return SimpleConsensusValidationResult::new_with_error(
                DecimalsOverLimitError::new(self.decimals(), 16).into(),
            );
        }

        for (language, localization) in self.localizations() {
            let singular_form = localization.singular_form();
            let plural_form = localization.plural_form();

            if !singular_form
                .chars()
                .all(|c| !c.is_control() && !c.is_whitespace())
            {
                // This would mean we have an invalid character
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenNameCharacterError::new(
                        "singular form".to_string(),
                        singular_form.to_string(),
                    )
                    .into(),
                );
            }

            if !plural_form
                .chars()
                .all(|c| !c.is_control() && !c.is_whitespace())
            {
                // This would mean we have an invalid character
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenNameCharacterError::new(
                        "plural form".to_string(),
                        plural_form.to_string(),
                    )
                    .into(),
                );
            }

            if !language
                .chars()
                .all(|c| !c.is_control() && !c.is_whitespace())
            {
                // This would mean we have an invalid character
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenNameCharacterError::new(
                        "language code".to_string(),
                        language.clone(),
                    )
                    .into(),
                );
            }

            if singular_form.len() < 3 || singular_form.len() > 25 {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenNameLengthError::new(singular_form.len(), 3, 25, "singular form")
                        .into(),
                );
            }
            if plural_form.len() < 3 || plural_form.len() > 25 {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenNameLengthError::new(plural_form.len(), 3, 25, "plural form")
                        .into(),
                );
            }

            if language.len() < 2 || language.len() > 12 {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenNameLengthError::new(language.len(), 2, 12, "language code").into(),
                );
            }

            if !LANG_CODE_REGEX.is_match(language) {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenLanguageCodeError::new(language.clone()).into(),
                );
            }
        }

        // If we reach here with no errors, return an empty result
        SimpleConsensusValidationResult::new()
    }
}
