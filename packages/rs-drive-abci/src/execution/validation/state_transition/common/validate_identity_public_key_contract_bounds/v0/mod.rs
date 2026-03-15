use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::consensus::basic::document::{
    DataContractNotPresentError, InvalidDocumentTypeError,
};
use dpp::consensus::basic::identity::{DataContractBoundsNotPresentError, InvalidKeyPurposeForContractBoundsError};
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::identity::identity_public_key_already_exists_for_unique_contract_bounds_error::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use dpp::identifier::Identifier;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Purpose::{DECRYPTION, ENCRYPTION};
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyKindRequestType, KeyRequestType, OptionalSingleIdentityPublicKeyOutcome};
use drive::grovedb::TransactionArg;

pub(super) fn validate_identity_public_keys_contract_bounds_v0(
    identity_id: Identifier,
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let consensus_validation_results = identity_public_keys_with_witness
        .iter()
        .map(|identity_public_key| {
            validate_identity_public_key_contract_bounds_v0(
                identity_id,
                identity_public_key,
                drive,
                transaction,
                execution_context,
                platform_version,
            )
        })
        .collect::<Result<Vec<SimpleConsensusValidationResult>, Error>>()?;
    Ok(SimpleConsensusValidationResult::merge_many_errors(
        consensus_validation_results,
    ))
}

