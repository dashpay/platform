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
use dpp::data_contract::document_type::{
    is_referring_system_agreement_property, DocumentPropertyReferenceTarget,
    DocumentPropertyType, DocumentReferenceDeclaration, DocumentReferenceLookup, DocumentTypeRef,
    IdentityKeyReferenceRequirements, KeyReferenceIdentityProperty, ListElementReference,
    PropertyReference, ReferringWrite,
};
use dpp::data_contract::DataContract;
use dpp::document::property_names::{CREATOR_ID, OWNER_ID};
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
/// contract and token) and can be limited to changed fields for replace
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
    // The `listElement` references of the type, checked once every other
    // reference is, and the documents the scalar `permanentDocument`
    // references validated in this write fetched, by property path: a list
    // element read through one of them is checked against the document in
    // hand, so its check reads nothing more
    let mut list_elements: Vec<(&str, &ListElementReference, bool)> = Vec::new();
    let mut fetched_documents: BTreeMap<&str, Option<Document>> = BTreeMap::new();

    for (path, property) in document_type.flattened_properties() {
        // A reference is an identifier property's value, or each element of
        // a typed array of identifiers whose `items` declare it (protocol
        // version 14, the version whose document create and replace state
        // validation call this; no document type of an earlier version can
        // hold a typed array, so the element arm is never reached there)
        let Some(reference) = property.property_type.reference() else {
            continue;
        };
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
        // Checked below, once the document its list is read from is in hand
        if let Some(reference) = reference_target.as_list_element_reference() {
            list_elements.push((path.as_str(), reference, holds_elements));
            continue;
        }

        let bound_property_changed = if let Some(changed) = changed_fields {
            // Some targets bind a sibling property of the same document to
            // the reference; replacing that sibling must re-validate the
            // reference even when the reference property itself is untouched:
            // - a propertyAgreement pair binds each referring property;
            // - an identityPublicKey reference binds the key id property,
            //   since the referenced key is the (identity id, key id) pair
            //   and a freshly written key id must exist and not be disabled;
            // - a lookup binds every property its key reads, since the
            //   referenced document is the one the whole key finds.
            // A writer gate (an agreement keyed by `$ownerId`) is re-checked
            // on EVERY replace: the writer is transition metadata that never
            // appears among the changed fields, and either document may have
            // been transferred since the last write, so a replace of an
            // unrelated field by a now-unauthorized owner must still fail. A
            // lookup whose key reads `$ownerId` needs no such rule: its
            // declaring type can be neither transferred nor traded
            // (registration refuses it otherwise), so the writer never moves.
            // The same rules hold for the elements of a typed array, which
            // share one declaration: the array is one field, so a replace
            // that changes it re-validates the elements the stored list did
            // not hold, and a changed bound property, a writer gate or a
            // deletableDocument target re-validates them all.
            let bound_property_changed = match reference_target {
                DocumentPropertyReferenceTarget::PermanentDocument {
                    property_agreement, ..
                } => property_agreement.keys().any(|referring_property| {
                    is_referring_system_agreement_property(referring_property)
                        || is_changed_field(changed, referring_property)
                }),
                DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                    property_agreement,
                    lookup,
                    ..
                } => {
                    property_agreement.keys().any(|referring_property| {
                        is_referring_system_agreement_property(referring_property)
                            || is_changed_field(changed, referring_property)
                    }) || lookup_key_may_have_changed(lookup, changed)
                }
                // A deletableDocument reference is re-validated on EVERY
                // replace, touched or not: its target may have been deleted
                // since the last write, and a referring document is not
                // allowed to be rewritten around a dead reference. The
                // replace has to repoint it at a document that exists, or
                // clear it; leaving it (or pointing it at another missing
                // document) fails the existence check below. A writer gate
                // is therefore never evaluated against a missing document:
                // it is checked against the new target, or not at all once
                // the reference is cleared.
                DocumentPropertyReferenceTarget::DeletableDocument { .. } => true,
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property, ..
                } => is_changed_field(changed, key_id_property),
                DocumentPropertyReferenceTarget::Identity
                | DocumentPropertyReferenceTarget::Contract { .. }
                | DocumentPropertyReferenceTarget::Token
                | DocumentPropertyReferenceTarget::ListElement(_) => false,
            };
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

        // A scalar `permanentDocument` reference hands back the document it
        // fetched: only such a property can be a list element's
        // `documentProperty` (registration refuses any other)
        let keeps_document = !holds_elements
            && reference_target
                .as_any_document_reference()
                .is_some_and(|declaration| declaration.permanent);

        // The elements of a typed array are each checked as a single
        // reference is, in list order, and the first that fails refuses the
        // write with the error a single reference would give, naming the
        // element by its list path (`reasons[2]` for the third). The count
        // is bounded by `maxItems`, which registration counts against
        // `SystemLimits::max_references_per_document` with the type's other
        // references, and every fetch is billed as a single reference's is.
        // A replace re-validating the list only because it changed leaves
        // out the elements the stored list already held, unchanged
        // references, as an unchanged single reference is left alone; a
        // changed bound property, a writer gate or a deletableDocument
        // target re-validates them all. An element repeating an earlier one
        // has its outcome already, so it is not fetched again
        let skipped = if holds_elements && !bound_property_changed {
            stored_elements_v0(path, stored_values)
        } else {
            BTreeSet::new()
        };
        for value in reference_values_v0(path, holds_elements, document_data, skipped) {
            let (value_path, referenced_id) = match value {
                Ok(value) => value,
                Err(refusal) => return Ok(refusal),
            };
            let mut referenced_document = None;
            let result = validate_reference_v0(
                contract,
                document_type,
                document_data,
                owner_id,
                reference_target,
                referenced_id,
                &value_path,
                &mut referenced_contracts,
                keeps_document.then_some(&mut referenced_document),
                platform,
                block_info,
                transaction,
                execution_context,
                platform_version,
            )?;
            if !result.is_valid() {
                return Ok(result);
            }
            if keeps_document {
                fetched_documents.insert(path.as_str(), referenced_document);
            }
        }
    }

    for (path, reference, holds_elements) in list_elements {
        let result = validate_list_element_v0(
            contract,
            document_type,
            document_data,
            owner_id,
            path,
            reference,
            holds_elements,
            changed_fields,
            stored_values,
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

    Ok(SimpleConsensusValidationResult::new())
}

/// One identifier a reference declaration holds, with the path its errors
/// name, or the refusal of a value that is no identifier.
type ReferenceValue<'p> = Result<(Cow<'p, str>, [u8; 32]), SimpleConsensusValidationResult>;

