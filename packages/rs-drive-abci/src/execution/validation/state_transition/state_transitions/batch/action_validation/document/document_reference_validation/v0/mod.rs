use std::collections::{BTreeMap, BTreeSet};

use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentTypeRef,
};
use dpp::data_contract::DataContract;
use dpp::errors::consensus::state::document::referenced_document_property_mismatch_error::ReferencedDocumentPropertyMismatchError;
use dpp::errors::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
use dpp::errors::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
use dpp::errors::consensus::state::document::referenced_identity_key_disabled_error::ReferencedIdentityKeyDisabledError;
use dpp::errors::consensus::state::document::referenced_identity_key_not_found_error::ReferencedIdentityKeyNotFoundError;
use dpp::errors::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::KeyID;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, OptionalSingleIdentityPublicKeyOutcome,
};
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_document_with_id;
use crate::platform_types::platform::PlatformStateRef;

/// Versioned, stateful validation of document references using the v0 rules.
///
/// This performs existence checks for the supported reference targets (identity,
/// contract and token) and can be limited to changed fields for replace
/// transitions. It is intended to be called via the higher-level
/// `DocumentReferenceValidation` dispatcher that selects the version.
pub(crate) trait DocumentReferenceValidationV0 {
    #[allow(clippy::too_many_arguments)]
    fn validate_document_references_v0(
        &self,
        document_data: &BTreeMap<String, Value>,
        changed_fields: Option<&BTreeSet<String>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentReferenceValidationV0 for DocumentBaseTransitionAction {
    fn validate_document_references_v0(
        &self,
        document_data: &BTreeMap<String, Value>,
        changed_fields: Option<&BTreeSet<String>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.data_contract_fetch_info();
        let contract = &contract_fetch_info.contract;
        let document_type_name = self.document_type_name();

        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        validate_document_type_references_v0(
            contract,
            document_type,
            document_data,
            changed_fields,
            platform,
            block_info,
            transaction,
            execution_context,
            platform_version,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_document_type_references_v0(
    contract: &DataContract,
    document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    changed_fields: Option<&BTreeSet<String>>,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    for (path, property) in document_type.flattened_properties() {
        let DocumentPropertyType::IdentifierWithReference(reference_target) =
            &property.property_type
        else {
            continue;
        };

        if let Some(changed) = changed_fields {
            // A propertyAgreement pair binds the referring property too:
            // replacing it must re-validate the reference even when the
            // reference property itself is untouched.
            let agreement_property_changed = match reference_target {
                DocumentPropertyReferenceTarget::PermanentDocument {
                    property_agreement, ..
                } => property_agreement
                    .keys()
                    .any(|referring_property| is_changed_field(changed, referring_property)),
                _ => false,
            };
            if !is_changed_field(changed, path) && !agreement_property_changed {
                continue;
            }
        }

        let referenced_id = match document_data.get_optional_identifier_at_path(path) {
            Ok(Some(referenced_id)) => referenced_id,
            // A reference property that is not set is not validated; whether it may be
            // absent at all is enforced by the document type's required fields
            Ok(None) => continue,
            Err(err) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    InvalidIdentifierError::new(path.to_string(), err.to_string()).into(),
                ))
            }
        };

        let exists = match reference_target {
            DocumentPropertyReferenceTarget::Identity => {
                execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                    RetrieveIdentityInfo::only_revision(),
                ));

                platform
                    .drive
                    .fetch_identity_revision(referenced_id, true, transaction, platform_version)?
                    .is_some()
            }
            DocumentPropertyReferenceTarget::Contract => {
                let (fee, referenced_contract) =
                    platform.drive.get_contract_with_fetch_info_and_fee(
                        referenced_id,
                        Some(&block_info.epoch),
                        false,
                        transaction,
                        platform_version,
                    )?;

                let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "fee must exist when fetching a referenced contract with an epoch",
                )))?;

                // The cost is added even if the referenced contract does not exist or was cached
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