fn validate_identity_public_key_contract_bounds_v0(
    identity_id: Identifier,
    identity_public_key_in_creation: &IdentityPublicKeyInCreation,
    drive: &Drive,
    transaction: TransactionArg,
    _execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    //todo: we should add to the execution context the cost of fetching contracts
    let purpose = identity_public_key_in_creation.purpose();
    if let Some(contract_bounds) = identity_public_key_in_creation.contract_bounds() {
        match contract_bounds {
            ContractBounds::SingleContract { id: contract_id } => {
                // we should fetch the contract
                let contract = drive.get_contract_with_fetch_info(
                    contract_id.to_buffer(),
                    false,
                    transaction,
                    platform_version,
                )?;
                match contract {
                    None => Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::DataContractNotPresentError(
                            DataContractNotPresentError::new(*contract_id),
                        )),
                    )),
                    Some(contract) => {
                        match purpose {
                            ENCRYPTION => {
                                let Some(requirements) = contract
                                    .contract
                                    .config()
                                    .requires_identity_encryption_bounded_key()
                                else {
                                    return Ok(SimpleConsensusValidationResult::new_with_error(
                                        ConsensusError::BasicError(
                                            BasicError::DataContractBoundsNotPresentError(
                                                DataContractBoundsNotPresentError::new(
                                                    *contract_id,
                                                ),
                                            ),
                                        ),
                                    ));
                                };

                                match requirements {
                                    // We should make sure no other key exists for these bounds
                                    StorageKeyRequirements::Unique => {
                                        let key_request = IdentityKeysRequest {
                                            identity_id: identity_id.to_buffer(),
                                            request_type: KeyRequestType::ContractBoundKey(
                                                contract_id.to_buffer(),
                                                purpose,
                                                KeyKindRequestType::CurrentKeyOfKindRequest,
                                            ),
                                            limit: None,
                                            offset: None,
                                        };
                                        let maybe_conflicting_key = drive.fetch_identity_keys::<OptionalSingleIdentityPublicKeyOutcome>(key_request, transaction, platform_version)?;
                                        if let Some(conflicting_key) = maybe_conflicting_key {
                                            Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(StateError::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError::new(identity_id, *contract_id, purpose, identity_public_key_in_creation.id(), conflicting_key.id())))))
                                        } else {
                                            Ok(SimpleConsensusValidationResult::new())
                                        }
                                    }
                                    StorageKeyRequirements::Multiple
                                    | StorageKeyRequirements::MultipleReferenceToLatest => {
                                        Ok(SimpleConsensusValidationResult::new())
                                    }
                                }
                            }
                            DECRYPTION => {
                                let Some(requirements) = contract
                                    .contract
                                    .config()
                                    .requires_identity_decryption_bounded_key()
                                else {
                                    return Ok(SimpleConsensusValidationResult::new_with_error(
                                        ConsensusError::BasicError(
                                            BasicError::DataContractBoundsNotPresentError(
                                                DataContractBoundsNotPresentError::new(
                                                    *contract_id,
                                                ),
                                            ),
                                        ),
                                    ));
                                };

                                match requirements {
                                    StorageKeyRequirements::Unique => {
                                        // We should make sure no other key exists for these bounds
                                        let key_request = IdentityKeysRequest {
                                            identity_id: identity_id.to_buffer(),
                                            request_type: KeyRequestType::ContractBoundKey(
                                                contract_id.to_buffer(),
                                                purpose,
                                                KeyKindRequestType::CurrentKeyOfKindRequest,
                                            ),
                                            limit: None,
                                            offset: None,
                                        };
                                        let maybe_conflicting_key = drive.fetch_identity_keys::<OptionalSingleIdentityPublicKeyOutcome>(key_request, transaction, platform_version)?;
                                        if let Some(conflicting_key) = maybe_conflicting_key {
                                            Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(StateError::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError::new(identity_id, *contract_id, purpose, identity_public_key_in_creation.id(), conflicting_key.id())))))
                                        } else {
                                            Ok(SimpleConsensusValidationResult::new())
                                        }
                                    }
                                    StorageKeyRequirements::Multiple
                                    | StorageKeyRequirements::MultipleReferenceToLatest => {
                                        Ok(SimpleConsensusValidationResult::new())
                                    }
                                }
                            }
                            purpose => Ok(SimpleConsensusValidationResult::new_with_error(
                                ConsensusError::BasicError(
                                    BasicError::InvalidKeyPurposeForContractBoundsError(
                                        InvalidKeyPurposeForContractBoundsError::new(
                                            purpose,
                                            vec![ENCRYPTION, DECRYPTION],
                                        ),
                                    ),
                                ),
                            )),
                        }
                    }
                }
            }
            ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name,
            } => {
                let contract = drive.get_contract_with_fetch_info(
                    contract_id.to_buffer(),
                    false,
                    transaction,
                    platform_version,
                )?;
                match contract {
                    None => Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::DataContractNotPresentError(
                            DataContractNotPresentError::new(*contract_id),
                        )),
                    )),
                    Some(contract) => {
                        let document_type = contract
                            .contract
                            .document_type_optional_for_name(document_type_name.as_str());
                        match document_type {
                            None => Ok(SimpleConsensusValidationResult::new_with_error(
                                ConsensusError::BasicError(BasicError::InvalidDocumentTypeError(
                                    InvalidDocumentTypeError::new(
                                        document_type_name.clone(),
                                        *contract_id,
                                    ),
                                )),
                            )),
                            Some(document_type) => {
                                match purpose {
                                    ENCRYPTION => {
                                        let Some(requirements) = document_type
                                            .requires_identity_encryption_bounded_key()
                                        else {
                                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                                    ConsensusError::BasicError(
                                                        BasicError::DataContractBoundsNotPresentError(
                                                            DataContractBoundsNotPresentError::new(*contract_id),
                                                        ),
                                                    ),
                                                ));
                                        };

                                        match requirements {
                                            StorageKeyRequirements::Unique => {
                                                // We should make sure no other key exists for these bounds
                                                let key_request = IdentityKeysRequest {
                                                    identity_id: identity_id.to_buffer(),
                                                    request_type: KeyRequestType::ContractDocumentTypeBoundKey(contract_id.to_buffer(), document_type_name.clone(), purpose, KeyKindRequestType::CurrentKeyOfKindRequest),
                                                    limit: None,
                                                    offset: None,
                                                };
                                                let maybe_conflicting_key = drive.fetch_identity_keys::<OptionalSingleIdentityPublicKeyOutcome>(key_request, transaction, platform_version)?;
                                                if let Some(conflicting_key) = maybe_conflicting_key
                                                {
                                                    Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(StateError::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError::new(identity_id, *contract_id, purpose, identity_public_key_in_creation.id(), conflicting_key.id())))))
                                                } else {
                                                    Ok(SimpleConsensusValidationResult::new())
                                                }
                                            }
                                            StorageKeyRequirements::Multiple
                                            | StorageKeyRequirements::MultipleReferenceToLatest => {
                                                Ok(SimpleConsensusValidationResult::new())
                                            }
                                        }
                                    }
                                    DECRYPTION => {
                                        let Some(requirements) = document_type
                                            .requires_identity_encryption_bounded_key()
                                        else {
                                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                                    ConsensusError::BasicError(
                                                        BasicError::DataContractBoundsNotPresentError(
                                                            DataContractBoundsNotPresentError::new(*contract_id),
                                                        ),
                                                    ),
                                                ));
                                        };

                                        match requirements {
                                            StorageKeyRequirements::Unique => {
                                                let key_request = IdentityKeysRequest {
                                                    identity_id: identity_id.to_buffer(),
                                                    request_type: KeyRequestType::ContractDocumentTypeBoundKey(contract_id.to_buffer(), document_type_name.clone(), purpose, KeyKindRequestType::CurrentKeyOfKindRequest),
                                                    limit: None,
                                                    offset: None,
                                                };
                                                let maybe_conflicting_key = drive.fetch_identity_keys::<OptionalSingleIdentityPublicKeyOutcome>(key_request, transaction, platform_version)?;
                                                if let Some(conflicting_key) = maybe_conflicting_key
                                                {
                                                    Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(StateError::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError::new(identity_id, *contract_id, purpose, identity_public_key_in_creation.id(), conflicting_key.id())))))
                                                } else {
                                                    Ok(SimpleConsensusValidationResult::new())
                                                }
                                            }
                                            StorageKeyRequirements::Multiple
                                            | StorageKeyRequirements::MultipleReferenceToLatest => {
                                                Ok(SimpleConsensusValidationResult::new())
                                            }
                                        }
                                    }
                                    _ => Ok(SimpleConsensusValidationResult::new_with_error(
                                        ConsensusError::BasicError(
                                            BasicError::InvalidKeyPurposeForContractBoundsError(
                                                InvalidKeyPurposeForContractBoundsError::new(
                                                    purpose,
                                                    vec![ENCRYPTION, DECRYPTION],
                                                ),
                                            ),
                                        ),
                                    )),
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::version::DefaultForPlatformVersion;

    fn make_key_in_creation(
        id: u32,
        purpose: Purpose,
        contract_bounds: Option<ContractBounds>,
    ) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV0 {
            id: id.into(),
            key_type: KeyType::ECDSA_SECP256K1,
            purpose,
            security_level: SecurityLevel::HIGH,
            contract_bounds,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::default(),
        }
        .into()
    }

    #[test]
    fn should_pass_when_no_contract_bounds() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();
        let key = make_key_in_creation(0, ENCRYPTION, None);

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(result.is_valid(), "should be valid when no contract bounds");
    }

    #[test]
    fn should_fail_when_single_contract_does_not_exist() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();
        let non_existent_contract_id = Identifier::random();

        let key = make_key_in_creation(
            0,
            ENCRYPTION,
            Some(ContractBounds::SingleContract {
                id: non_existent_contract_id,
            }),
        );

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when contract does not exist"
        );
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::DataContractNotPresentError(e)) => {
                assert_eq!(e.data_contract_id(), &non_existent_contract_id);
            }
            other => panic!("expected DataContractNotPresentError, got {:?}", other),
        }
    }

    #[test]
    fn should_fail_when_single_contract_document_type_contract_does_not_exist() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();
        let non_existent_contract_id = Identifier::random();

        let key = make_key_in_creation(
            0,
            ENCRYPTION,
            Some(ContractBounds::SingleContractDocumentType {
                id: non_existent_contract_id,
                document_type_name: "note".to_string(),
            }),
        );

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when contract does not exist"
        );
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::DataContractNotPresentError(e)) => {
                assert_eq!(e.data_contract_id(), &non_existent_contract_id);
            }
            other => panic!("expected DataContractNotPresentError, got {:?}", other),
        }
    }

    #[test]
    fn should_fail_when_purpose_is_not_encryption_or_decryption_for_single_contract() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let data_contract = dpp::tests::fixtures::get_data_contract_fixture(
            None,
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let contract_id = data_contract.id();

        platform
            .drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let identity_id = Identifier::random();
        let key = make_key_in_creation(
            0,
            Purpose::AUTHENTICATION,
            Some(ContractBounds::SingleContract { id: contract_id }),
        );

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when purpose is not encryption or decryption"
        );
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::InvalidKeyPurposeForContractBoundsError(_)) => {}
            other => panic!(
                "expected InvalidKeyPurposeForContractBoundsError, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn should_fail_when_purpose_is_not_encryption_or_decryption_for_document_type_bound() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let data_contract = dpp::tests::fixtures::get_data_contract_fixture(
            None,
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let contract_id = data_contract.id();
        // Get a valid document type name from the contract
        let doc_type_name = data_contract
            .document_types()
            .keys()
            .next()
            .unwrap()
            .clone();

        platform
            .drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let identity_id = Identifier::random();
        let key = make_key_in_creation(
            0,
            Purpose::AUTHENTICATION,
            Some(ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name: doc_type_name,
            }),
        );

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when purpose is not encryption or decryption"
        );
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::InvalidKeyPurposeForContractBoundsError(_)) => {}
            other => panic!(
                "expected InvalidKeyPurposeForContractBoundsError, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn should_fail_when_document_type_does_not_exist_in_contract() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let data_contract = dpp::tests::fixtures::get_data_contract_fixture(
            None,
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let contract_id = data_contract.id();

        platform
            .drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let identity_id = Identifier::random();
        let key = make_key_in_creation(
            0,
            ENCRYPTION,
            Some(ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name: "nonExistentType".to_string(),
            }),
        );

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when document type does not exist"
        );
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::InvalidDocumentTypeError(e)) => {
                assert_eq!(e.document_type(), "nonExistentType");
            }
            other => panic!("expected InvalidDocumentTypeError, got {:?}", other),
        }
    }

    #[test]
    fn should_pass_with_empty_key_list() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &[],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(result.is_valid(), "should be valid with empty key list");
    }

    #[test]
    fn should_accumulate_errors_from_multiple_keys() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();
        let non_existent_contract_id_1 = Identifier::random();
        let non_existent_contract_id_2 = Identifier::random();

        let keys = vec![
            make_key_in_creation(
                0,
                ENCRYPTION,
                Some(ContractBounds::SingleContract {
                    id: non_existent_contract_id_1,
                }),
            ),
            make_key_in_creation(
                1,
                ENCRYPTION,
                Some(ContractBounds::SingleContract {
                    id: non_existent_contract_id_2,
                }),
            ),
        ];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            identity_id,
            &keys,
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(!result.is_valid(), "should be invalid");
        assert_eq!(result.errors.len(), 2, "should have errors from both keys");
    }
}
