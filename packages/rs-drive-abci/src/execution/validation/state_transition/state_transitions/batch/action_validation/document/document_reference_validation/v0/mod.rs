use std::collections::{BTreeMap, BTreeSet};

use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::reference_lookup::owner_can_change;
use dpp::data_contract::document_type::{
    is_referring_system_agreement_property, DocumentPropertyReferenceTarget,
    DocumentPropertyType, DocumentReferenceDeclaration, DocumentReferenceLookup, DocumentTypeRef,
    IdentityKeyReferenceRequirements, KeyReferenceIdentityProperty, ListElementReference,
    PropertyReference,
    ReferenceCombinator, ReferenceHolder, ReferringWrite,
};
use dpp::data_contract::DataContract;
use dpp::document::property_names::{CREATOR_ID, ID, OWNER_ID};
use dpp::document::{Document, DocumentV0Getters};
use dpp::errors::consensus::state::document::referenced_document_property_mismatch_error::ReferencedDocumentPropertyMismatchError;
use dpp::errors::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_deletable_error::ReferencedDocumentTypeNotDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
use dpp::errors::consensus::state::document::referenced_contract_requirement_not_met_error::ReferencedContractRequirementNotMetError;
use dpp::errors::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
use dpp::errors::consensus::state::document::referenced_identity_key_disabled_error::ReferencedIdentityKeyDisabledError;
use dpp::errors::consensus::state::document::referenced_identity_key_not_found_error::ReferencedIdentityKeyNotFoundError;
use dpp::errors::consensus::state::document::referenced_identity_key_requirement_not_met_error::ReferencedIdentityKeyRequirementNotMetError;
use dpp::errors::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::KeyID;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use std::borrow::Cow;
use std::sync::Arc;
use drive::drive::contract::DataContractFetchInfo;
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
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::{
    fetch_document_through_lookup, fetch_document_with_id,
};
use crate::platform_types::platform::PlatformStateRef;