/// The identifiers one reference declaration holds in `document_data`, in
/// order, each with the path its errors name: an identifier property's value
/// (none when it is not set; whether it may be absent is the document type's
/// required list), or each element of a typed array by its list path
/// (`reasons[2]` for the third; none when the list is absent or empty),
/// leaving out the elements `skipped` holds and every repeat of an earlier
/// one. A value that is no identifier ends the list with the refusal naming
/// it, after the values before it, so those are judged first, in order.
fn reference_values_v0<'p>(
    path: &'p str,
    holds_elements: bool,
    document_data: &BTreeMap<String, Value>,
    mut skipped: BTreeSet<[u8; 32]>,
) -> Vec<ReferenceValue<'p>> {
    let invalid = |value_path: String, reason: String| {
        SimpleConsensusValidationResult::new_with_error(
            InvalidIdentifierError::new(value_path, reason).into(),
        )
    };
    if !holds_elements {
        return match document_data.get_optional_identifier_at_path(path) {
            Ok(Some(value)) => vec![Ok((Cow::Borrowed(path), value))],
            Ok(None) => vec![],
            Err(err) => vec![Err(invalid(path.to_string(), err.to_string()))],
        };
    }
    let elements = match document_data.get_optional_at_path(path) {
        Ok(Some(Value::Array(elements))) => elements,
        Ok(None) => return vec![],
        Ok(Some(_)) => {
            return vec![Err(invalid(
                path.to_string(),
                "a typed array of identifiers must be a list".to_string(),
            ))]
        }
        Err(err) => return vec![Err(invalid(path.to_string(), err.to_string()))],
    };
    let mut values = Vec::with_capacity(elements.len());
    for (index, element) in elements.iter().enumerate() {
        let element_path = format!("{path}[{index}]");
        match element.to_hash256() {
            Ok(value) => {
                if skipped.insert(value) {
                    values.push(Ok((Cow::Owned(element_path), value)));
                }
            }
            Err(err) => {
                values.push(Err(invalid(element_path, err.to_string())));
                break;
            }
        }
    }
    values
}

