use crate::prelude::DataContract;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

mod v0;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

impl DataContract {
    /// Validates the provided keywords to ensure they meet the requirements for data contracts.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the data contract these keywords belong to.
    /// - `keywords`: A slice of keyword strings to validate.
    /// - `validate_structure`: If true, validates individual keyword structure (length, characters).
    ///   If false, only validates count and uniqueness. Use false when structure was already
    ///   validated in basic structure validation.
    /// - `platform_version`: A reference to the [`PlatformVersion`](crate::version::PlatformVersion)
    ///   object specifying the version of the platform and determining which validation method to use.
    ///
    /// # Returns
    /// - `Ok(SimpleConsensusValidationResult)` if all the keywords pass validation:
    ///   - No more than 50 keywords
    ///   - Each keyword is between 3 and 50 characters (if `validate_structure` is true)
    ///   - Each keyword contains no control or whitespace characters (if `validate_structure` is true)
    ///   - All keywords are unique
    /// - `Err(ProtocolError)` if an unknown or unsupported platform version is provided.
    ///
    /// # Errors
    /// - Returns a `ProtocolError::UnknownVersionMismatch` if the platform version is not recognized.
    /// - Returns validation errors for:
    ///   - Too many keywords (`TooManyKeywordsError`)
    ///   - Invalid keyword length (`InvalidKeywordLengthError`) - only if `validate_structure` is true
    ///   - Invalid keyword characters (`InvalidKeywordCharacterError`) - only if `validate_structure` is true
    ///   - Duplicate keywords (`DuplicateKeywordsError`)
    pub fn validate_keywords(
        contract_id: Identifier,
        keywords: &[String],
        validate_structure: bool,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_keywords
        {
            0 => Self::validate_keywords_v0(contract_id, keywords, validate_structure),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_keywords".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
