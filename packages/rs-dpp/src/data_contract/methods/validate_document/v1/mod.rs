use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::document_type::DocumentType;

use crate::consensus::basic::document::{
    DocumentFieldMaxSizeExceededError, InvalidDocumentTypeError,
};
use crate::consensus::basic::value_error::ValueError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0Getters};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::ops::Deref;

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_document_properties_v1(
        &self,
        name: &str,
        value: Value,
        system: &DocumentSystemValues,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let Some(document_type) = self.document_type_optional_for_name(name) else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(name.to_owned(), self.id()).into(),
            ));
        };

        if let Some(max_depth) = platform_version.system_limits.max_document_value_depth {
            let max_depth = max_depth as usize;
            // The enclosing properties map mirrors the transition's plain `BTreeMap` data
            // wrapper, which the wire decoder never counts: each property value receives the
            // full depth budget so no decodable payload can violate this rule.
            let excess_depth = match &value {
                Value::Map(map) => map.iter().find_map(|(key, property_value)| {
                    key.first_depth_exceeding(max_depth)
                        .or_else(|| property_value.first_depth_exceeding(max_depth))
                }),
                other => other.first_depth_exceeding(max_depth),
            };
            if let Some(actual_depth) = excess_depth {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::ValueError(
                        ValueError::new_from_string(format!(
                            "document value depth {actual_depth} exceeds system maximum {max_depth}"
                        )),
                    )),
                ));
            }
        }

        // Path readers choose the first copy of a key, while JSON and stored objects
        // choose the last. Refuse ambiguity before either view can validate the document.
        if value.first_repeated_text_key().is_some() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::ValueError(ValueError::new_from_string(
                    "document properties contain a repeated map key".to_owned(),
                ))),
            ));
        }

        let validator = document_type.json_schema_validator_ref().deref();

        if let Some((key, size)) =
            value.has_data_larger_than(platform_version.system_limits.max_field_value_size)
        {
            let field = match key {
                Some(Value::Text(field)) => field.clone(),
                _ => "".to_string(),
            };
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::DocumentFieldMaxSizeExceededError(
                    DocumentFieldMaxSizeExceededError::new(
                        field,
                        size as u64,
                        platform_version.system_limits.max_field_value_size as u64,
                    ),
                )),
            ));
        }

        // Compute path constraints before JSON conversion consumes the value, and report
        // them after schema validation so their inputs are known to have the required types.
        let max_bytes_result =
            document_type.validate_max_bytes_properties(&value, platform_version)?;

        let generated_from_result =
            document_type.validate_generated_from_properties(&value, platform_version)?;

        let property_constraints_result =
            document_type.validate_property_constraints(&value, system, platform_version)?;

        let json_value = match value.try_into_validating_json() {
            Ok(json_value) => json_value,
            Err(e) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::ValueError(e.into())),
                ))
            }
        };

        // Compile json schema validator if it's not yet compiled
        let schema_result = if !validator.is_compiled(platform_version)? {
            // It is normal that we get a protocol error here, since the document type is coming
            // from the state
            let root_schema = DocumentType::enrich_with_base_schema(
                document_type.schema().clone(),
                self.schema_defs().map(|defs| Value::from(defs.clone())),
                platform_version,
            )?;
            // The schema is held as the contract was sent; JSON Schema knows no
            // `identifier` or `bytes` type, so the validator compiles the long
            // form a shorthand stands for, as the parse compiled it. The
            // contract passed the expansion when it was registered.
            let root_schema = DocumentType::expand_property_type_shorthands(
                &root_schema,
                false,
                platform_version,
            )
            .map_err(ProtocolError::DataContractError)?
            .unwrap_or(root_schema);

            let root_json_schema = root_schema
                .try_to_validating_json()
                .map_err(ProtocolError::ValueError)?;

            validator.compile_and_validate(&root_json_schema, &json_value, platform_version)?
        } else {
            validator.validate(&json_value, platform_version)?
        };
        if !schema_result.is_valid() {
            return Ok(schema_result);
        }
        if !max_bytes_result.is_valid() {
            return Ok(max_bytes_result);
        }
        if !generated_from_result.is_valid() {
            return Ok(generated_from_result);
        }

        Ok(property_constraints_result)
    }

    #[inline(always)]
    pub(super) fn validate_document_v1(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Validate user defined properties
        self.validate_document_properties_v1(
            name,
            document.properties().into(),
            &DocumentSystemValues::of_document(document),
            platform_version,
        )
    }
}