/// Versioned, stateful validation of document references using the v0 rules.
///
/// This performs existence checks for the supported reference targets (identity,
/// contract and token, documents, identity keys, and the leaves of a reference
/// expression combined by `anyOf` and `allOf`) and can be limited to changed fields for replace
/// transitions. It is intended to be called via the higher-level
/// `DocumentReferenceValidation` dispatcher that selects the version.
pub(crate) trait DocumentReferenceValidationV0 {
    #[allow(clippy::too_many_arguments)]
    fn deletable_document_reference_target_is_gone_v0(
        &self,
        property: &str,
        referenced_id: Identifier,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;

    #[allow(clippy::too_many_arguments)]
    fn validate_document_references_v0(
        &self,
        document_data: &BTreeMap<String, Value>,
        owner_id: Identifier,
        creator_id: Option<Identifier>,
        changed_fields: Option<&BTreeSet<String>>,
        stored_values: Option<&BTreeMap<String, Value>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentReferenceValidationV0 for DocumentBaseTransitionAction {
    fn deletable_document_reference_target_is_gone_v0(
        &self,
        property: &str,
        referenced_id: Identifier,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let contract_fetch_info = self.data_contract_fetch_info();
        let contract = &contract_fetch_info.contract;
        let Some(document_type) =
            contract.document_type_optional_for_name(self.document_type_name())
        else {
            return Ok(false);
        };
        let Some(DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id: referenced_contract_id,
                document_type_name,
                ..
            },
        )) = document_type
            .flattened_properties()
            .get(property)
            .map(|property| &property.property_type)
        else {
            // Not a deletableDocument reference: nothing can be "gone". A
            // typed array of deletableDocument references never gets here
            // either: the parser refuses one on an immutable property, the
            // only kind this exception serves
            return Ok(false);
        };

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
            let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "fee must exist when fetching a referenced contract with an epoch",
            )))?;
            // The cost is added even if the referenced contract does not exist or was cached
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            let Some(fetch_info) = fetch_info else {
                return Ok(true);
            };
            referenced_contract_fetch_info = fetch_info;
            &referenced_contract_fetch_info.contract
        };
        let Some(referenced_document_type) =
            referenced_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(true);
        };

        let referenced_document = fetch_document_with_id(
            platform.drive,
            referenced_contract,
            referenced_document_type,
            referenced_id,
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        )?;
        Ok(referenced_document.is_none())
    }

    fn validate_document_references_v0(
        &self,
        document_data: &BTreeMap<String, Value>,
        owner_id: Identifier,
        creator_id: Option<Identifier>,
        changed_fields: Option<&BTreeSet<String>>,
        stored_values: Option<&BTreeMap<String, Value>>,
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
            owner_id,
            creator_id,
            changed_fields,
            stored_values,
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
    owner_id: Identifier,
    creator_id: Option<Identifier>,
    changed_fields: Option<&BTreeSet<String>>,
    stored_values: Option<&BTreeMap<String, Value>>,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // The documents this write's references fetch by id, shared among them
    let mut fetched_documents = FetchedDocuments::default();

    // Whether a replace may be written by an owner other than the one who
    // wrote a reference: a transfer or a purchase hands the document on
    // without any write. Both flags are immutable on contract update
    let writer_can_change = owner_can_change(document_type);

    // A reference is the writer's (`ownerRefersTo`, whose value is the
    // document's `$ownerId` and which the errors name by that path), the
    // creator's (`creatorRefersTo`, `$creatorId`), an identifier property's
    // value, or each element of a typed array of identifiers whose `items`
    // declare it. The first two and the last are protocol version 14
    // declarations, the version whose document create and replace state
    // validation call this: no document type of an earlier version has an
    // owner or creator reference or a typed array, so none of those arms is
    // reached there. The owner and creator references follow the replace
    // rules of the target they declare, as a property's does: their value
    // never changes (the writer is the owner on a type that can be neither
    // transferred nor traded, which generation 3 requires of `ownerRefersTo`,
    // and the creator is set once), so a replace re-validates one when a
    // property its lookup or a `propertyAgreement` reads changed, or always
    // for a writer gate.
    for (holder, reference) in document_type.reference_declarations() {
        let path = holder.path();
        let (reference_target, holds_elements) = match reference {
            // A key reference on the key id property itself: the value is the
            // key id and the declaration names whose key it is. A transfer
            // itself is not checked, so the reference governs writing, not
            // holding; which replaces re-validate it depends on where the
            // identity comes from.
            PropertyReference::KeyId(reference) => {
                let identity_property = &reference.identity_property;
                if let Some(changed) = changed_fields {
                    let must_revalidate = match identity_property {
                        // The owner is transition metadata, never among the
                        // changed fields, and may have changed since the key
                        // id was written (a transfer or a purchase), so every
                        // replace re-validates, as a writer gate is
                        KeyReferenceIdentityProperty::OwnerId => true,
                        // The creator never changes
                        KeyReferenceIdentityProperty::CreatorId => is_changed_field(changed, path),
                        // The identity is in the document: either side of the
                        // (identity, key id) pair changing re-validates it
                        KeyReferenceIdentityProperty::Property(identity_path) => {
                            is_changed_field(changed, path)
                                || is_changed_field(changed, identity_path)
                        }
                    };
                    if !must_revalidate {
                        continue;
                    }
                }
                let result = validate_key_id_reference_v0(
                    path,
                    identity_property,
                    &reference.key_requirements,
                    document_type.name(),
                    contract.id(),
                    document_data,
                    owner_id,
                    creator_id,
                    platform,
                    transaction,
                    execution_context,
                    platform_version,
                )?;
                if !result.is_valid() {
                    return Ok(result);
                }
                continue;
            }
            PropertyReference::Value(target) => (target, false),
            PropertyReference::Elements { target, .. } => (target, true),
        };

        let bound_property_changed = if let Some(changed) = changed_fields {
            // Some targets bind a sibling property of the same document to
            // the reference; replacing that sibling must re-validate the
            // reference even when the reference property itself is untouched
            // (see `binds_a_changed_property`, which also covers the writer
            // gates, the contract owner requirements and the
            // deletableDocument targets re-checked on every replace). The
            // same rules hold for the elements of a typed array, which share
            // one declaration: the array is one field, so a replace that
            // changes it re-validates the elements the stored list did not
            // hold, and a changed bound property, a writer gate, a contract
            // owner requirement or a deletableDocument target re-validates
            // them all.
            let bound_property_changed =
                binds_a_changed_property(reference_target, changed, writer_can_change);
            if !is_changed_field(changed, path) && !bound_property_changed {
                continue;
            }
            bound_property_changed
        } else {
            false
        };

        // The referenced contracts this declaration resolved, so the
        // elements of a typed array fetch a foreign contract once
        let mut referenced_contracts = BTreeMap::new();

        if !holds_elements {
            let referenced_id = match holder {
                // The writer: an identity target has nothing to fetch, the
                // transition already proved the writer exists
                ReferenceHolder::Owner => {
                    if matches!(reference_target, DocumentPropertyReferenceTarget::Identity) {
                        continue;
                    }
                    owner_id.to_buffer()
                }
                // The creator: the writer on a create, which the transition
                // proved exists, and the stored creator on a replace, which
                // existed when it wrote the document (an identity is never
                // removed), so an identity target has nothing to fetch either
                ReferenceHolder::Creator => {
                    if matches!(reference_target, DocumentPropertyReferenceTarget::Identity) {
                        continue;
                    }
                    // Generation 3 admits `creatorRefersTo` only on a document
                    // type that records creator ids, and only when the type is
                    // created: adding it by an update is an incompatible
                    // schema change. So every document of such a type was
                    // written while its type recorded creator ids, and has
                    // one, unlike the documents a `$creatorId` key reference
                    // added by an update meets (see
                    // `validate_key_id_reference_v0`). The one way around it
                    // would be a well-formed stray `creatorRefersTo` key on a
                    // schema admitted by meta-schema v0 (protocol versions 1
                    // to 11), which generation 3 reads on load; the census of
                    // mainnet and testnet found none (see
                    // `try_from_schema_generation_3`)
                    creator_id
                        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "a creatorRefersTo declaration needs a document type that records \
                             creator ids",
                        )))?
                        .to_buffer()
                }
                ReferenceHolder::Property(path) => {
                    match document_data.get_optional_identifier_at_path(path) {
                        Ok(Some(referenced_id)) => referenced_id,
                        // A reference property that is not set is not validated; whether it
                        // may be absent at all is enforced by the document type's required
                        // fields
                        Ok(None) => continue,
                        Err(err) => {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidIdentifierError::new(path.to_string(), err.to_string())
                                    .into(),
                            ))
                        }
                    }
                }
            };
            let result = validate_reference_v0(
                contract,
                document_type,
                document_data,
                owner_id,
                reference_target,
                referenced_id,
                path,
                &mut referenced_contracts,
                &mut fetched_documents,
                platform,
                block_info,
                transaction,
                execution_context,
                platform_version,
            )?;
            if !result.is_valid() {
                return Ok(result);
            }
        } else {
            let elements = match document_data.get_optional_at_path(path) {
                Ok(Some(Value::Array(elements))) => elements,
                // An absent list, like an absent reference, is not
                // validated; an empty one has nothing to validate
                Ok(None) => continue,
                Ok(Some(_)) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidIdentifierError::new(
                            path.to_string(),
                            "a typed array of identifiers must be a list".to_string(),
                        )
                        .into(),
                    ))
                }
                Err(err) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidIdentifierError::new(path.to_string(), err.to_string()).into(),
                    ))
                }
            };
            // Each element is checked as a single reference is, in list
            // order, and the first that fails refuses the write with the
            // error a single reference would give, naming the element by
            // its list path (`reasons[2]` for the third). The count is
            // bounded by `maxItems`, which registration counts against
            // `SystemLimits::max_references_per_document` with the
            // type's other references, and every fetch is billed as a
            // single reference's is. A replace re-validating the list
            // only because it changed leaves out the elements the stored
            // list already held, unchanged references, as an unchanged
            // single reference is left alone; a changed bound property,
            // a writer gate, a contract owner requirement or a
            // deletableDocument target re-validates them all. An element
            // repeating an earlier one has its outcome already, so it is
            // not fetched again
            let mut checked: BTreeSet<[u8; 32]> = BTreeSet::new();
            if !bound_property_changed {
                if let Some(Ok(Some(Value::Array(stored_elements)))) =
                    stored_values.map(|stored| stored.get_optional_at_path(path))
                {
                    checked.extend(
                        stored_elements
                            .iter()
                            .filter_map(|element| element.to_hash256().ok()),
                    );
                }
            }
            for (index, element) in elements.iter().enumerate() {
                let element_path = format!("{path}[{index}]");
                let referenced_id = match element.to_hash256() {
                    Ok(referenced_id) => referenced_id,
                    Err(err) => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidIdentifierError::new(element_path, err.to_string()).into(),
                        ))
                    }
                };
                if !checked.insert(referenced_id) {
                    continue;
                }
                let result = validate_reference_v0(
                    contract,
                    document_type,
                    document_data,
                    owner_id,
                    reference_target,
                    referenced_id,
                    &element_path,
                    &mut referenced_contracts,
                    &mut fetched_documents,
                    platform,
                    block_info,
                    transaction,
                    execution_context,
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

/// The documents one write's references fetched by id, by (contract,
/// document id) and then document type name, each with the lists collected
/// from it for its list elements, by list path: a document two references
/// name (a `permanentDocument` reference to a charter and a `listElement`
/// read through the same `$id` property, or the elements of one typed array)
/// is fetched and billed once, and a list is collected into a set once. A
/// hit allocates nothing. Lookup results are not kept: a key is not an id.
#[derive(Default)]
struct FetchedDocuments {
    documents: BTreeMap<(Identifier, Identifier), BTreeMap<String, FetchedDocument>>,
}

/// One document fetched by id (`None` when it does not exist) and the lists
/// read from it.
struct FetchedDocument {
    document: Option<Document>,
    lists: BTreeMap<String, BTreeSet<[u8; 32]>>,
}

impl FetchedDocuments {
    /// The document `document_type_name` of `contract_id` holds under
    /// `document_id`, fetched with `fetch` the first time this write asks.
    fn document(
        &mut self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        fetch: impl FnOnce() -> Result<Option<Document>, Error>,
    ) -> Result<Option<&Document>, Error> {
        let by_type = self
            .documents
            .entry((contract_id, document_id))
            .or_default();
        if !by_type.contains_key(document_type_name) {
            let document = fetch()?;
            by_type.insert(
                document_type_name.to_string(),
                FetchedDocument {
                    document,
                    lists: BTreeMap::new(),
                },
            );
        }
        Ok(by_type
            .get(document_type_name)
            .and_then(|fetched| fetched.document.as_ref()))
    }

    /// Whether `value` is an element of `list_reference`'s list on the
    /// document [`Self::document`] fetched for the same key, collected into a
    /// set the first time; `false` when no such document was fetched or it
    /// does not exist.
    fn is_listed(
        &mut self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        list_reference: &ListElementReference,
        value: &[u8; 32],
    ) -> bool {
        let Some(FetchedDocument {
            document: Some(document),
            lists,
        }) = self
            .documents
            .get_mut(&(contract_id, document_id))
            .and_then(|by_type| by_type.get_mut(document_type_name))
        else {
            return false;
        };
        if !lists.contains_key(&list_reference.in_list) {
            lists.insert(
                list_reference.in_list.clone(),
                list_reference.listed_values(document.properties()),
            );
        }
        lists
            .get(&list_reference.in_list)
            .is_some_and(|listed| listed.contains(value))
    }
}

/// Whether a replace that changed `changed_fields` must re-validate a
/// reference declared by `reference_target` although the reference property
/// itself is untouched, because the target binds a sibling property of the
/// same document, or because it is re-checked on every replace:
/// - a propertyAgreement pair binds each referring property;
/// - an identityPublicKey reference binds the key id property, since the
///   referenced key is the (identity id, key id) pair and a freshly written
///   key id must exist and not be disabled;
/// - a lookup binds every property its key reads, since the referenced
///   document is the one the whole key finds.
///
/// A writer gate (an agreement keyed by `$ownerId`) is re-checked on EVERY
/// replace: the writer is transition metadata that never appears among the
/// changed fields, and either document may have been transferred since the
/// last write, so a replace of an unrelated field by a now-unauthorized owner
/// must still fail. A contract reference's `owner` requirement relates the
/// writer to the referenced contract, so it is re-checked on every replace
/// too, when `writer_can_change`: the declaring type's documents can be
/// transferred or traded. A lookup whose key reads `$ownerId` needs no such
/// rule: its declaring type can be neither transferred nor traded
/// (registration refuses it otherwise), so the writer never moves. A
/// reference expression (`anyOf` / `allOf`) is re-validated when any of its
/// leaves would be, and then as a whole: which operands hold may have changed.
fn binds_a_changed_property(
    reference_target: &DocumentPropertyReferenceTarget,
    changed_fields: &BTreeSet<String>,
    writer_can_change: bool,
) -> bool {
    match reference_target {
        DocumentPropertyReferenceTarget::PermanentDocument {
            property_agreement, ..
        } => property_agreement.keys().any(|referring_property| {
            is_referring_system_agreement_property(referring_property)
                || is_changed_field(changed_fields, referring_property)
        }),
        DocumentPropertyReferenceTarget::PermanentDocumentLookup {
            property_agreement,
            lookup,
            ..
        } => {
            property_agreement.keys().any(|referring_property| {
                is_referring_system_agreement_property(referring_property)
                    || is_changed_field(changed_fields, referring_property)
            }) || lookup_key_may_have_changed(lookup, changed_fields)
        }
        // A deletableDocument reference is re-validated on EVERY replace,
        // touched or not: its target may have been deleted since the last
        // write, and a referring document is not allowed to be rewritten
        // around a dead reference. The replace has to repoint it at a
        // document that exists, or clear it; leaving it (or pointing it at
        // another missing document) fails the existence check. A writer gate
        // is therefore never evaluated against a missing document: it is
        // checked against the new target, or not at all once the reference
        // is cleared. Through a lookup the same holds, the document the key
        // finds now being the target: an expression holding one is
        // re-validated on every replace through the walk below, and an
        // `ownerRefersTo` one gates every replace on the writer's document
        // still existing. Reached from protocol version 14 only, the version
        // whose parser produces it.
        DocumentPropertyReferenceTarget::DeletableDocument { .. }
        | DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. } => true,
        DocumentPropertyReferenceTarget::IdentityPublicKey {
            key_id_property, ..
        } => is_changed_field(changed_fields, key_id_property),
        // A list element binds the properties its agreement reads, the
        // `$id` pair's among them: a changed one may name another document,
        // whose list is then checked against every value
        DocumentPropertyReferenceTarget::ListElement(reference) => reference
            .property_agreement
            .keys()
            .any(|referring_property| {
                is_referring_system_agreement_property(referring_property)
                    || is_changed_field(changed_fields, referring_property)
            }),
        // A contract reference's `owner` requirement is judged against the
        // writer, transition metadata: a transfer or a purchase hands the
        // document to an owner the requirement never checked, without any
        // write, so on a type whose documents can change owner the
        // reference is re-validated on every replace, as a writer gate is,
        // and the new owner has to repoint it at a contract that meets the
        // requirement for them (or clear it, where it is optional); the
        // generation 3 parser refuses such a reference on an immutable
        // property of that type, which could not be repointed. On any
        // other type every replace is written by the owner the requirement
        // was checked against, and a contract's owner never changes, so the
        // outcome stands and no contract fetch is billed for it. The other
        // requirements (moderation, minimumAgeSeconds,
        // minimumSecondsSinceUpdate, readonly, keepsHistory, ownerProtected)
        // are facts about the referenced contract, not the writer: they
        // never bring a reference back and stay checked when the reference
        // is written, a create or a replace changing it. A reference the
        // owner requirement brings back is checked whole, as a writer
        // gate's is. In place in generation 0, reached from protocol version
        // 14 only: the document create and replace state validations that
        // call this validation run at that version alone, and only its
        // parser produces `contractRequirements`
        DocumentPropertyReferenceTarget::Contract {
            contract_requirements,
        } => writer_can_change && contract_requirements.owner.is_some(),
        DocumentPropertyReferenceTarget::Identity | DocumentPropertyReferenceTarget::Token => false,
        // In place in generation 0, reached from protocol version 14 only,
        // the only version whose parser produces an expression. A contract
        // target is never an operand (the parser combines identity and
        // document targets only), but the flag is passed down so an operand
        // is judged as the same target alone
        DocumentPropertyReferenceTarget::AnyOf(operands)
        | DocumentPropertyReferenceTarget::AllOf(operands) => operands
            .operands()
            .iter()
            .any(|operand| binds_a_changed_property(operand, changed_fields, writer_can_change)),
    }
}

