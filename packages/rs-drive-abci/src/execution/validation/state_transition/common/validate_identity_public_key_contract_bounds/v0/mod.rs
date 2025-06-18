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
                                            BasicError::DataContractNotPresentError(
                                                DataContractNotPresentError::new(*contract_id),
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