/// The identifiers the stored document's typed array at `path` held, which a
/// replace that did not move the reference leaves out of its checks.
fn stored_elements_v0(
    path: &str,
    stored_values: Option<&BTreeMap<String, Value>>,
) -> BTreeSet<[u8; 32]> {
    match stored_values.map(|stored| stored.get_optional_at_path(path)) {
        Some(Ok(Some(Value::Array(stored_elements)))) => stored_elements
            .iter()
            .filter_map(|element| element.to_hash256().ok())
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// Checks one `listElement` reference of a write, declared at `path`: its
/// value, or each element of its typed array, must be an element of the list
/// held by the document its `documentProperty` refers to. A value that is not
/// (or one set while `documentProperty` is not) refuses the write with
/// `ReferencedEntityNotFoundError` naming it, an element by its list path.
///
/// On a replace it is checked when the value changed, or when
/// `documentProperty` may now find another document
/// ([`document_reference_may_move`], the rule its own reference is
/// re-validated by); only then are the elements the stored list already
/// held checked again, since the list is another document's. The list's
/// document is the one `documentProperty`'s reference fetched when it was
/// validated in this write (`fetched_documents`), so the check reads nothing
/// more; when that reference was left alone (a replace changing only the
/// value) the document is fetched here, billed as the reference's fetch is,
/// without judging the reference again. The list is collected once, so each
/// value is a set lookup.
#[allow(clippy::too_many_arguments)]
fn validate_list_element_v0<'a>(
    contract: &DataContract,
    document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    path: &str,
    reference: &'a ListElementReference,
    holds_elements: bool,
    changed_fields: Option<&BTreeSet<String>>,
    stored_values: Option<&BTreeMap<String, Value>>,
    fetched_documents: &mut BTreeMap<&'a str, Option<Document>>,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let document_property = reference.document_property.as_str();
    let list_may_have_moved = match changed_fields {
        None => true,
        Some(changed) => reference
            .document_property_declaration(&document_type)
            .map_or(true, |declaration| {
                document_reference_may_move(&declaration, document_property, changed)
            }),
    };
    let value_changed = changed_fields.is_none_or(|changed| is_changed_field(changed, path));
    if !list_may_have_moved && !value_changed {
        return Ok(SimpleConsensusValidationResult::new());
    }

    let skipped = if holds_elements && !list_may_have_moved {
        stored_elements_v0(path, stored_values)
    } else {
        BTreeSet::new()
    };
    let values = reference_values_v0(path, holds_elements, document_data, skipped);
    if values.is_empty() {
        return Ok(SimpleConsensusValidationResult::new());
    }

    let list_document = match fetched_documents.get(document_property) {
        Some(document) => document,
        None => {
            let resolved = resolve_list_document_v0(
                contract,
                document_type,
                document_data,
                owner_id,
                reference,
                &mut BTreeMap::new(),
                platform,
                block_info,
                transaction,
                execution_context,
                platform_version,
            )?;
            match resolved {
                Ok(document) => fetched_documents
                    .entry(document_property)
                    .or_insert(document),
                Err(refusal) => return Ok(refusal),
            }
        }
    };
    let listed_values = list_document
        .as_ref()
        .map(|document| reference.listed_values(document.properties()))
        .unwrap_or_default();

    for value in values {
        let (value_path, value) = match value {
            Ok(value) => value,
            Err(refusal) => return Ok(refusal),
        };
        if !listed_values.contains(&value) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ReferencedEntityNotFoundError::new(
                    Identifier::from(value),
                    DocumentPropertyReferenceTarget::ListElement(reference.clone()),
                    value_path.into_owned(),
                )
                .into(),
            ));
        }
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// The document holding the list of `reference`, found through its
/// `documentProperty`'s `permanentDocument` reference in `document_data`
/// without judging that reference: its contract and document type are
/// resolved and the document fetched by id or through the lookup, billed as
/// the reference's own fetch is, but neither deletability nor a
/// `propertyAgreement` is checked again. `None` when `documentProperty` is not
/// set, or carries no such reference (registration refuses that). The
/// reference was validated when it was written, and its document can never
/// be deleted.
#[allow(clippy::too_many_arguments)]
fn resolve_list_document_v0(
    contract: &DataContract,
    document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    reference: &ListElementReference,
    referenced_contracts: &mut BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>>,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<Result<Option<Document>, SimpleConsensusValidationResult>, Error> {
    let Ok(declaration) = reference.document_property_declaration(&document_type) else {
        return Ok(Ok(None));
    };
    let document_property = reference.document_property.as_str();
    let referenced_id = match document_data.get_optional_identifier_at_path(document_property) {
        Ok(Some(referenced_id)) => referenced_id,
        Ok(None) => return Ok(Ok(None)),
        Err(err) => {
            return Ok(Err(SimpleConsensusValidationResult::new_with_error(
                InvalidIdentifierError::new(document_property.to_string(), err.to_string()).into(),
            )))
        }
    };
    let effective_contract_id = declaration.contract_id.unwrap_or(contract.id());
    let type_not_found = || {
        SimpleConsensusValidationResult::new_with_error(
            ReferencedDocumentTypeNotFoundError::new(
                effective_contract_id,
                declaration.document_type_name.to_string(),
                document_property.to_string(),
            )
            .into(),
        )
    };
    let Some(referenced_contract) = referenced_contract_v0(
        contract,
        declaration.contract_id,
        referenced_contracts,
        platform,
        block_info,
        transaction,
        execution_context,
        platform_version,
    )?
    else {
        return Ok(Err(type_not_found()));
    };
    let Some(referenced_document_type) = referenced_contract
        .contract()
        .document_type_optional_for_name(declaration.document_type_name)
    else {
        return Ok(Err(type_not_found()));
    };
    fetch_referenced_document_v0(
        referenced_contract.contract(),
        referenced_document_type,
        declaration.lookup,
        referenced_id,
        document_data,
        owner_id,
        platform,
        block_info,
        transaction,
        execution_context,
        platform_version,
    )
    .map(Ok)
}

/// The contract a document reference's type lives in: the declaring contract
/// itself, already loaded for the transition, or another one.
enum ReferencedContract<'a> {
    Declaring(&'a DataContract),
    Other(Arc<DataContractFetchInfo>),
}

impl ReferencedContract<'_> {
    fn contract(&self) -> &DataContract {
        match self {
            ReferencedContract::Declaring(contract) => contract,
            ReferencedContract::Other(fetch_info) => &fetch_info.contract,
        }
    }
}

