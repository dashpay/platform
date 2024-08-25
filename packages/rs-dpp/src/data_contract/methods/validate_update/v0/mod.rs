use crate::data_contract::accessors::v0::DataContractV0Getters;

use crate::consensus::basic::data_contract::{
    IncompatibleDataContractSchemaError, InvalidDataContractVersionError,
};
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::schema::validate_schema_compatibility;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use serde_json::json;

pub trait DataContractUpdateValidationMethodsV0 {
    fn validate_update(
        &self,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_update_v0(
        &self,
        new_data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Check if the contract is owned by the same identity
        if self.owner_id() != new_data_contract.owner_id() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DataContractUpdatePermissionError::new(self.id(), new_data_contract.owner_id())
                    .into(),
            ));
        }

        // Check version is bumped
        // Failure (version != previous version + 1): Keep ST and transform it to a nonce bump action.
        // How: A user pushed an update that was not the next version.

        let new_version = new_data_contract.version();
        let old_version = self.version();
        if new_version < old_version || new_version - old_version != 1 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDataContractVersionError::new(old_version + 1, new_version).into(),
            ));
        }

        // Validate that the config was not updated
        // * Includes verifications that:
        //     - Old contract is not read_only
        //     - New contract is not read_only
        //     - Keeps history did not change
        //     - Can be deleted did not change
        //     - Documents keep history did not change
        //     - Documents can be deleted contract default did not change
        //     - Documents mutable contract default did not change
        //     - Requires identity encryption bounded key did not change
        //     - Requires identity decryption bounded key did not change
        // * Failure (contract does not exist): Keep ST and transform it to a nonce bump action.
        // * How: A user pushed an update to a contract that changed its configuration.

        let config_validation_result = self.config().validate_update(
            new_data_contract.config(),
            self.id(),
            platform_version,
        )?;

        if !config_validation_result.is_valid() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                config_validation_result.errors,
            ));
        }

        // Validate updates for existing document types to make sure that previously created
        // documents will be still valid with a new version of the data contract
        for (document_type_name, old_document_type) in self.document_types() {
            // Make sure that existing document aren't removed
            let Some(new_document_type) =
                new_data_contract.document_type_optional_for_name(document_type_name)
            else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.id(),
                        document_type_name,
                        "document type can't be removed",
                    )
                    .into(),
                ));
            };

            // Validate document type update rules
            let validate_update_result = old_document_type
                .as_ref()
                .validate_update(new_document_type, platform_version)?;

            if !validate_update_result.is_valid() {
                return Ok(SimpleConsensusValidationResult::new_with_errors(
                    validate_update_result.errors,
                ));
            }
        }

        // Schema $defs should be compatible
        if let Some(old_defs_map) = self.schema_defs() {
            // If new contract doesn't have $defs, it means that it's $defs was removed and compatibility is broken
            let Some(new_defs_map) = new_data_contract.schema_defs() else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    IncompatibleDataContractSchemaError::new(
                        self.id(),
                        "remove".to_string(),
                        "/$defs".to_string(),
                    )
                    .into(),
                ));
            };

            // If $defs is updated we need to make sure that our data contract is still compatible
            // with previously created data
            if old_defs_map != new_defs_map {
                // both new and old $defs already validated as a part of new and old contract
                let old_defs_json = Value::from(old_defs_map)
                    .try_into_validating_json()
                    .map_err(ProtocolError::ValueError)?;

                let new_defs_json = Value::from(new_defs_map)
                    .try_into_validating_json()
                    .map_err(ProtocolError::ValueError)?;

                let old_defs_schema = json!({
                    "$defs": old_defs_json
                });

                let new_defs_schema = json!({
                    "$defs": new_defs_json
                });

                // We do not allow to remove or modify $ref in document type schemas
                // it means that compatible changes in $defs won't break the overall compatibility
                // Make sure that updated $defs schema is compatible
                let compatibility_validation_result = validate_schema_compatibility(
                    &old_defs_schema,
                    &new_defs_schema,
                    platform_version,
                )?;

                if !compatibility_validation_result.is_valid() {
                    let errors = compatibility_validation_result
                        .errors
                        .into_iter()
                        .map(|operation| {
                            IncompatibleDataContractSchemaError::new(
                                self.id(),
                                operation.name,
                                operation.path,
                            )
                            .into()
                        })
                        .collect();

                    return Ok(SimpleConsensusValidationResult::new_with_errors(errors));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::basic_error::BasicError;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::v0::DataContractConfigSettersV0;
    use crate::prelude::IdentityNonce;
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use platform_value::platform_value;
    use platform_value::Identifier;

    mod validate_update {
        use super::*;
        use crate::data_contract::accessors::v0::DataContractV0Setters;
        use crate::data_contract::document_type::DocumentTypeMutRef;

        #[test]
        fn should_return_invalid_result_if_owner_id_is_not_the_same() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.as_v0_mut().unwrap().owner_id = Identifier::random();

            let result = old_data_contract
                .validate_update(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractUpdatePermissionError(e)
                )] if *e.data_contract_id() == old_data_contract.id() && *e.identity_id() == new_data_contract.owner_id()
            );
        }

        #[test]
        fn should_return_invalid_result_if_contract_version_is_not_greater_for_one() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let new_data_contract = old_data_contract.clone();

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidDataContractVersionError(e)
                )] if e.expected_version() == old_data_contract.version() + 1 && e.version() == new_data_contract.version()
            );
        }

        #[test]
        fn should_return_invalid_result_if_config_was_updated() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract.config_mut().set_readonly(true);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if e.additional_message() == "contract can not be changed to readonly"
            );
        }

        #[test]
        fn should_return_invalid_result_when_document_type_is_removed() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract
                .document_types_mut()
                .remove("niceDocument");

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can't be removed"
            );
        }

        #[test]
        fn should_return_invalid_result_when_document_type_has_incompatible_change() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);

            let DocumentTypeMutRef::V0(new_document_type) = new_data_contract
                .document_types_mut()
                .get_mut("niceDocument")
                .unwrap()
                .as_mut_ref();
            new_document_type.documents_mutable = false;

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether its documents are mutable: changing from true to false"
            );
        }

        #[test]
        fn should_return_invalid_result_when_defs_is_removed() {
            let platform_version = PlatformVersion::latest();

            let mut old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            // Remove document that uses $defs, so we can safely remove it for testing
            old_data_contract
                .document_types_mut()
                .remove("prettyDocument");

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract
                .set_schema_defs(None, false, &mut Vec::new(), platform_version)
                .expect("failed to set schema defs");

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDataContractSchemaError(e)
                )] if e.operation() == "remove" && e.field_path() == "/$defs"
            );
        }

        #[test]
        fn should_return_invalid_result_when_updated_defs_is_incompatible() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            let incompatible_defs_value = platform_value!({
                "lastName": {
                    "type": "number",
                },
            });
            let incompatible_defs = incompatible_defs_value
                .into_btree_string_map()
                .expect("should convert to map");

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract
                .set_schema_defs(
                    Some(incompatible_defs),
                    false,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("failed to set schema defs");

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDataContractSchemaError(e)
                )] if e.operation() == "replace" && e.field_path() == "/$defs/lastName/type"
            );
        }

        #[test]
        fn should_pass_when_all_changes_are_compatible() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, platform_version)
                .expect("failed validate update");

            assert!(result.is_valid());
        }
    }
}
