use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::document_type::schema::validation_schema::{
    native_validator_of, DocumentSchemaValidator,
};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0Getters};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

#[cfg(test)]
mod tests;

impl DataContract {
    /// Generation 1's checks ([`Self::validate_document_properties_v1_checks`]), with the
    /// document type's schema checked by its native validator
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
        let validator = match self.native_validator(&document_type, platform_version)? {
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

        Self::validate_document_properties_v1_checks(
            document_type,
            value,
            system,
            platform_version,
            |json_value| {
                Ok(match validator.validate(json_value)? {
                    Some(violation) => SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::JsonSchemaError(violation)),
                    ),
                    None => SimpleConsensusValidationResult::new(),
                })
            },
        )
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

    /// The native validator of the document type `name`, built on first use and kept on the
    /// document type, or the refusal of its schema, kept the same way
    /// ([`native_validator_of`]): a type read back from its stored form is checked against the
    /// typed validation schema stored with it, by the generation that built it, or refused when
    /// it was stored without one; any other against the one the builder `platform_version`
    /// selects builds from its schema.
    fn native_validator<'v>(
        &self,
        document_type: &'v DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Result<&'v DocumentSchemaValidator, &'v DataContractError>, ProtocolError> {
        native_validator_of(
            document_type.json_schema_validator_ref(),
            document_type.schema(),
            self.schema_defs(),
            platform_version,
        )
    }
}