                referenced_contract.is_some()
            }
            DocumentPropertyReferenceTarget::Token => {
                // Token contract info is written for every token when its contract is
                // inserted and is never deleted, so it serves as the existence record
                let (referenced_token_info, fee) =
                    platform.drive.fetch_token_contract_info_with_costs(
                        referenced_id,
                        block_info,
                        true,
                        transaction,
                        platform_version,
                    )?;

                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

                referenced_token_info.is_some()
            }
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: referenced_contract_id,
                document_type_name,
                property_agreement,
            } => {
                // An absent contract id targets the declaring contract itself; the
                // declaring contract may also name its own id explicitly. Either
                // way it is already loaded for this transition, so no fetch is
                // billed for it
                let effective_contract_id = referenced_contract_id.unwrap_or(contract.id());
                let referenced_contract_fetch_info;
                let referenced_contract = if effective_contract_id == contract.id() {
                    contract
                } else {
                    let (fee, fetch_info) = platform.drive.get_contract_with_fetch_info_and_fee(
                        effective_contract_id.to_buffer(),
                        Some(&block_info.epoch),
                        false,
                        transaction,
                        platform_version,
                    )?;

                    let fee =
                        fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "fee must exist when fetching a referenced contract with an epoch",
                        )))?;

                    // The cost is added even if the referenced contract does not exist or was cached
                    execution_context
                        .add_operation(ValidationOperation::PrecalculatedOperation(fee));

                    let Some(fetch_info) = fetch_info else {
                        // A missing contract and a missing document type resolve to the
                        // same failure: the declared document type could not be found
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedDocumentTypeNotFoundError::new(
                                effective_contract_id,
                                document_type_name.clone(),
                                path.to_string(),
                            )
                            .into(),
                        ));
                    };

                    referenced_contract_fetch_info = fetch_info;
                    &referenced_contract_fetch_info.contract
                };

                let Some(referenced_document_type) =
                    referenced_contract.document_type_optional_for_name(document_type_name)
                else {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedDocumentTypeNotFoundError::new(
                            effective_contract_id,
                            document_type_name.clone(),
                            path.to_string(),
                        )
                        .into(),
                    ));
                };

                // Only document types whose documents can never be deleted may be
                // referenced: `canBeDeleted` is immutable on contract updates and
                // document types can not be removed, so a reference validated here
                // can never dangle
                if referenced_document_type.documents_can_be_deleted() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedDocumentTypeDeletableError::new(
                            effective_contract_id,
                            document_type_name.clone(),
                            path.to_string(),
                        )
                        .into(),
                    ));
                }

                let referenced_document = fetch_document_with_id(
                    platform.drive,
                    referenced_contract,
                    referenced_document_type,
                    Identifier::from(referenced_id),
                    &block_info.epoch,
                    execution_context,
                    transaction,
                    platform_version,
                )?;

                // Property agreement: the referenced document is already in
                // hand for the existence check, so comparing the declared
                // pairs adds no reads. Each side is normalized through its
                // OWN document type's key encoding — one deterministic
                // normal form per value kind, so an identifier stored as
                // bytes and one carried as an identifier compare equal.
                //
                // Absence is part of the agreement, strictly: both sides
                // absent agree, one side absent is a mismatch. Anything
                // laxer breaks the properties agreements exist for — with
                // referring-absent-always-ok, a document could opt out of
                // echoing a value its referenced document carries (e.g. a
                // like on a TAGGED post omitting the tag, silently
                // deflating every per-tag aggregate), and the referenced
                // side's absence is what lets a referring doctype whose
                // agreement key triggers a skipIfAbsent index stay
                // consistently absent for untagged targets.
                if let Some(referenced_document) = &referenced_document {
                    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
                    use dpp::document::DocumentV0Getters;

                    for (referring_property, referenced_property) in property_agreement {
                        let mismatch = || {
                            SimpleConsensusValidationResult::new_with_error(
                                ReferencedDocumentPropertyMismatchError::new(
                                    path.to_string(),
                                    referring_property.clone(),
                                    referenced_property.clone(),
                                )
                                .into(),
                            )
                        };
                        // A lookup ERROR (a non-map value where the dotted
                        // path expects an intermediate object) is a
                        // mismatch, never absence — folding it into `None`
                        // would let two malformed sides "agree" as
                        // both-absent.
                        let Ok(referring_value) =
                            document_data.get_optional_at_path(referring_property)
                        else {
                            return Ok(mismatch());
                        };
                        let Ok(referenced_value) = referenced_document
                            .properties()
                            .get_optional_at_path(referenced_property)
                        else {
                            return Ok(mismatch());
                        };
                        let (referring_value, referenced_value) =
                            match (referring_value, referenced_value) {
                                (Some(referring_value), Some(referenced_value)) => {
                                    (referring_value, referenced_value)
                                }
                                // Both absent: the sides agree.
                                (None, None) => continue,
                                // One side absent: a mismatch, exactly as a
                                // differing value would be.
                                (Some(_), None) | (None, Some(_)) => return Ok(mismatch()),
                            };
                        let Ok(referring_encoded) = document_type.serialize_value_for_key(
                            referring_property,
                            referring_value,
                            platform_version,
                        ) else {
                            return Ok(mismatch());
                        };
                        let Ok(referenced_encoded) = referenced_document_type
                            .serialize_value_for_key(
                                referenced_property,
                                referenced_value,
                                platform_version,
                            )
                        else {
                            return Ok(mismatch());
                        };
                        if referring_encoded != referenced_encoded {
                            return Ok(mismatch());
                        }
                    }
                }

                referenced_document.is_some()
            }
            DocumentPropertyReferenceTarget::IdentityPublicKey { key_id_property } => {
                // The referenced key id is carried by the named sibling property
                let key_id: KeyID =
                    match document_data.get_optional_integer_at_path(key_id_property) {
                        Ok(Some(key_id)) => key_id,
                        Ok(None) => {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                ReferencedKeyIdPropertyInvalidError::new(
                                    key_id_property.clone(),
                                    path.to_string(),
                                    "the key id property is not set".to_string(),
                                )
                                .into(),
                            ))
                        }
                        Err(err) => {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                ReferencedKeyIdPropertyInvalidError::new(
                                    key_id_property.clone(),
                                    path.to_string(),
                                    err.to_string(),
                                )
                                .into(),
                            ))
                        }
                    };

                execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                    RetrieveIdentityInfo::one_key(),
                ));

                // A missing identity and a missing key resolve to the same
                // failure: the referenced key could not be found
                let Some(key) = platform
                    .drive
                    .fetch_identity_keys::<OptionalSingleIdentityPublicKeyOutcome>(
                        IdentityKeysRequest::new_specific_key_query(&referenced_id, key_id),
                        transaction,
                        platform_version,
                    )?
                else {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedIdentityKeyNotFoundError::new(
                            Identifier::from(referenced_id),
                            key_id,
                            path.to_string(),
                        )
                        .into(),
                    ));
                };

                // Keys can never be removed, so an existing reference can not
                // dangle; a disabled key is still rejected for fresh writes
                if key.is_disabled() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedIdentityKeyDisabledError::new(
                            Identifier::from(referenced_id),
                            key_id,
                            path.to_string(),
                        )
                        .into(),
                    ));
                }

                true
            }
        };

        if !exists {
            let missing_id =
                Identifier::from_bytes(&referenced_id).map_err(|e| Error::Protocol(e.into()))?;

            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(
                    ReferencedEntityNotFoundError::new(
                        missing_id,
                        reference_target.clone(),
                        path.to_string(),
                    ),
                )),
            ));
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}

/// A flattened property path counts as changed when the replace transition changed
/// the path itself or any of its ancestors: `changed_data_fields` holds top-level
/// document keys, so a changed object key replaces its entire subtree, including
/// any nested reference properties under it.
fn is_changed_field(changed_fields: &BTreeSet<String>, path: &str) -> bool {
    changed_fields.iter().any(|field| {
        path == field
            || path
                .strip_prefix(field.as_str())
                .is_some_and(|rest| rest.starts_with('.'))
    })
}
