use crate::data_contract::enrich_with_base_schema::PREFIX_BYTE_0;
use crate::data_contract::validation::data_contract_validation::BASE_DOCUMENT_SCHEMA;
use crate::data_contract::validation::multi_validator;
use crate::data_contract::validation::multi_validator::{
    byte_array_has_no_items_as_parent_validator, pattern_is_valid_regex_validator,
};
use crate::data_contract::validation::validate_data_contract_max_depth::validate_data_contract_max_depth;
use crate::data_contract::DataContract;
use crate::prelude::ConsensusValidationResult;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use log::trace;

impl DataContract {
    /// Validate the data contract from a raw value
    pub(super) fn validate_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = ConsensusValidationResult::default();
        // trace!("validating against data contract meta validator");
        // result.merge(JsonSchemaValidator::validate_data_contract_schema(
        //     &raw_data_contract
        //         .try_to_validating_json()
        //         .map_err(ProtocolError::ValueError)?, platform_version,
        // )?);
        // if !result.is_valid() {
        //     return Ok(result);
        // }
        //
        // // todo: reenable version validation
        // // trace!("validating by protocol protocol version validator");
        // // result.merge(
        // //     self.protocol_version_validator.validate(
        // //         raw_data_contract
        // //             .get_integer("protocolVersion")
        // //             .map_err(ProtocolError::ValueError)?,
        // //     )?,
        // // );
        // // if !result.is_valid() {
        // //     return Ok(result);
        // // }
        //
        // trace!("validating data contract max depth");
        // result.merge(validate_data_contract_max_depth(raw_data_contract));
        // if !result.is_valid() {
        //     return Ok(result);
        // }
        //
        // trace!("validating data contract patterns & byteArray parents");
        // result.merge(multi_validator::validate(
        //     raw_data_contract,
        //     &[
        //         pattern_is_valid_regex_validator,
        //         byte_array_has_no_items_as_parent_validator,
        //     ],
        // ));
        // if !result.is_valid() {
        //     return Ok(result);
        // }
        //
        // let data_contract = Self::from_object(raw_data_contract.clone())?;
        // let enriched_data_contract =
        //     data_contract.enrich_with_base_schema(&BASE_DOCUMENT_SCHEMA, PREFIX_BYTE_0, &[])?;
        //
        // trace!("validating the documents");
        // for (document_type, document_schema) in enriched_data_contract.documents.iter() {
        //     trace!("validating document schema '{}'", document_type);
        //     let json_schema_validation_result =
        //         JsonSchemaValidator::validate_schema(document_schema, platform_version)?;
        //     result.merge(json_schema_validation_result);
        // }
        // if !result.is_valid() {
        //     return Ok(result);
        // }
        //
        // trace!("indices validation");
        // for (document_type, document_schema) in enriched_data_contract
        //     .documents
        //     .iter()
        //     .filter(|(_, value)| value.get("indices").is_some())
        // {
        //     trace!("validating indices in {}", document_type);
        //     let indices = document_schema.get_indices::<Vec<_>>()?;
        //
        //     trace!("\t validating duplicates");
        //     let validation_result = Self::validate_index_naming_duplicates(&indices, document_type);
        //     result.merge(validation_result);
        //
        //     trace!("\t validating uniqueness");
        //     let validation_result = Self::validate_max_unique_indices(&indices, document_type);
        //     result.merge(validation_result);
        //
        //     trace!("\t validating indices");
        //     let (validation_result, should_stop_further_validation) =
        //         Self::validate_index_definitions(&indices, document_type, document_schema);
        //     result.merge(validation_result);
        //     if should_stop_further_validation {
        //         return Ok(result);
        //     }
        // }

        Ok(result)
    }
}
