use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::enrich_with_base_schema::PREFIX_BYTE_0;
use crate::data_contract::validation::data_contract_validation::BASE_DOCUMENT_SCHEMA;
use crate::data_contract::validation::multi_validator;
use crate::data_contract::validation::multi_validator::{
    byte_array_has_no_items_as_parent_validator, pattern_is_valid_regex_validator,
};
use crate::data_contract::validation::validate_data_contract_max_depth::validate_data_contract_max_depth;
use crate::prelude::DataContract;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::PlatformVersion;
use crate::{Convertible, ProtocolError};

impl DataContract {
    pub fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::default();
        let raw_data_contract = self.clone().into_object()?;

        result.merge(validate_data_contract_max_depth(&raw_data_contract));
        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(multi_validator::validate(
            &raw_data_contract,
            &[
                pattern_is_valid_regex_validator,
                byte_array_has_no_items_as_parent_validator,
            ],
        ));
        if !result.is_valid() {
            return Ok(result);
        }

        let enriched_data_contract =
            self.enrich_with_base_schema(&BASE_DOCUMENT_SCHEMA, PREFIX_BYTE_0, &[])?;

        for (_, document_schema) in enriched_data_contract.documents()?.iter() {
            let json_schema_validation_result =
                JsonSchemaValidator::validate_schema(document_schema, platform_version)?;
            result.merge(json_schema_validation_result);
        }
        if !result.is_valid() {
            return Ok(result);
        }

        for (document_name, document_type) in self.document_types().iter() {
            if document_type.indices().is_empty() {
                continue;
            }

            let document_schema = self.documents()?.get(document_name).ok_or(
                ProtocolError::CorruptedCodeExecution(
                    "there should be a document schema".to_string(),
                ),
            )?;

            //todo: re-enable
            // let validation_result = DataContract::validate_index_naming_duplicates(
            //     &document_type.indices(),
            //     document_name,
            // );
            // result.merge(validation_result);
            //
            // let validation_result =
            //     DataContract::validate_max_unique_indices(&document_type.indices(), document_name);
            // result.merge(validation_result);
            //
            // let (validation_result, should_stop_further_validation) =
            //     DataContract::validate_index_definitions(
            //         &document_type.indices(),
            //         document_name,
            //         document_schema,
            //     );
            // result.merge(validation_result);
            // if should_stop_further_validation {
            //     return Ok(result);
            // }
        }

        Ok(result)
    }
}
