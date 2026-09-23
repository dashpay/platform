use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    is_referenced_system_agreement_property, is_referring_system_agreement_property,
    DocumentProperty, DocumentPropertyReferenceTarget, DocumentPropertyType,
    DocumentReferenceDeclaration, KeyReferenceIdentityProperty,
};
use dpp::data_contract::DataContract;
use dpp::document::property_names::CREATOR_ID;
use dpp::errors::consensus::state::document::referenced_document_property_agreement_invalid_error::ReferencedDocumentPropertyAgreementInvalidError;
use dpp::errors::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_deletable_error::ReferencedDocumentTypeNotDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
use dpp::errors::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::query::TransactionArg;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};

/// Whether two property types hold the same KIND of value for agreement
/// purposes: sizes and other constraints may differ (both sides validated
/// their own documents already), and an identifier, or a `u32` key id, is one
/// kind whether or not it carries its own reference annotation.
fn same_value_kind(a: &DocumentPropertyType, b: &DocumentPropertyType) -> bool {
    let normalized_kind = |property_type: &DocumentPropertyType| match property_type {
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            std::mem::discriminant(&DocumentPropertyType::Identifier)
        }
        DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
            std::mem::discriminant(&DocumentPropertyType::U32)
        }
        other => std::mem::discriminant(other),
    };
    normalized_kind(a) == normalized_kind(b)
}

