use crate::consensus::basic::data_contract::{
    DuplicateKeywordsError, InvalidKeywordCharacterError, InvalidKeywordLengthError,
    TooManyKeywordsError,
};
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::HashSet;

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_keywords_v0(
        contract_id: Identifier,
        keywords: &[String],
        validate_structure: bool,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Validate there are no more than 50 keywords
        if keywords.len() > 50 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                TooManyKeywordsError::new(contract_id, keywords.len() as u8).into(),
            ));
        }

        // Validate the keywords are all unique and between 3 and 50 characters
        let mut seen_keywords = HashSet::new();
        for keyword in keywords {
            if validate_structure {
                // Check keyword length
                if keyword.len() < 3 || keyword.len() > 50 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidKeywordLengthError::new(contract_id, keyword.to_string()).into(),
                    ));
                }

                if !keyword
                    .chars()
                    .all(|c| !c.is_control() && !c.is_whitespace())
                {
                    // This would mean we have an invalid character
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidKeywordCharacterError::new(contract_id, keyword.to_string()).into(),
                    ));
                }
            }

            // Check uniqueness (always done)
            if !seen_keywords.insert(keyword) {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DuplicateKeywordsError::new(contract_id, keyword.to_string()).into(),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
