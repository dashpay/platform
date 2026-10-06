use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    is_referenced_system_agreement_property, is_referring_system_agreement_property, is_transient,
    DocumentProperty, DocumentPropertyReferenceTarget, DocumentPropertyType,
    DocumentReferenceDeclaration, DocumentTypeRef, KeyReferenceIdentityProperty,
    PreallocatedKeySource, PropertyReference, ReferenceHolder, MAX_INDEX_SIZE,
};
use dpp::data_contract::DataContract;
use dpp::document::property_names::CREATOR_ID;
use dpp::errors::consensus::state::document::referenced_document_list_invalid_error::ReferencedDocumentListInvalidError;
use dpp::errors::consensus::state::document::referenced_document_lookup_invalid_error::ReferencedDocumentLookupInvalidError;
use dpp::errors::consensus::state::document::referenced_document_property_agreement_invalid_error::ReferencedDocumentPropertyAgreementInvalidError;
use dpp::errors::consensus::state::document::referenced_document_type_index_only_error::ReferencedDocumentTypeIndexOnlyError;
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
use crate::execution::validation::state_transition::common::document_reference_kind::document_reference_kind_mismatch;

/// Whether two property types hold the same KIND of value for agreement
/// purposes: sizes and other constraints may differ (both sides validated
/// their own documents already). The rule is `DocumentPropertyType::value_kind`,
/// shared with the key parts of a `refersTo` `findBy`.
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

/// The name of a preallocated index of `document_type` whose trees are keyed
/// by the referenced document's `referenced_property` through the agreement
/// pair naming it from `referring_property`, on the reference carried by
/// `reference_property`: creating a referenced document then writes that
/// property's value as a tree key of the index. `None` when no preallocated
/// index is keyed through the pair, or only through a `moderatedDocument`
/// binding the referenced type's removal record would not keep, which the
/// referenced document's insert never preallocates through
/// (`Index::preallocation_bindings_for_target`).
fn preallocated_index_keyed_by(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    reference_property: &str,
    referring_property: &str,
    referenced_property: &str,
) -> Option<String> {
    document_type
        .indexes()
        .values()
        .filter(|index| index.preallocated)
        .find(|index| {
            index
                .preallocation_bindings(document_type.flattened_properties(), contract.id())
                .iter()
                .any(|binding| {
                    // A referenced type the contract lacks is refused elsewhere
                    let kept = contract
                        .document_type_for_name(binding.target_document_type_name)
                        .map_or(true, |target| {
                            binding.is_kept_on_removal(target.moderator_deletion_kept_fields())
                        });
                    kept && binding.referring_property == reference_property
                        && index.properties.iter().zip(&binding.key_sources).any(
                            |(index_property, key_source)| {
                                index_property.name == referring_property
                                    && *key_source
                                        == PreallocatedKeySource::ReferencedDocumentProperty(
                                            referenced_property,
                                        )
                            },
                        )
                })
        })
        .map(|index| index.name.clone())
}

