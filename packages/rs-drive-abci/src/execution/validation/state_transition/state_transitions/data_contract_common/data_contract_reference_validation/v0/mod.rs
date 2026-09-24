use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    is_referenced_system_agreement_property, is_referring_system_agreement_property, is_transient,
    DocumentProperty, DocumentPropertyReferenceTarget, DocumentPropertyType,
    DocumentReferenceDeclaration, DocumentTypeRef, KeyReferenceIdentityProperty, PropertyReference,
    ReferenceHolder,
};
use dpp::data_contract::DataContract;
use dpp::document::property_names::CREATOR_ID;
use dpp::errors::consensus::state::document::referenced_document_list_invalid_error::ReferencedDocumentListInvalidError;
use dpp::errors::consensus::state::document::referenced_document_lookup_invalid_error::ReferencedDocumentLookupInvalidError;
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
/// their own documents already). The rule is `DocumentPropertyType::value_kind`,
/// shared with the key parts of a `refersTo` lookup.
fn same_value_kind(a: &DocumentPropertyType, b: &DocumentPropertyType) -> bool {
    a.value_kind() == b.value_kind()
}

/// Whether a key reference pairing the key id at `key_id_path` with the
/// identity at `identity_path` of `document_type` would store the key id
/// without its identity: the identity transient (or inside a transient
/// object) while the key id is not. A stored key id alone names no key. With
/// both transient, or the key id alone, nothing unreadable is stored.
fn stores_key_id_without_identity(
    document_type: DocumentTypeRef,
    key_id_path: &str,
    identity_path: &str,
) -> bool {
    is_transient(document_type, identity_path) && !is_transient(document_type, key_id_path)
}