/// Checks one referenced value against its declaration `reference_target`:
/// the value `referenced_id` is an identifier property's value or one
/// element of a typed array of them, and `path` is how the errors name it
/// (the property path, or the element's list path). A single target (a leaf)
/// is checked by [`validate_reference_target_v0`]. A reference expression is
/// evaluated operand by operand in declared order, each operand a leaf or a
/// nested expression evaluated the same way: an `anyOf` stops at the first
/// operand that holds, and when none does the result is the last operand's;
/// an `allOf` stops at the first operand that fails, with that operand's
/// result. So a refusal is always the error a leaf declared alone would
/// give, and the author's order decides which one a writer is shown. Every
/// read is billed as it is made, those of the operands that failed included.
/// The recursion is as deep as the expression, which registration keeps
/// within `SystemLimits::max_reference_expression_depth`.
///
/// In place in generation 0, which every table selects: its callers, the
/// document create and replace state validations, reach it from protocol
/// version 14 only, and only that version's parser produces an expression, so
/// every earlier write goes straight to the leaf check it always ran.
#[allow(clippy::too_many_arguments)]
fn validate_reference_v0(
    contract: &DataContract,
    document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    reference_target: &DocumentPropertyReferenceTarget,
    referenced_id: [u8; 32],
    path: &str,
    referenced_contracts: &mut BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>>,
    fetched_documents: &mut FetchedDocuments,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let Some((combinator, operands)) = reference_target.combinator() else {
        return validate_reference_target_v0(
            contract,
            document_type,
            document_data,
            owner_id,
            reference_target,
            referenced_id,
            path,
            referenced_contracts,
            fetched_documents,
            platform,
            block_info,
            transaction,
            execution_context,
            platform_version,
        );
    };
    // An empty list would hold vacuously as an allOf
    if operands.operands().is_empty() {
        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "a refersTo reference expression lists at least two operands, which the parser \
             enforces",
        )));
    }
    let mut result = SimpleConsensusValidationResult::new();
    for operand in operands.operands() {
        result = validate_reference_v0(
            contract,
            document_type,
            document_data,
            owner_id,
            operand,
            referenced_id,
            path,
            referenced_contracts,
            fetched_documents,
            platform,
            block_info,
            transaction,
            execution_context,
            platform_version,
        )?;
        let decided = match combinator {
            ReferenceCombinator::AnyOf => result.is_valid(),
            ReferenceCombinator::AllOf => !result.is_valid(),
        };
        if decided {
            return Ok(result);
        }
    }
    // Every operand was checked: for an anyOf none held and this is the last
    // one's refusal, for an allOf all held
    Ok(result)
}

