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
        // TODO: it must validate when we update documents and defs

    }
}
