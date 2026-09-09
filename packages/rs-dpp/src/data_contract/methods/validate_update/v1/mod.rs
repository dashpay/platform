use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Generation 1 (protocol version 14, `requiredSince`): generation 0
    /// plus validation of `requiredSince` annotations on document types
    /// introduced by the update, which have no old counterpart for the
    /// per-type pass to see. (Required-set changes on *existing* document
    /// types are judged inside the shared per-type dispatcher, which
    /// resolves its own generation from the platform version, so this
    /// method needs no logic of its own for them.)
    ///
    /// Delegating to generation 0 is safe because that generation is
    /// shipped and therefore frozen. The checks are independent and
    /// short-circuiting, so appending the extra one changes only which
    /// error is reported when an update violates several rules at once —
    /// never whether it is rejected.
    #[inline(always)]
    pub(super) fn validate_update_v1(
        &self,
        new_data_contract: &DataContract,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let result = self.validate_update_v0(new_data_contract, block_info, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(self.validate_update_new_document_types_required_since(new_data_contract))
    }

    /// Document types introduced by this update have no old counterpart,
    /// so the per-type update validation never sees them. Their
    /// `requiredSince` annotations must name the version this update
    /// creates — anything else would pre-schedule (or backdate) a
    /// wire-layout change without validation.
    fn validate_update_new_document_types_required_since(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        for (document_type_name, new_document_type) in new_data_contract.document_types() {
            if self
                .document_type_optional_for_name(document_type_name)
                .is_some()
            {
                continue;
            }
            for (property_name, property) in new_document_type.as_ref().properties() {
                if let Some(required_since) = property.required_since {
                    if required_since != new_data_contract.version() {
                        return SimpleConsensusValidationResult::new_with_error(
                            DataContractInvalidRequiredFieldsUpdateError::new(
                                document_type_name.clone(),
                                format!(
                                    "new document type property '{property_name}' must carry requiredSince {}, the contract version this update creates",
                                    new_data_contract.version()
                                ),
                            )
                            .into(),
                        );
                    }
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::basic_error::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Setters;
    use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
    use crate::data_contract::schema::DataContractSchemaMethodsV0;
    use crate::prelude::IdentityNonce;
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use platform_value::platform_value;

    #[test]
    fn should_validate_required_since_on_document_types_added_by_the_update() {
        let platform_version = PlatformVersion::latest();

        let old_data_contract = get_data_contract_fixture(
            None,
            IdentityNonce::default(),
            platform_version.protocol_version,
        )
        .data_contract_owned();

        let new_type_schema = |required_since: u32| {
            platform_value!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "position": 0,
                        "maxLength": 60_u32,
                        "requiredSince": required_since,
                    }
                },
                "required": ["message"],
                "additionalProperties": false
            })
        };

        // A new document type pre-scheduling requiredness at version 99
        // has no old counterpart, so the per-type update validation
        // never runs on it — this pass must catch it
        let mut new_data_contract = old_data_contract.clone();
        new_data_contract.set_version(old_data_contract.version() + 1);
        new_data_contract
            .set_document_schema(
                "note",
                new_type_schema(99),
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("should add document type");

        let result = old_data_contract
            .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
            .expect("failed validate update");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
            )] if e.details().contains("must carry requiredSince 2")
        );

        // The same new document type annotated with the version this
        // update creates is accepted
        let mut new_data_contract = old_data_contract.clone();
        new_data_contract.set_version(old_data_contract.version() + 1);
        new_data_contract
            .set_document_schema(
                "note",
                new_type_schema(old_data_contract.version() + 1),
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("should add document type");

        let result = old_data_contract
            .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
            .expect("failed validate update");

        assert!(
            result.is_valid(),
            "a new document type annotated with the version this update \
             creates must be accepted, got {:?}",
            result.errors
        );
    }
}