/// Checks one reference against platform state for a single target: the
/// referenced id `referenced_id`, declared by `reference_target`, is an
/// identifier property's value or one element of a typed array of them, and
/// `path` is how the errors name it (the property path, or the element's list
/// path). The target must exist and meet the declaration's contract
/// requirements, a referenced document's type must be deletable or not as
/// declared, and each `propertyAgreement` pair must hold between
/// `document_data` (or the writer `owner_id`) and the referenced document.
/// Every read is billed to `execution_context`; a foreign contract holding a
/// referenced document type is resolved through `referenced_contracts`,
/// which the caller shares among the elements of one array and the leaves
/// of one reference expression.
#[allow(clippy::too_many_arguments)]
fn validate_reference_target_v0(
    contract: &DataContract,
    document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    reference_target: &DocumentPropertyReferenceTarget,
    referenced_id: [u8; 32],
    path: &str,
    referenced_contracts: &mut BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>>,
    fetched_documents: &mut FetchedDocuments,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
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
        DocumentPropertyReferenceTarget::Contract {
            contract_requirements,
        } => {
            let (fee, referenced_contract) = platform.drive.get_contract_with_fetch_info_and_fee(
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

            match referenced_contract {
                None => false,
                Some(fetch_info) => {
                    // The declaration's requirements are checked against the contract
                    // just fetched and the write itself (its owner and block time), so
                    // they cost no further read; the first unmet one refuses the write
                    let write = ReferringWrite {
                        owner_id,
                        block_time_ms: block_info.time_ms,
                    };
                    if let Some(requirement) =
                        contract_requirements.first_unmet_by(&fetch_info.contract, write)
                    {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedContractRequirementNotMetError::new(
                                Identifier::from(referenced_id),
                                requirement.field().to_string(),
                                requirement.required(),
                                path.to_string(),
                            )
                            .into(),
                        ));
                    }
                    true
                }
            }
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
        // The document references: found by the value (their id), through a
        // lookup, or, for a list element, by the document its `$id`
        // agreement pair names, whose list must then hold the value
        DocumentPropertyReferenceTarget::PermanentDocument { .. }
        | DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
        | DocumentPropertyReferenceTarget::DeletableDocument { .. }
        | DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. }
        | DocumentPropertyReferenceTarget::ListElement(_) => {
            let Some(DocumentReferenceDeclaration {
                contract_id: referenced_contract_id,
                document_type_name,
                property_agreement,
                permanent,
                lookup,
                in_list: _,
            }) = reference_target.as_any_document_reference()
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "every document reference target carries a document reference declaration",
                )));
            };
            let list_reference = reference_target.as_list_element_reference();
            // An absent contract id targets the declaring contract itself; the
            // declaring contract may also name its own id explicitly. Either
            // way it is already loaded for this transition, so no fetch is
            // billed for it
            let effective_contract_id = referenced_contract_id.unwrap_or(contract.id());
            let referenced_contract_fetch_info;
            let referenced_contract = if effective_contract_id == contract.id() {
                contract
            } else {
                // The elements of one typed array share their declaration,
                // so they resolve its contract once: the first element's
                // fetch is billed and the rest reuse it. A single
                // reference comes with a map of its own, one fetch
                let fetch_info = match referenced_contracts.get(&effective_contract_id) {
                    Some(resolved) => resolved.clone(),
                    None => {
                        let (fee, fetch_info) =
                            platform.drive.get_contract_with_fetch_info_and_fee(
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

                        referenced_contracts.insert(effective_contract_id, fetch_info.clone());
                        fetch_info
                    }
                };

                let Some(fetch_info) = fetch_info else {
                    // A missing contract and a missing document type resolve to the
                    // same failure: the declared document type could not be found
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedDocumentTypeNotFoundError::new(
                            effective_contract_id,
                            document_type_name.to_string(),
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
                        document_type_name.to_string(),
                        path.to_string(),
                    )
                    .into(),
                ));
            };

            // A `permanentDocument` reference admits only document types
            // whose documents can never be deleted: `canBeDeleted` is
            // immutable on contract updates and document types can not be
            // removed, so a reference validated here can never dangle. A
            // `deletableDocument` reference makes no such promise, and
            // admits only document types whose documents CAN be deleted:
            // the referenced document must exist now, and may be deleted
            // later. Deletable means by anyone, the contract's moderators
            // included (`canBeDeletedByModerators`), as at contract
            // registration
            let target_is_deletable = referenced_document_type.documents_can_be_deleted()
                || referenced_document_type.documents_can_be_deleted_by_moderators();
            if permanent && target_is_deletable {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentTypeDeletableError::new(
                        effective_contract_id,
                        document_type_name.to_string(),
                        path.to_string(),
                    )
                    .into(),
                ));
            }
            if !permanent && !target_is_deletable {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentTypeNotDeletableError::new(
                        effective_contract_id,
                        document_type_name.to_string(),
                        path.to_string(),
                    )
                    .into(),
                ));
            }

            // The document: the one whose id the value is, or the one the
            // (permanentDocument) lookup finds through a unique index of its
            // type with the value as one key part, or, for a list element, the
            // one whose id the `$id` pair's property holds (unset: no document,
            // so no list holds the value). A by-id fetch is shared with every
            // other reference of the same document in this write
            let document_id = match list_reference {
                None => Some(referenced_id),
                Some(list_reference) => {
                    let Some(id_property) = list_reference.document_id_property() else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "a listElement reference carries a $id agreement pair, which the \
                             parser enforces",
                        )));
                    };
                    match document_data.get_optional_identifier_at_path(id_property) {
                        Ok(Some(document_id)) => Some(document_id),
                        Ok(None) => None,
                        Err(err) => {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidIdentifierError::new(
                                    id_property.to_string(),
                                    err.to_string(),
                                )
                                .into(),
                            ))
                        }
                    }
                }
            };
            let looked_up_document;
            let referenced_document: Option<&Document> = match (lookup, document_id) {
                (_, None) => None,
                (Some(lookup), Some(document_id)) => {
                    looked_up_document = fetch_document_through_lookup(
                        platform.drive,
                        referenced_contract,
                        referenced_document_type,
                        lookup,
                        Identifier::from(document_id),
                        document_data,
                        owner_id,
                        &block_info.epoch,
                        execution_context,
                        transaction,
                        platform_version,
                    )?;
                    looked_up_document.as_ref()
                }
                (None, Some(document_id)) => fetched_documents.document(
                    effective_contract_id,
                    document_type_name,
                    Identifier::from(document_id),
                    || {
                        fetch_document_with_id(
                            platform.drive,
                            referenced_contract,
                            referenced_document_type,
                            Identifier::from(document_id),
                            &block_info.epoch,
                            execution_context,
                            transaction,
                            platform_version,
                        )
                    },
                )?,
            };

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
            if let Some(referenced_document) = referenced_document {
                for (referring_property, referenced_property) in property_agreement {
                    // A list element's document is the one its `$id` pair
                    // names, fetched by that very id: the pair holds
                    if list_reference.is_some() && referenced_property == ID {
                        continue;
                    }
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
                    // The referring side is a schema property of the document
                    // being written, or the writer's own `$ownerId`, which
                    // lives on the transition rather than in its data: that
                    // pair is a write gate, and the writer is `owner_id`.
                    let referring_value: Option<Cow<Value>> = if referring_property == OWNER_ID {
                        Some(Cow::Owned(Value::Identifier(owner_id.to_buffer())))
                    } else {
                        let Ok(referring_value) =
                            document_data.get_optional_at_path(referring_property)
                        else {
                            return Ok(mismatch());
                        };
                        referring_value.map(Cow::Borrowed)
                    };
                    // The referenced side may name one of the system
                    // identifiers a document carries outside its data:
                    // `$ownerId`, which follows the document through
                    // transfers, `$creatorId`, set once at creation and
                    // absent on document types that do not record it and on
                    // documents written before their type did (an absent
                    // side, judged below like any other), and `$id`, the
                    // document's own (the pair a list element is found by,
                    // which holds by construction). Contract
                    // registration validated that each faces an identifier
                    // property on the referring side, and the key serializer
                    // below already encodes the names as 32-byte identifiers.
                    let referenced_value: Option<Cow<Value>> = match referenced_property.as_str() {
                        OWNER_ID => Some(Cow::Owned(Value::Identifier(
                            referenced_document.owner_id().to_buffer(),
                        ))),
                        ID => Some(Cow::Owned(Value::Identifier(
                            referenced_document.id().to_buffer(),
                        ))),
                        CREATOR_ID => referenced_document.creator_id().map(|creator_id| {
                            Cow::Owned(Value::Identifier(creator_id.to_buffer()))
                        }),
                        _ => {
                            let Ok(referenced_value) = referenced_document
                                .properties()
                                .get_optional_at_path(referenced_property)
                            else {
                                return Ok(mismatch());
                            };
                            referenced_value.map(Cow::Borrowed)
                        }
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
                        &referring_value,
                        platform_version,
                    ) else {
                        return Ok(mismatch());
                    };
                    let Ok(referenced_encoded) = referenced_document_type.serialize_value_for_key(
                        referenced_property,
                        &referenced_value,
                        platform_version,
                    ) else {
                        return Ok(mismatch());
                    };
                    if referring_encoded != referenced_encoded {
                        return Ok(mismatch());
                    }
                }
            }

            // A list element exists when the document does and its list
            // holds the value; the list is collected once per document and
            // path, so each value is a set lookup
            let document_exists = referenced_document.is_some();
            match (list_reference, document_id) {
                (Some(list_reference), Some(document_id)) if document_exists => fetched_documents
                    .is_listed(
                        effective_contract_id,
                        document_type_name,
                        Identifier::from(document_id),
                        list_reference,
                        &referenced_id,
                    ),
                (Some(_), _) => false,
                (None, _) => document_exists,
            }
        }
        DocumentPropertyReferenceTarget::IdentityPublicKey {
            key_id_property,
            key_requirements,
        } => {
            // The referenced key id is carried by the named sibling property
            let key_id: KeyID = match document_data.get_optional_integer_at_path(key_id_property) {
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

            let result = validate_referenced_identity_key_v0(
                Identifier::from(referenced_id),
                key_id,
                path,
                key_requirements,
                document_type.name(),
                contract.id(),
                platform,
                transaction,
                execution_context,
                platform_version,
            )?;
            if !result.is_valid() {
                return Ok(result);
            }

            true
        }
        // `validate_reference_v0` evaluates reference expressions and passes
        // only their leaves here
        DocumentPropertyReferenceTarget::AnyOf(_) | DocumentPropertyReferenceTarget::AllOf(_) => {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a reference expression reached the leaf check, which only its evaluator calls",
            )))
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

    Ok(SimpleConsensusValidationResult::new())
}