/// The contract `contract_id` names for a document reference of a document of
/// `contract`: `contract` itself when it names none or `contract`'s own id,
/// which costs no fetch, or another contract, fetched once per
/// `referenced_contracts` (shared among the elements of one array, misses
/// included) and billed even when it does not exist or was cached. `None` when
/// it does not exist.
#[allow(clippy::too_many_arguments)]
fn referenced_contract_v0<'a>(
    contract: &'a DataContract,
    contract_id: Option<Identifier>,
    referenced_contracts: &mut BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>>,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<Option<ReferencedContract<'a>>, Error> {
    let effective_contract_id = contract_id.unwrap_or(contract.id());
    if effective_contract_id == contract.id() {
        return Ok(Some(ReferencedContract::Declaring(contract)));
    }
    let fetch_info = match referenced_contracts.get(&effective_contract_id) {
        Some(resolved) => resolved.clone(),
        None => {
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
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            referenced_contracts.insert(effective_contract_id, fetch_info.clone());
            fetch_info
        }
    };
    Ok(fetch_info.map(ReferencedContract::Other))
}

/// The document a document reference's value finds in `referenced_document_type`:
/// the one whose id it is, or, for a `lookup`, the one the unique index finds
/// for the key assembled with the value as one part. Billed as a document
/// fetch either way.
#[allow(clippy::too_many_arguments)]
fn fetch_referenced_document_v0(
    referenced_contract: &DataContract,
    referenced_document_type: DocumentTypeRef<'_>,
    lookup: Option<&DocumentReferenceLookup>,
    referenced_id: [u8; 32],
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<Option<Document>, Error> {
    match lookup {
        None => fetch_document_with_id(
            platform.drive,
            referenced_contract,
            referenced_document_type,
            Identifier::from(referenced_id),
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        ),
        Some(lookup) => fetch_document_through_lookup(
            platform.drive,
            referenced_contract,
            referenced_document_type,
            lookup,
            Identifier::from(referenced_id),
            document_data,
            owner_id,
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        ),
    }
}

/// Checks one reference against platform state: the referenced id
/// `referenced_id`, declared by `reference_target`, is an identifier
/// property's value or one element of a typed array of them, and `path` is
/// how the errors name it (the property path, or the element's list path).
/// The target must exist and meet the declaration's contract requirements,
/// a referenced document's type must be deletable or not as declared, and
/// each `propertyAgreement` pair must hold between `document_data` (or the
/// writer `owner_id`) and the referenced document. Every read is billed to
/// `execution_context`; a foreign contract holding a referenced document
/// type is resolved through `referenced_contracts`, which the caller shares
/// among the elements of one array. A document reference found valid hands
/// the document it fetched to `referenced_document` when the caller passes
/// one, for the list elements read through it.
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
    referenced_document: Option<&mut Option<Document>>,
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
        DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id: referenced_contract_id,
            document_type_name,
            property_agreement,
        }
        | DocumentPropertyReferenceTarget::PermanentDocumentLookup {
            contract_id: referenced_contract_id,
            document_type_name,
            property_agreement,
            ..
        }
        | DocumentPropertyReferenceTarget::DeletableDocument {
            contract_id: referenced_contract_id,
            document_type_name,
            property_agreement,
        } => {
            let permanent = matches!(
                reference_target,
                DocumentPropertyReferenceTarget::PermanentDocument { .. }
                    | DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
            );
            // An absent contract id targets the declaring contract itself; the
            // declaring contract may also name its own id explicitly. Either
            // way it is already loaded for this transition, so no fetch is
            // billed for it. The elements of one typed array share their
            // declaration, so they resolve another contract once: the first
            // element's fetch is billed and the rest reuse it. A single
            // reference comes with a map of its own, one fetch
            let effective_contract_id = referenced_contract_id.unwrap_or(contract.id());
            let Some(referenced_contract) = referenced_contract_v0(
                contract,
                *referenced_contract_id,
                referenced_contracts,
                platform,
                block_info,
                transaction,
                execution_context,
                platform_version,
            )?
            else {
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

            let Some(referenced_document_type) = referenced_contract
                .contract()
                .document_type_optional_for_name(document_type_name)
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
                        document_type_name.clone(),
                        path.to_string(),
                    )
                    .into(),
                ));
            }
            if !permanent && !target_is_deletable {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentTypeNotDeletableError::new(
                        effective_contract_id,
                        document_type_name.clone(),
                        path.to_string(),
                    )
                    .into(),
                ));
            }

            // The value is the referenced document's id, unless the
            // (permanentDocument) declaration looks the document up through a
            // unique index of its type, with the value, or the element, as one
            // part of the key
            let lookup = match reference_target {
                DocumentPropertyReferenceTarget::PermanentDocumentLookup { lookup, .. } => {
                    Some(lookup)
                }
                _ => None,
            };
            let fetched_document = fetch_referenced_document_v0(
                referenced_contract.contract(),
                referenced_document_type,
                lookup,
                referenced_id,
                document_data,
                owner_id,
                platform,
                block_info,
                transaction,
                execution_context,
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
            if let Some(referenced_document) = &fetched_document {
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
                    // The referenced side may name one of the two system
                    // identifiers a document carries outside its data:
                    // `$ownerId`, which follows the document through
                    // transfers, and `$creatorId`, set once at creation
                    // and absent on document types that do not record
                    // it. Contract registration validated that either
                    // faces an identifier property on the referring
                    // side, and the key serializer below already encodes
                    // both names as 32-byte identifiers.
                    let referenced_value: Option<Cow<Value>> = match referenced_property.as_str() {
                        OWNER_ID => Some(Cow::Owned(Value::Identifier(
                            referenced_document.owner_id().to_buffer(),
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

            let exists = fetched_document.is_some();
            if let Some(referenced_document) = referenced_document {
                *referenced_document = fetched_document;
            }
            exists
        }
        // The batch validation checks list elements through
        // `validate_list_element_v0`, which reuses the document the
        // `documentProperty` reference fetched; a caller reaching here with
        // one is still judged correctly, the list's document fetched without
        // judging that reference again
        DocumentPropertyReferenceTarget::ListElement(reference) => {
            match resolve_list_document_v0(
                contract,
                document_type,
                document_data,
                owner_id,
                reference,
                referenced_contracts,
                platform,
                block_info,
                transaction,
                execution_context,
                platform_version,
            )? {
                Ok(list_document) => list_document.is_some_and(|list_document| {
                    reference
                        .listed_values(list_document.properties())
                        .contains(&referenced_id)
                }),
                Err(refusal) => return Ok(refusal),
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
/// while that property is not is refused. An unset key id is not validated;
/// whether it may be absent is the document type's required list.
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
        // that records creator ids, so a document of such a type has one
        KeyReferenceIdentityProperty::CreatorId => {
            creator_id.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a $creatorId key reference needs a document type that records creator ids",
            )))?
        }
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

/// Whether a replace may have moved the document reference declared at `path`
/// by `declaration` onto another document: the property itself changed, or,
/// for a lookup, a property its key reads (`lookup_key_may_have_changed`, the
/// rule the reference's own re-validation applies). A list element read
/// through the reference is then checked again, every element included, since
/// its list may be another document's.
fn document_reference_may_move(
    declaration: &DocumentReferenceDeclaration,
    path: &str,
    changed_fields: &BTreeSet<String>,
) -> bool {
    is_changed_field(changed_fields, path)
        || declaration
            .lookup
            .is_some_and(|lookup| lookup_key_may_have_changed(lookup, changed_fields))
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