/// Checks every reference declaration of the given contract that carries
/// declaration content.
///
/// `permanentDocument` and `deletableDocument`: the referenced contract must
/// exist (the declaring contract itself when no contract id is named,
/// including when it names its own id) and the referenced document type must
/// exist in it; for `permanentDocument` that type must forbid deletion, for
/// `deletableDocument` it must allow it.
/// Every `propertyAgreement` pair is checked for both, and a `lookup` into
/// another contract's document type is checked against that type's indexes
/// (one into the declaring contract was checked by the contract parse). Self
/// references are checked against the in-flight contract, so a contract may
/// reference its own document types on creation; foreign contract fetches are
/// billed.
///
/// `listElement`: a document reference like `permanentDocument` (same
/// checks, the type must forbid deletion, `$id` admitted on the referenced
/// side of a pair), plus, for a list in a document type of another contract,
/// that `inList` is a stored typed array of identifiers fixed once a document
/// is written. One in the declaring contract was checked by the contract
/// parse.
///
/// `identityPublicKey`: the declared key id property must exist in the same
/// document type and be an integer. On either side of a key reference, a
/// stored key id may not pair with a transient identity, which would leave it
/// naming no key; an agreement's referenced property may not be transient,
/// since no stored document carries its value.
///
/// A declaration on the `items` of a typed array of identifiers holds for
/// every element and is checked once, exactly as a single reference's: the
/// referring side of an agreement is still a property of the declaring
/// document type (or the writer), the referenced side a property of the
/// referenced document type. `identityPublicKey` never reaches here on
/// elements: the parser refuses it there.
///
/// A document type's `ownerRefersTo` or `creatorRefersTo` declaration, whose
/// value is the writer or the creator, is checked as a single identifier
/// reference's is, first; the parser only admits an `identity` or a
/// `permanentDocument` lookup one, or an expression of them.
///
/// Each leaf of a reference expression (`anyOf` / `allOf`) is checked exactly
/// as the same target declared alone, in declared order, and every one of them
/// must pass: an `anyOf` lets a WRITE satisfy one operand, but each leaf has to
/// be a declaration that could hold. A `propertyAgreement` belongs to its own
/// leaf and is checked against that leaf's document type only.
///
/// The error paths name the failing declaration as
/// `documentTypeName.propertyPath`, an element declaration by its list
/// path, `documentTypeName.propertyPath[]`, and a leaf of a reference
/// expression by where it sits, `documentTypeName.propertyPath.anyOf[1]` or
/// `documentTypeName.propertyPath[].anyOf[1].allOf[0]`, and the owner and
/// creator references as `documentTypeName.$ownerId` and
/// `documentTypeName.$creatorId`. Validation stops at the first invalid
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
        // The writer's or the creator's reference first (`ownerRefersTo` or
        // `creatorRefersTo`, whose value is the document's `$ownerId` or
        // `$creatorId` and which is named by that path), then the properties'
        // own. Neither is ever a key reference: the parser only admits an
        // identity or a permanentDocument lookup there, or an expression of
        // them. Inert before protocol version 14: this module is only called
        // from contract create and update state validation 1, selected from
        // it, and no parse before it sets an owner or creator reference.
        for (holder, reference) in document_type.as_ref().reference_declarations() {
            let path = holder.path();
            // The property carrying the reference, which an agreement may not
            // name on its referring side; the owner's and the creator's are
            // carried by no property
            let reference_property = match holder {
                ReferenceHolder::Property(path) => Some(path),
                ReferenceHolder::Owner | ReferenceHolder::Creator => None,
            };
            let declaration_path = format!("{declaring_type_name}.{path}");

            let (reference_target, declaration_path) = match reference {
                // A key reference on the key id property: what `identityProperty`
                // names must fit the document type; nothing else about the
                // declaration is state-dependent
                PropertyReference::KeyId(reference) => {
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
                    match &reference.identity_property {
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
                            // In place: inert before protocol version 14, which
                            // alone reaches this module (contract create and
                            // update state validation 1)
                            if stores_key_id_without_identity(
                                document_type.as_ref(),
                                path,
                                identity_path,
                            ) {
                                return Ok(invalid(&format!(
                                    "the key id is stored but the identity property \
                                     {identity_path} is transient or inside a transient object: \
                                     a reader could not tell whose key it is"
                                )));
                            }
                        }
                    }
                    continue;
                }
                PropertyReference::Value(target) => (target, declaration_path),
                // A typed array only parses from protocol version 14, whose
                // contract create and update state validation are the only
                // callers, so this arm is never reached before it
                PropertyReference::Elements { target, .. } => {
                    (target, format!("{declaring_type_name}.{path}[]"))
                }
            };

            // Each leaf of a reference expression is checked as it would be
            // declared alone, its errors naming it by where it sits
            // (`documentTypeName.propertyPath.anyOf[1].allOf[0]`). In place in
            // generation 0, which every table selects: contract create and
            // update state validation call it from protocol version 14 only,
            // and a single declaration is its own one leaf at an empty path, so
            // it is checked exactly as before expressions existed
            for (leaf_path, target) in reference_target.leaves_with_paths() {
                let target_path = if leaf_path.is_empty() {
                    declaration_path.clone()
                } else {
                    format!("{declaration_path}.{leaf_path}")
                };
                let result = validate_reference_target_declaration_v0(
                    contract,
                    document_type.as_ref(),
                    reference_property,
                    target,
                    target_path,
                    &mut fetched_contracts,
                    drive,
                    block_info,
                    execution_context,
                    transaction,
                    platform_version,
                )?;
                if !result.is_valid() {
                    return Ok(result);
                }
            }
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}