/// Checks every reference declaration of the given contract that carries
/// declaration content.
///
/// `permanentDocument` and `deletableDocument`: the referenced contract must
/// exist (the declaring contract itself when no contract id is named,
/// including when it names its own id) and the referenced document type must
/// exist in it; for `permanentDocument` that type must forbid deletion, for
/// `deletableDocument` it must allow it.
/// Every `propertyAgreement` pair is checked for both. Self references are
/// checked against the in-flight
/// contract, so a contract may reference its own document types on creation;
/// foreign contract fetches are billed.
///
/// `identityPublicKey`: the declared key id property must exist in the same
/// document type and be an integer.
///
/// The error paths name the failing declaration as
/// `documentTypeName.propertyPath`. Validation stops at the first invalid
/// declaration: this bounds the billed work an invalid contract can cause and
/// matches document write-time reference validation. Foreign contract
/// resolutions are memoized per contract id, so a contract declaring many
/// references into the same foreign contract is billed one fetch for it.
pub(super) fn validate_data_contract_references_v0(
    contract: &DataContract,
    drive: &Drive,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // Memoizes foreign contract resolutions (including misses) so repeated
    // declarations naming the same contract are billed a single fetch
    let mut fetched_contracts: BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>> =
        BTreeMap::new();

    for (declaring_type_name, document_type) in contract.document_types() {
        for (path, property) in document_type.as_ref().flattened_properties() {
            let declaration_path = format!("{declaring_type_name}.{path}");

            let reference_target = match &property.property_type {
                DocumentPropertyType::IdentifierWithReference(reference_target) => reference_target,
                // A key reference on the key id property: what `identityProperty`
                // names must fit the document type; nothing else about the
                // declaration is state-dependent
                DocumentPropertyType::KeyIdWithReference(identity_property) => {
                    let invalid = |message: &str| {
                        SimpleConsensusValidationResult::new_with_error(
                            ReferencedKeyIdPropertyInvalidError::new(
                                path.to_string(),
                                declaration_path.clone(),
                                message.to_string(),
                            )
                            .into(),
                        )
                    };
                    match identity_property {
                        KeyReferenceIdentityProperty::OwnerId => {}
                        KeyReferenceIdentityProperty::CreatorId => {
                            if !document_type
                                .as_ref()
                                .should_use_creator_id(
                                    contract.system_version_type(),
                                    contract.config().version(),
                                    platform_version,
                                )
                                .map_err(Error::Protocol)?
                            {
                                return Ok(invalid(
                                    "identityProperty $creatorId needs a document type that \
                                     records creator ids: only transferable or tradeable \
                                     document types of a format-1 contract do",
                                ));
                            }
                        }
                        KeyReferenceIdentityProperty::Property(identity_path) => {
                            match document_type
                                .as_ref()
                                .flattened_properties()
                                .get(identity_path)
                            {
                                None => {
                                    return Ok(invalid(&format!(
                                        "the document type does not define the identity \
                                         property {identity_path}"
                                    )));
                                }
                                // The (identity, key id) pair is declared once
                                Some(DocumentProperty {
                                    property_type:
                                        DocumentPropertyType::IdentifierWithReference(
                                            DocumentPropertyReferenceTarget::IdentityPublicKey {
                                                ..
                                            },
                                        ),
                                    ..
                                }) => {
                                    return Ok(invalid(&format!(
                                        "the identity property {identity_path} carries its own \
                                         identityPublicKey reference"
                                    )));
                                }
                                Some(identity_property)
                                    if !matches!(
                                        identity_property.property_type,
                                        DocumentPropertyType::Identifier
                                            | DocumentPropertyType::IdentifierWithReference(_)
                                    ) =>
                                {
                                    return Ok(invalid(&format!(
                                        "the identity property {identity_path} must be an \
                                         identifier"
                                    )));
                                }
                                Some(_) => {}
                            }
                        }
                    }
                    continue;
                }
                _ => continue,
            };

            // The key id property must exist in the same document type and be
            // an integer; nothing else about the declaration is state-dependent
            if let DocumentPropertyReferenceTarget::IdentityPublicKey { key_id_property } =
                reference_target
            {
                match document_type
                    .as_ref()
                    .flattened_properties()
                    .get(key_id_property)
                {
                    None => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedKeyIdPropertyInvalidError::new(
                                key_id_property.clone(),
                                declaration_path,
                                "the document type does not define this property".to_string(),
                            )
                            .into(),
                        ));
                    }
                    Some(key_property) if !key_property.property_type.is_integer() => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedKeyIdPropertyInvalidError::new(
                                key_id_property.clone(),
                                declaration_path,
                                "the property must be an integer".to_string(),
                            )
                            .into(),
                        ));
                    }
                    // A key id that already names whose key it is (the writer's)
                    // can not also be a key of the referenced identity
                    Some(DocumentProperty {
                        property_type: DocumentPropertyType::KeyIdWithReference(_),
                        ..
                    }) => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedKeyIdPropertyInvalidError::new(
                                key_id_property.clone(),
                                declaration_path,
                                "the property carries its own identityPublicKey reference"
                                    .to_string(),
                            )
                            .into(),
                        ));
                    }
                    Some(_) => continue,
                }
            }

            let Some(DocumentReferenceDeclaration {
                contract_id,
                document_type_name,
                property_agreement,
                permanent,
            }) = reference_target.as_document_reference()
            else {
                continue;
            };

            let effective_contract_id = contract_id.unwrap_or(contract.id());

            let referenced_contract_fetch_info;
            let referenced_contract = if effective_contract_id == contract.id() {
                contract
            } else {
                let resolved = match fetched_contracts.get(&effective_contract_id) {
                    Some(cached) => cached.clone(),
                    None => {
                        let (fee, fetch_info) = drive.get_contract_with_fetch_info_and_fee(
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

                        // The cost is added even if the referenced contract does not exist
                        // or was served from Drive's own contract cache; only locally
                        // memoized repeats above skip it
                        execution_context
                            .add_operation(ValidationOperation::PrecalculatedOperation(fee));

                        fetched_contracts.insert(effective_contract_id, fetch_info.clone());

                        fetch_info
                    }
                };

                let Some(fetch_info) = resolved else {
                    // A missing contract and a missing document type resolve to the
                    // same failure: the declared document type could not be found
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedDocumentTypeNotFoundError::new(
                            effective_contract_id,
                            document_type_name.to_string(),
                            declaration_path,
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
                        document_type_name.to_string(),
                        declaration_path,
                    )
                    .into(),
                ));
            };

            // The two document references are disjoint: a
            // `permanentDocument` one demands a document type that forbids
            // deletion, a `deletableDocument` one a document type that
            // allows it, so the declaration always states which guarantee
            // the reference carries. Deletable means by anyone: a document type moderators
            // can delete from is deletable whatever its `canBeDeleted` says about a document's
            // own owner, since a reference to it could dangle. Neither flag can change on an
            // update, so the answer holds for good.
            let target_is_deletable = referenced_document_type.documents_can_be_deleted()
                || referenced_document_type.documents_can_be_deleted_by_moderators();
            if permanent && target_is_deletable {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentTypeDeletableError::new(
                        effective_contract_id,
                        document_type_name.to_string(),
                        declaration_path,
                    )
                    .into(),
                ));
            }
            if !permanent && !target_is_deletable {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentTypeNotDeletableError::new(
                        effective_contract_id,
                        document_type_name.to_string(),
                        declaration_path,
                    )
                    .into(),
                ));
            }

            // propertyAgreement declarations: both sides must exist, be
            // plain values (not containers), and share one value kind — a
            // cross-kind equality could never be satisfied and would brick
            // every create of the declaring document type. The referenced
            // side may instead be one of the referenced document's
            // `$ownerId` and `$creatorId` system identifiers, which then
            // must face an identifier on the referring side; `$creatorId`
            // further needs a referenced type that records creator ids at
            // all, or again no document could ever agree. The referring side
            // may be the writer's own `$ownerId` instead of a schema property,
            // an identifier that lives on the transition: that pair is a
            // write gate.
            let writer_identifier_type = DocumentPropertyType::Identifier;
            for (referring_property, referenced_property) in property_agreement {
                let invalid = |reason: &str| {
                    SimpleConsensusValidationResult::new_with_error(
                        ReferencedDocumentPropertyAgreementInvalidError::new(
                            declaration_path.clone(),
                            referring_property.clone(),
                            referenced_property.clone(),
                            reason.to_string(),
                        )
                        .into(),
                    )
                };
                if referring_property == path {
                    return Ok(invalid(
                        "the referring property cannot be the reference property itself",
                    ));
                }
                let declaring_document_type = document_type.as_ref();
                let referring_type = if referring_property.starts_with('$') {
                    if !is_referring_system_agreement_property(referring_property) {
                        return Ok(invalid(
                            "the referring side must be a schema property of the declaring \
                             document type or its $ownerId",
                        ));
                    }
                    &writer_identifier_type
                } else {
                    let Some(referring) = declaring_document_type
                        .flattened_properties()
                        .get(referring_property)
                    else {
                        return Ok(invalid(
                            "the declaring document type does not define the referring property",
                        ));
                    };
                    &referring.property_type
                };
                if referenced_property.starts_with('$') {
                    if !is_referenced_system_agreement_property(referenced_property) {
                        return Ok(invalid(
                            "only the referenced document's $ownerId and $creatorId system \
                             properties may be agreed with",
                        ));
                    }
                    if !matches!(
                        referring_type,
                        DocumentPropertyType::Identifier
                            | DocumentPropertyType::IdentifierWithReference(_)
                    ) {
                        return Ok(invalid(
                            "$ownerId and $creatorId are identifiers, so the referring \
                             property must be an identifier",
                        ));
                    }
                    if referenced_property == CREATOR_ID
                        && !referenced_document_type
                            .should_use_creator_id(
                                referenced_contract.system_version_type(),
                                referenced_contract.config().version(),
                                platform_version,
                            )
                            .map_err(Error::Protocol)?
                    {
                        return Ok(invalid(
                            "the referenced document type does not record $creatorId: only \
                             transferable or tradeable document types of a format-1 contract \
                             do",
                        ));
                    }
                    continue;
                }
                let Some(referenced) = referenced_document_type
                    .flattened_properties()
                    .get(referenced_property)
                else {
                    return Ok(invalid(
                        "the referenced document type does not define the referenced property",
                    ));
                };
                if matches!(referring_type, DocumentPropertyType::Object(_))
                    || matches!(referenced.property_type, DocumentPropertyType::Object(_))
                {
                    return Ok(invalid(
                        "agreement properties must be plain values, not object containers",
                    ));
                }
                // The write-time check compares index key encodings, which a
                // list does not have, so an agreement on one would never hold
                if matches!(referring_type, DocumentPropertyType::TypedArray(_))
                    || matches!(
                        referenced.property_type,
                        DocumentPropertyType::TypedArray(_)
                    )
                {
                    return Ok(invalid(
                        "agreement properties must be single values, not typed arrays",
                    ));
                }
                if !same_value_kind(referring_type, &referenced.property_type) {
                    return Ok(invalid(
                        "the two properties must share one value kind: a cross-kind \
                         equality could never be satisfied",
                    ));
                }
            }
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}
