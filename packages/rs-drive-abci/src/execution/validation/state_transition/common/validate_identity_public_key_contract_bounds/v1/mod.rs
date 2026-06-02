//! v1 of `validate_identity_public_keys_contract_bounds`.
//!
//! Two behavioral changes vs. v0, both gated together at PROTOCOL_VERSION_12:
//!
//! 1. **Correctness fix.** The `Purpose::DECRYPTION` arm of
//!    `ContractBounds::SingleContractDocumentType` now consults
//!    `requires_identity_decryption_bounded_key()` instead of v0's mistaken
//!    `requires_identity_encryption_bounded_key()`.
//!
//! 2. **Fee accounting (audit N6/N7, follow-up to #3670 / issue #3673).**
//!    Every contract fetch and identity-key lookup performed during validation
//!    is now billed to the passed-in execution context, instead of being
//!    discarded as v0 did (which had an explicit `//todo:` for this).

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::identity::{
    DataContractBoundsNotPresentError, InvalidKeyPurposeForContractBoundsError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::identity::identity_public_key_already_exists_for_unique_contract_bounds_error::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use dpp::identifier::Identifier;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Purpose;
use dpp::identity::Purpose::{DECRYPTION, ENCRYPTION};
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, OptionalSingleIdentityPublicKeyOutcome,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(super) fn validate_identity_public_keys_contract_bounds_v1(
    identity_id: Identifier,
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    epoch: &Epoch,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut per_key_results = Vec::with_capacity(identity_public_keys_with_witness.len());
    for identity_public_key in identity_public_keys_with_witness {
        per_key_results.push(validate_identity_public_key_contract_bounds_v1(
            identity_id,
            identity_public_key,
            drive,
            epoch,
            transaction,
            execution_context,
            platform_version,
        )?);
    }
    Ok(SimpleConsensusValidationResult::merge_many_errors(
        per_key_results,
    ))
}

fn validate_identity_public_key_contract_bounds_v1(
    identity_id: Identifier,
    identity_public_key_in_creation: &IdentityPublicKeyInCreation,
    drive: &Drive,
    epoch: &Epoch,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let purpose = identity_public_key_in_creation.purpose();
    let Some(contract_bounds) = identity_public_key_in_creation.contract_bounds() else {
        return Ok(SimpleConsensusValidationResult::new());
    };

    // Resolve the bounded contract. System contracts come from the in-memory cache
    // (no fee); user contracts come from grovedb (billed); a missing contract still
    // costs the grovedb lookup that proved its absence (also billed).
    let contract_id = match contract_bounds {
        ContractBounds::SingleContract { id } => *id,
        ContractBounds::SingleContractDocumentType { id, .. } => *id,
    };
    let outcome = drive.get_system_or_user_contract_with_fee(
        contract_id.to_buffer(),
        epoch,
        transaction,
        platform_version,
    )?;
    if let Some(fee) = outcome.fee() {
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee.clone()));
    }
    let Some(contract) = outcome.contract() else {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            ConsensusError::BasicError(BasicError::DataContractNotPresentError(
                DataContractNotPresentError::new(contract_id),
            )),
        ));
    };

    match contract_bounds {
        ContractBounds::SingleContract { .. } => {
            let requirements_for_purpose = match purpose {
                ENCRYPTION => contract.config().requires_identity_encryption_bounded_key(),
                DECRYPTION => contract.config().requires_identity_decryption_bounded_key(),
                purpose => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(
                            BasicError::InvalidKeyPurposeForContractBoundsError(
                                InvalidKeyPurposeForContractBoundsError::new(
                                    purpose,
                                    vec![ENCRYPTION, DECRYPTION],
                                ),
                            ),
                        ),
                    ));
                }
            };
            let Some(requirements) = requirements_for_purpose else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::DataContractBoundsNotPresentError(
                        DataContractBoundsNotPresentError::new(contract_id),
                    )),
                ));
            };
            check_unique_bound_key(
                identity_id,
                identity_public_key_in_creation,
                contract_id,
                purpose,
                requirements,
                KeyRequestType::ContractBoundKey(
                    contract_id.to_buffer(),
                    purpose,
                    KeyKindRequestType::CurrentKeyOfKindRequest,
                ),
                drive,
                epoch,
                transaction,
                execution_context,
                platform_version,
            )
        }
        ContractBounds::SingleContractDocumentType {
            document_type_name, ..
        } => {
            let Some(document_type) =
                contract.document_type_optional_for_name(document_type_name.as_str())
            else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidDocumentTypeError(
                        InvalidDocumentTypeError::new(document_type_name.clone(), contract_id),
                    )),
                ));
            };
            let requirements_for_purpose = match purpose {
                ENCRYPTION => document_type.requires_identity_encryption_bounded_key(),
                // v1 fix: v0 mistakenly called `requires_identity_encryption_bounded_key()` here.
                DECRYPTION => document_type.requires_identity_decryption_bounded_key(),
                purpose => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(
                            BasicError::InvalidKeyPurposeForContractBoundsError(
                                InvalidKeyPurposeForContractBoundsError::new(
                                    purpose,
                                    vec![ENCRYPTION, DECRYPTION],
                                ),
                            ),
                        ),
                    ));
                }
            };
            let Some(requirements) = requirements_for_purpose else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::DataContractBoundsNotPresentError(
                        DataContractBoundsNotPresentError::new(contract_id),
                    )),
                ));
            };
            check_unique_bound_key(
                identity_id,
                identity_public_key_in_creation,
                contract_id,
                purpose,
                requirements,
                KeyRequestType::ContractDocumentTypeBoundKey(
                    contract_id.to_buffer(),
                    document_type_name.clone(),
                    purpose,
                    KeyKindRequestType::CurrentKeyOfKindRequest,
                ),
                drive,
                epoch,
                transaction,
                execution_context,
                platform_version,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_unique_bound_key(
    identity_id: Identifier,
    identity_public_key_in_creation: &IdentityPublicKeyInCreation,
    contract_id: Identifier,
    purpose: Purpose,
    requirements: StorageKeyRequirements,
    request_type: KeyRequestType,
    drive: &Drive,
    epoch: &Epoch,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    match requirements {
        StorageKeyRequirements::Unique => {
            let key_request = IdentityKeysRequest {
                identity_id: identity_id.to_buffer(),
                request_type,
                limit: None,
                offset: None,
            };
            let (maybe_conflicting_key, fee) = drive
                .fetch_identity_keys_with_costs::<OptionalSingleIdentityPublicKeyOutcome>(
                    key_request,
                    epoch,
                    transaction,
                    platform_version,
                )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            if let Some(conflicting_key) = maybe_conflicting_key {
                Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(
                            IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError::new(
                                identity_id,
                                contract_id,
                                purpose,
                                identity_public_key_in_creation.id(),
                                conflicting_key.id(),
                            ),
                        ),
                    ),
                ))
            } else {
                Ok(SimpleConsensusValidationResult::new())
            }
        }
        StorageKeyRequirements::Multiple | StorageKeyRequirements::MultipleReferenceToLatest => {
            Ok(SimpleConsensusValidationResult::new())
        }
    }
}
