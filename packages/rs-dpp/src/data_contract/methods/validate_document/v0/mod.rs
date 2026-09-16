use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
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

pub trait DataContractDocumentValidationMethodsV0 {
    fn validate_document(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;

    fn validate_document_properties(
        &self,
        name: &str,
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_document_properties_v0(
        &self,
        name: &str,
        value: Value,
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

        let json_value = match value.try_into_validating_json() {
            Ok(json_value) => json_value,
            Err(e) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::ValueError(e.into())),
                ))
            }
        };

        // Compile json schema validator if it's not yet compiled
        if !validator.is_compiled(platform_version)? {
            // It is normal that we get a protocol error here, since the document type is coming
            // from the state
            let root_schema = DocumentType::enrich_with_base_schema(
                // TODO: I just wondering if we could you references here
                //  instead of cloning
                document_type.schema().clone(),
                self.schema_defs().map(|defs| Value::from(defs.clone())),
                platform_version,
            )?;

            let root_json_schema = root_schema
                .try_to_validating_json()
                .map_err(ProtocolError::ValueError)?;

            validator.compile_and_validate(&root_json_schema, &json_value, platform_version)
        } else {
            validator.validate(&json_value, platform_version)
        }
    }

    #[inline(always)]
    pub(super) fn validate_document_v0(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Validate user defined properties
        self.validate_document_properties_v0(name, document.properties().into(), platform_version)
    }
}

#[cfg(all(test, feature = "fixtures-and-mocks"))]
mod tests {
    use super::DataContractDocumentValidationMethodsV0;
    use crate::consensus::basic::value_error::ValueError;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::created_data_contract::CreatedDataContract;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_value::Value;
    use platform_version::version::PlatformVersion;

    fn data_contract() -> CreatedDataContract {
        let platform_version = PlatformVersion::latest();
        get_data_contract_fixture(None, 0, platform_version.protocol_version)
    }

    fn nested_document_value(container_count: usize, leaf: Value) -> Value {
        let nested = (0..container_count).fold(leaf, |value, depth| {
            if depth % 2 == 0 {
                Value::Array(vec![value])
            } else {
                Value::Map(vec![(Value::Text("nested".to_owned()), value)])
            }
        });

        Value::Map(vec![(Value::Text("name".to_owned()), nested)])
    }

    #[test]
    fn should_reject_excessive_document_value_depth_before_field_size_validation() {
        let platform_version = PlatformVersion::latest();
        let data_contract = data_contract().data_contract_owned();
        let max_depth = platform_version
            .system_limits
            .max_document_value_depth
            .expect("latest protocol should enforce document value depth");
        let value = nested_document_value(
            max_depth as usize + 1,
            Value::Text(
                "x".repeat(platform_version.system_limits.max_field_value_size as usize + 1),
            ),
        );

        let result = data_contract
            .validate_document_properties("noTimeDocument", value, platform_version)
            .expect("validation should return a consensus result");

        let Some(ConsensusError::BasicError(BasicError::ValueError(ValueError { .. }))) =
            result.first_error()
        else {
            panic!("expected document value depth error, got {result:?}");
        };
        assert_eq!(
            result.first_error().expect("expected an error").to_string(),
            format!(
                "document value depth {} exceeds system maximum {max_depth}",
                max_depth + 1
            )
        );
    }

    #[test]
    fn should_allow_document_value_depth_at_the_limit() {
        let platform_version = PlatformVersion::latest();
        let data_contract = data_contract().data_contract_owned();
        let max_depth = platform_version
            .system_limits
            .max_document_value_depth
            .expect("latest protocol should enforce document value depth");
        let value = nested_document_value(
            max_depth as usize,
            Value::Text(
                "x".repeat(platform_version.system_limits.max_field_value_size as usize + 1),
            ),
        );

        let result = data_contract
            .validate_document_properties("noTimeDocument", value, platform_version)
            .expect("validation should return a consensus result");

        assert!(matches!(
            result.first_error(),
            Some(ConsensusError::BasicError(
                BasicError::DocumentFieldMaxSizeExceededError(_)
            ))
        ));
    }

    #[test]
    fn should_preserve_valid_document_properties() {
        let platform_version = PlatformVersion::latest();
        let data_contract = data_contract().data_contract_owned();
        let value = Value::Map(vec![(
            Value::Text("name".to_owned()),
            Value::Text("Alice".to_owned()),
        )]);

        let result = data_contract
            .validate_document_properties("noTimeDocument", value, platform_version)
            .expect("validation should return a consensus result");

        assert!(
            result.is_valid(),
            "expected valid properties, got {result:?}"
        );
    }
}