/// A key reference declared on the key id property at `path`
/// (`DocumentPropertyType::KeyIdWithReference`): the value is the key id, and
/// `identity_property` names whose key it is. For `$ownerId` that is the
/// writer, `owner_id`, whose existence the transition already proved, and for
/// `$creatorId` the document's creator, `creator_id` (the writer on a create,
/// the stored creator on a replace), so the key fetch is the only read; for a
/// property path the identity is read from the document, and a key id set
/// while that property is not is refused. A key id set on a document that
/// records no creator, one written before its type recorded creator ids, is
/// refused the same way. An unset key id is not validated; whether it may be
/// absent is the document type's required list.
#[allow(clippy::too_many_arguments)]
fn validate_key_id_reference_v0(
    path: &str,
    identity_property: &KeyReferenceIdentityProperty,
    key_requirements: &IdentityKeyReferenceRequirements,
    document_type_name: &str,
    declaring_contract_id: Identifier,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    creator_id: Option<Identifier>,
    platform: &PlatformStateRef,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let key_id: KeyID = match document_data.get_optional_integer_at_path(path) {
        Ok(Some(key_id)) => key_id,
        Ok(None) => return Ok(SimpleConsensusValidationResult::new()),
        // The declaring property is the key id property itself
        Err(err) => {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ReferencedKeyIdPropertyInvalidError::new(
                    path.to_string(),
                    path.to_string(),
                    err.to_string(),
                )
                .into(),
            ))
        }
    };

    let identity_id = match identity_property {
        KeyReferenceIdentityProperty::OwnerId => owner_id,
        // Contract registration admits `$creatorId` only on a document type
        // that records creator ids, but a document written before its type
        // did records none: no type did before protocol version 10, nor one
        // of a contract whose config was still version 0. A contract update
        // may add a property carrying this reference to such a type, so a
        // replace setting the key id of an old document names no identity,
        // and is refused as a key id set while its identity property is not.
        // In place in generation 0, which every table selects: its callers,
        // the document create and replace state validations, reach it from
        // protocol version 14 only, the only version whose parser produces a
        // key reference, so no earlier write gets here
        KeyReferenceIdentityProperty::CreatorId => match creator_id {
            Some(creator_id) => creator_id,
            None => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedKeyIdPropertyInvalidError::new(
                        path.to_string(),
                        path.to_string(),
                        "the document records no $creatorId: it was created before its \
                         document type recorded creator ids"
                            .to_string(),
                    )
                    .into(),
                ))
            }
        },
        KeyReferenceIdentityProperty::Property(identity_path) => {
            match document_data.get_optional_identifier_at_path(identity_path) {
                Ok(Some(identity_id)) => Identifier::from(identity_id),
                Ok(None) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedKeyIdPropertyInvalidError::new(
                            path.to_string(),
                            path.to_string(),
                            format!("the identity property {identity_path} is not set"),
                        )
                        .into(),
                    ))
                }
                Err(err) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidIdentifierError::new(identity_path.clone(), err.to_string()).into(),
                    ))
                }
            }
        }
    };

    validate_referenced_identity_key_v0(
        identity_id,
        key_id,
        path,
        key_requirements,
        document_type_name,
        declaring_contract_id,
        platform,
        transaction,
        execution_context,
        platform_version,
    )
}