/// Checks one single target declaration of the property at
/// `reference_property` of `document_type` (`None` for the type's owner or
/// creator reference), a declaration of its own or one leaf of a reference
/// expression, against the contract and state: see
/// [`validate_data_contract_references_v0`].
/// `declaration_path` is how the errors name it; foreign contract resolutions
/// are shared through `fetched_contracts`.
#[allow(clippy::too_many_arguments)]
fn validate_reference_target_declaration_v0(
    contract: &DataContract,
    document_type: DocumentTypeRef<'_>,
    reference_property: Option<&str>,
    reference_target: &DocumentPropertyReferenceTarget,
    declaration_path: String,
    fetched_contracts: &mut BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>>,
    drive: &Drive,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // The key id property must exist in the same document type and be
    // an integer; nothing else about the declaration is state-dependent
    if let DocumentPropertyReferenceTarget::IdentityPublicKey {
        key_id_property, ..
    } = reference_target
    {
        match document_type.flattened_properties().get(key_id_property) {
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
                        "the property carries its own identityPublicKey reference".to_string(),
                    )
                    .into(),
                ));
            }
            Some(_) => {
                // In place: inert before protocol version 14, which alone
                // reaches this module (contract create and update state
                // validation 1)
                let stored_without_identity = reference_property.is_some_and(|identity_path| {
                    stores_key_id_without_identity(document_type, key_id_property, identity_path)
                });
                if stored_without_identity {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedKeyIdPropertyInvalidError::new(
                            key_id_property.clone(),
                            declaration_path,
                            "the key id is stored but the identity property carrying the \
                             reference is transient or inside a transient object: a reader \
                             could not tell whose key it is"
                                .to_string(),
                        )
                        .into(),
                    ));
                }
                return Ok(SimpleConsensusValidationResult::new());
            }
        }
    }

    let Some(DocumentReferenceDeclaration {
        contract_id,
        document_type_name,
        property_agreement,
        permanent,
        lookup,
        in_list,
    }) = reference_target.as_any_document_reference()
    else {
        return Ok(SimpleConsensusValidationResult::new());
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

                let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "fee must exist when fetching a referenced contract with an epoch",
                )))?;

                // The cost is added even if the referenced contract does not exist
                // or was served from Drive's own contract cache; only locally
                // memoized repeats above skip it
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

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

    // A lookup, on a document reference of either kind, must
    // resolve in the referenced document type: a unique index its keys
    // cover exactly, filled from sources of the right kinds, with a key
    // that stays with the document it found. The contract parse checks
    // a lookup into the declaring contract under full validation, where
    // it sees every document type; only here is another contract's
    // document type in hand.
    if let Some(lookup) = lookup {
        if effective_contract_id != contract.id() {
            if let Some(reason) =
                lookup.referenced_side_error(document_type, referenced_document_type)
            {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentLookupInvalidError::new(
                        declaration_path,
                        lookup.index.clone(),
                        reason,
                    )
                    .into(),
                ));
            }
        }
    }

    // A list element's list, in a document type of another contract: a
    // stored typed array of identifiers fixed once a document is written (the
    // type is permanent, checked above). The contract parse checks a list in
    // the declaring contract under full validation, where it sees every
    // document type; only here is another contract's document type in hand.
    // The `$id` pair naming the list's document was checked by the parse, and
    // the other pairs are checked below as every agreement is
    if let (Some(_), Some(reference)) = (in_list, reference_target.as_list_element_reference()) {
        if effective_contract_id != contract.id() {
            if let Some(reason) = reference.referenced_side_error(referenced_document_type) {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentListInvalidError::new(
                        declaration_path,
                        reference.in_list.clone(),
                        reason,
                    )
                    .into(),
                ));
            }
        }
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
        // The owner's or the creator's reference may name the writer on the
        // referring side: `$ownerId` there is a write gate like any other,
        // and for the owner's the same identity as its value
        if Some(referring_property.as_str()) == reference_property {
            return Ok(invalid(
                "the referring property cannot be the reference property itself",
            ));
        }
        let declaring_document_type = document_type;
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
                    "only the referenced document's $ownerId, $creatorId and $id system \
                     properties may be agreed with",
                ));
            }
            if !matches!(
                referring_type,
                DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
            ) {
                return Ok(invalid(
                    "$ownerId, $creatorId and $id are identifiers, so the referring \
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
        // No stored document carries a transient value, so a referring
        // document could only agree by omitting its own side. The referring
        // side may be transient: it is judged on the transition, a write
        // gate like the writer's `$ownerId`. In place: inert before protocol
        // version 14, which alone reaches this module (contract create and
        // update state validation 1).
        if is_transient(referenced_document_type, referenced_property) {
            return Ok(invalid(
                "the referenced property is transient or inside a transient object: no \
                 stored document carries its value, so none could be agreed with",
            ));
        }
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

    Ok(SimpleConsensusValidationResult::new())
}