/// Checks every reference declaration of the given contract that carries
/// declaration content.
///
/// `permanentDocument`, `moderatedDocument` and `deletableDocument`: the
/// referenced contract must exist (the declaring contract itself when no
/// contract id is named, including when it names its own id) and the
/// referenced document type must exist in it and admit the declared kind (see
/// [`document_reference_kind_mismatch`]): for `permanentDocument` its
/// documents never leave state, for `moderatedDocument` they leave it only
/// through a moderator's recorded removal, for `deletableDocument` in any
/// other way. A declaration resolved by a document's id (no `findBy`, or
/// `inList`) may not name an indexOnly document type, whose documents cannot
/// be fetched by id.
/// Every `where` entry is checked for all three, and a `findBy` into another
/// contract's document type is checked against that type's indexes
/// (one into the declaring contract was checked by the contract parse). Self
/// references are checked against the in-flight contract, so a contract may
/// reference its own document types on creation; foreign contract fetches are
/// billed.
///
/// A `permanentDocument` with `inList`: the same checks (the type must forbid
/// deletion, `$id` named by `findBy`), plus, for a list in a document type of
/// another contract,
/// that `inList` is a stored typed array of identifiers fixed once a document
/// is written. One in the declaring contract was checked by the contract
/// parse.
///
/// A pair through which a preallocated index of the declaring type is keyed
/// needs a referenced property whose every value fits a tree key, at most
/// 255 bytes: creating a referenced document writes the value as one.
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
/// reference's is, first; the parser only admits an `identity`, a document
/// reference found by `findBy` or with `inList`, or an expression of them.
///
/// Each leaf of a reference expression (`anyOf` / `allOf`) is checked exactly
/// as the same target declared alone, in declared order, and every one of them
/// must pass: an `anyOf` lets a WRITE satisfy one operand, but each leaf has to
/// be a declaration that could hold. A `where` belongs to its own leaf and is
/// checked against that leaf's document type only.
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
        // identity or a document reference found by `findBy` or with `inList`
        // there, or an expression of them. Inert before protocol version 14: this module is only called
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
                // A string or byte array property revealed into a computed
                // `findBy` key is judged as an identifier's reference is: the
                // referenced type, its permanence, the `where` entries and the
                // `findBy`. Only the protocol version 14 parser produces one
                PropertyReference::Value(target) | PropertyReference::Revealed(target) => {
                    (target, declaration_path)
                }
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
        kind,
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

    // An indexOnly document type keeps its documents only as index entries: Drive refuses to
    // fetch one by its id, so a write could never resolve a reference that names its document
    // by id (one without `findBy`, or a list element, whose list's document is read by the id
    // `findBy` reads `$id` from) and would fail with an internal error instead. No kind of
    // reference fixes that, so this comes before the kind check. A `findBy` into such a type is
    // refused by its own check (`DocumentReferenceLookup::referenced_side_error`). In place in
    // generation 0, which every table selects: only parser generation 3, selected from
    // protocol version 14, admits an indexOnly document type, so no earlier version reaches
    // this refusal.
    if lookup.is_none() && referenced_document_type.index_only() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            ReferencedDocumentTypeIndexOnlyError::new(
                effective_contract_id,
                document_type_name.to_string(),
                declaration_path,
            )
            .into(),
        ));
    }

    // The three document references are disjoint: a `permanentDocument` one demands a
    // document type whose documents never leave state, a `moderatedDocument` one a type whose
    // documents leave it only through a moderator's recorded removal, a `deletableDocument`
    // one any other, so the declaration always states which guarantee the reference carries.
    // A document type moderators can delete from is no longer permanent whatever its
    // `canBeDeleted` says about a document's own owner, since a reference to it could dangle,
    // and neither is one whose documents the platform deletes when their `ttl` passes. Nothing
    // the kind reads can change on an update, so the answer holds for good.
    if let Some(error) = document_reference_kind_mismatch(
        kind,
        referenced_document_type.document_reference_kind(),
        effective_contract_id,
        document_type_name,
        &declaration_path,
    ) {
        return Ok(SimpleConsensusValidationResult::new_with_error(error));
    }

    // A `findBy`, on a document reference of either kind, must resolve
    // in the referenced document type: a unique index over exactly the
    // properties it names, filled from sources of the right kinds, with a
    // key that stays with the document it found. The contract parse checks
    // a `findBy` into the declaring contract under full validation, where
    // it sees every document type; only here is another contract's
    // document type in hand.
    if let Some(lookup) = lookup {
        if effective_contract_id != contract.id() {
            // Consuming deletes the found document with the create, an operation on the
            // create's own contract: a commitment in another contract is only read. In place
            // in generation 0, which every table selects: only the protocol version 14 parser
            // produces a lookup, let alone one that consumes
            if lookup.consume {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentLookupInvalidError::new(
                        declaration_path,
                        lookup.find_by_names(),
                        "consume deletes the document findBy finds with the create, so it is \
                         only allowed beside a findBy into the declaring contract"
                            .to_string(),
                    )
                    .into(),
                ));
            }
            if let Some(reason) =
                lookup.referenced_side_error(document_type, referenced_document_type)
            {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentLookupInvalidError::new(
                        declaration_path,
                        lookup.find_by_names(),
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

    // `where` declarations: both sides must exist, be
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
        // The write-time check compares single values, which a list is
        // not, so an agreement on one would never hold
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
        // A preallocated index keyed through this pair writes the referenced
        // document's value as a tree key when that document is created, so
        // every value the referenced property can hold must fit one; the
        // referring side is an index property, bounded by the index rules. In
        // place: inert before protocol version 14, which alone reaches this
        // module (contract create and update state validation 1).
        let keyed_index = reference_property.and_then(|reference_property| {
            preallocated_index_keyed_by(
                contract,
                document_type,
                reference_property,
                referring_property,
                referenced_property,
            )
        });
        if let Some(index_name) = keyed_index {
            let max_width = referenced
                .property_type
                .saturating_max_byte_size(platform_version)
                .ok()
                .flatten();
            if max_width.is_none_or(|width| usize::from(width) > MAX_INDEX_SIZE) {
                return Ok(invalid(&format!(
                    "the preallocated index {index_name} keys its trees by the referenced \
                     property's value, and a tree key holds at most {MAX_INDEX_SIZE} bytes, but \
                     the referenced property can hold values of up to {} bytes",
                    max_width.unwrap_or(u16::MAX)
                )));
            }
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::platform_value::{platform_value, Value};

    /// A contract with a `post` only moderators remove, whose removal records keep `kept`,
    /// and a `like` whose preallocated `byHashtagPost` is keyed by the post's hashtag through a
    /// `moderatedDocument` reference. Parsed as a stored contract is read, without the
    /// registration checks.
    fn contract_keeping(kept: Option<Value>) -> DataContract {
        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available")
            .with_moderation(Some(ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let moderator_abilities = match kept {
            Some(kept) => platform_value!({ "delete": true, "deleteKeepsFields": kept }),
            None => platform_value!({ "delete": true }),
        };
        let identifier = |position: u64| {
            platform_value!({
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier", "position": position,
            })
        };
        let mut post_id = identifier(0);
        post_id
            .set_value(
                "refersTo",
                platform_value!({
                    "type": "moderatedDocument",
                    "documentType": "post",
                    "where": { "hashtag": "hashtag" },
                }),
            )
            .expect("refersTo applies");
        DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Value::Identifier([7; 32]),
                "ownerId": Value::Identifier([8; 32]),
                "version": 1,
                "config": dpp::platform_value::to_value(config).expect("the config converts"),
                "documentSchemas": {
                    "post": {
                        "type": "object",
                        "documentsMutable": false,
                        "canBeDeleted": false,
                        "moderatorAbilities": moderator_abilities,
                        "properties": {
                            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
                        },
                        "required": ["hashtag"],
                        "additionalProperties": false,
                    },
                    "like": {
                        "type": "object",
                        "indexOnly": true,
                        "documentsMutable": false,
                        "canBeDeleted": true,
                        "properties": {
                            "postId": post_id,
                            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 1 },
                        },
                        "indices": [{
                            "name": "byHashtagPost",
                            "properties": [{ "hashtag": "asc" }, { "postId": "asc" }],
                            "terminal": "$ownerId",
                            "preallocated": true,
                        }],
                        "required": ["postId", "hashtag"],
                        "additionalProperties": false,
                    },
                },
            }),
            false,
            platform_version,
        )
        .expect("the contract parses")
    }

    #[test]
    fn should_key_a_preallocated_index_only_through_a_pair_the_record_keeps() {
        let keyed_by = |contract: &DataContract| {
            let like = contract
                .document_type_for_name("like")
                .expect("the like type");
            preallocated_index_keyed_by(contract, like, "postId", "hashtag", "hashtag")
        };
        assert_eq!(
            keyed_by(&contract_keeping(Some(platform_value!(["hashtag"])))),
            Some("byHashtagPost".to_string())
        );
        // The post's insert never preallocates through a pair its record drops, so no tree
        // is keyed by its value
        assert_eq!(keyed_by(&contract_keeping(None)), None);
    }
}
