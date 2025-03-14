use crate::errors::consensus::basic::token::MissingDefaultLocalizationError;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::validation::SimpleConsensusValidationResult;

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

        // If we reach here with no errors, return an empty result
        SimpleConsensusValidationResult::new()
    }
}