/// The state check both `identityPublicKey` forms share: key `key_id` of
/// `identity_id`, referenced from `path`, must exist, not be disabled and
/// meet the declaration's `key_requirements`. A missing identity and a
/// missing key resolve to the same failure, the referenced key could not be
/// found. Keys can never be removed, so an existing reference can not
/// dangle; a disabled key is still rejected for fresh writes. The
/// requirements are checked against the key just fetched, so they cost no
/// further read, and the first unmet one refuses the write; a bound names
/// the declaring contract and one of its own document types (checked when
/// the contract was registered), so the check needs nothing beyond the key
/// and the contract in hand.
#[allow(clippy::too_many_arguments)]
fn validate_referenced_identity_key_v0(
    identity_id: Identifier,
    key_id: KeyID,
    path: &str,
    key_requirements: &IdentityKeyReferenceRequirements,
    document_type_name: &str,
    declaring_contract_id: Identifier,
    platform: &PlatformStateRef,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    execution_context.add_operation(ValidationOperation::RetrieveIdentity(
        RetrieveIdentityInfo::one_key(),
    ));

    let Some(key) = platform
        .drive
        .fetch_identity_keys::<OptionalSingleIdentityPublicKeyOutcome>(
            IdentityKeysRequest::new_specific_key_query(&identity_id.to_buffer(), key_id),
            transaction,
            platform_version,
        )?
    else {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            ReferencedIdentityKeyNotFoundError::new(identity_id, key_id, path.to_string()).into(),
        ));
    };

    if key.is_disabled() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            ReferencedIdentityKeyDisabledError::new(identity_id, key_id, path.to_string()).into(),
        ));
    }

    if let Some(requirement) = key_requirements.first_unmet_by(&key, declaring_contract_id) {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            ReferencedIdentityKeyRequirementNotMetError::new(
                document_type_name.to_string(),
                path.to_string(),
                identity_id,
                key_id,
                requirement.field().to_string(),
                requirement.required(),
                requirement.actual_of(&key),
            )
            .into(),
        ));
    }

    Ok(SimpleConsensusValidationResult::new())
}

/// Whether a replace may have changed the key of a lookup: a property the key
/// reads changed. The reference's own value is covered by the changed-field
/// check of the property itself, and a `$ownerId` part is fixed, since
/// registration admits it only on a type that cannot be transferred or traded.
fn lookup_key_may_have_changed(
    lookup: &DocumentReferenceLookup,
    changed_fields: &BTreeSet<String>,
) -> bool {
    lookup
        .referring_properties()
        .any(|path| is_changed_field(changed_fields, path))
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
