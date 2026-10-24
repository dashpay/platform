use crate::consensus::basic::document::{
    DocumentFieldMaxSizeExceededError, InvalidDocumentTypeError,
};
use crate::consensus::basic::value_error::ValueError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::document_type::schema::validation_schema::DocumentSchemaValidator;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0Getters};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Generation 1, with the document type's schema checked by its native validator
    /// ([`DocumentSchemaValidator`]) in place of the `jsonschema` crate, which reaches the
    /// same verdicts but for one rule of its own: in a `uniqueItems` list, `0.0` and `-0.0`
    /// are one value however long the list. A document type whose schema has no validation
    /// schema has its documents refused (`InvalidContractStructure`) before any other check:
    /// the `jsonschema` crate checks no document at this generation.
    #[inline(always)]
    pub(super) fn validate_document_properties_v2(
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
        let validator = match self.native_validator(name, &document_type, platform_version)? {
            Ok(validator) => validator,
            Err(refusal) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::ContractError(
                        DataContractError::InvalidContractStructure(format!(
                            "document type {name} has no validation schema, which its \
                             documents are checked against: {refusal}"
                        )),
                    )),
                ))
            }
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

        if let Some(violation) = validator.validate(&json_value)? {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::JsonSchemaError(violation)),
            ));
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
    pub(super) fn validate_document_v2(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        self.validate_document_properties_v2(
            name,
            document.properties().into(),
            &DocumentSystemValues::of_document(document),
            platform_version,
        )
    }

    /// The native validator of the document type `name`, built on first use and kept on
    /// the document type, or the refusal of its schema, kept the same way: the
    /// [`DataContractError`] building the validation schema or its patterns returns. Any
    /// other error is returned as it is, and not kept. Both are lent from the document type.
    fn native_validator<'v>(
        &self,
        name: &str,
        document_type: &'v DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Result<&'v DocumentSchemaValidator, &'v DataContractError>, ProtocolError> {
        let slot = document_type.json_schema_validator_ref().native();
        let built = match slot.get() {
            Some(built) => built,
            None => {
                let validator = match self.document_type_validation_schema(name, platform_version) {
                    Ok(schema) => DocumentSchemaValidator::new(schema),
                    Err(ProtocolError::DataContractError(refusal)) => Err(refusal),
                    Err(error) => return Err(error),
                };
                // Building is deterministic: a build that finished first holds the same result
                slot.get_or_init(|| validator)
            }
        };
        Ok(built.as_ref())
    }
}
