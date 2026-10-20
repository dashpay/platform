use std::collections::{BTreeMap, BTreeSet};

use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::{
    DocumentReferencePreimageInvalidError, InvalidDocumentTypeError,
};
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use dpp::data_contract::document_type::reference_lookup::owner_can_change;
use dpp::data_contract::document_type::{
    is_path_listed, is_referring_system_agreement_property, DocumentPropertyReferenceTarget,
    DocumentPropertyType, DocumentReferenceDeclaration, DocumentReferenceKind,
    DocumentReferenceLookup, DocumentTypeRef,
    IdentityKeyReferenceRequirements, KeyReferenceIdentityProperty, ListElementReference,
    ChooseFirstOrder, LookupChooseFirst, PropertyReference,
    ReferenceCombinator, ReferenceHolder, ReferringWrite,
};
use dpp::data_contract::config::moderation::{kept_value_at, ContractDocumentRemoval};
use dpp::data_contract::document_type::property_constraints::{
    referenced_document_value, referenced_document_values, referenced_fields, DocumentSystemValues,
};
use dpp::data_contract::document_type::{
    typed_array_property, where_lists_elements_read, where_lists_equal, WhereBoundRead,
    WhereCondition, WhereLiteralKind,
};
use dpp::data_contract::DataContract;
use dpp::document::property_names::{ID, OWNER_ID};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::errors::consensus::state::document::referenced_document_preceded_error::ReferencedDocumentPrecededError;
use dpp::errors::consensus::state::document::referenced_document_property_mismatch_error::ReferencedDocumentPropertyMismatchError;
use dpp::errors::consensus::state::document::referenced_document_removed_error::ReferencedDocumentRemovedError;
use dpp::errors::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
use dpp::errors::consensus::state::document::referenced_contract_requirement_not_met_error::ReferencedContractRequirementNotMetError;
use dpp::errors::consensus::state::document::referenced_document_requirement_not_met_error::ReferencedDocumentRequirementNotMetError;
use dpp::errors::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
use dpp::errors::consensus::state::document::referenced_identity_key_disabled_error::ReferencedIdentityKeyDisabledError;
use dpp::errors::consensus::state::document::referenced_identity_key_not_found_error::ReferencedIdentityKeyNotFoundError;
use dpp::errors::consensus::state::document::referenced_identity_key_requirement_not_met_error::ReferencedIdentityKeyRequirementNotMetError;
use dpp::errors::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::KeyID;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::Error as ValueError;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::sync::Arc;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, OptionalSingleIdentityPublicKeyOutcome,
};
use drive::query::{TransactionArg, WhereClause, WhereOperator};
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    ConsumedDocument, DocumentCreateTransitionActionAccessorsV0,
};
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use std::slice;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::common::document_reference_kind::document_reference_kind_mismatch;
use dpp::data_contract::document_type::methods::{read_data_at_path, RepeatedKey};
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::ConsumedLookupDocument;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::{
    fetch_document_through_lookup, fetch_document_with_id, fetch_documents_holding_in_order,
    hash_lookup_key, holds_key_part, BilledLookupKey,
};
use crate::execution::validation::state_transition::common::validate_document_not_expired::document_is_expired;
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
        referring_system: &DocumentSystemValues,
        changed_fields: Option<&BTreeSet<String>>,
        stored_values: Option<&BTreeMap<String, Value>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        derived_index_values: Option<&mut BTreeMap<String, Value>>,
        referenced_documents: &mut BTreeMap<String, Value>,
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
        referring_system: &DocumentSystemValues,
        changed_fields: Option<&BTreeSet<String>>,
        stored_values: Option<&BTreeMap<String, Value>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        derived_index_values: Option<&mut BTreeMap<String, Value>>,
        referenced_documents: &mut BTreeMap<String, Value>,
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
            referring_system,
            changed_fields,
            stored_values,
            platform,
            block_info,
            consumed_documents,
            derived_index_values,
            referenced_documents,
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
    referring_system: &DocumentSystemValues,
    changed_fields: Option<&BTreeSet<String>>,
    stored_values: Option<&BTreeMap<String, Value>>,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    consumed_documents: &mut Vec<ConsumedLookupDocument>,
    derived_index_values: Option<&mut BTreeMap<String, Value>>,
    referenced_documents: &mut BTreeMap<String, Value>,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // The documents this write's references fetch by id, shared among them
    let mut fetched_documents = FetchedDocuments::default();

    // The foreign contracts this write's references resolved, so each is
    // fetched, and billed, once whatever number of declarations, elements of a
    // typed array and leaves of an expression name it. Reached from protocol
    // version 14 only (document create state validation 2 and replace state
    // validation 1), where a reference a rule reads through is checked on
    // every replace
    let mut referenced_contracts = BTreeMap::new();

    // What the type's `propertyConstraints` rules read of the documents its references
    // point at, by reference property (protocol version 14). Every replace judges every
    // rule, so each such reference is checked on every replace, and once it holds the
    // values the rules read of the document its check read are recorded in
    // `referenced_documents`, for the rules to be judged on: no other read is made
    let rule_reads = referenced_fields(document_type.property_constraints());

    // A create is the one write with no changed fields, and the only one a lookup with a
    // computed key, which reveals a commitment, is judged at
    let is_create = changed_fields.is_none();

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
    // property its `findBy` or `where` reads changed, or always
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
            // A string or byte array property whose value its `findBy` reads in
            // place of an id: as `"."`, filling an index property holding the same
            // kind of value, or revealed into a computed key, a commitment judged
            // when the document is created only. An unset one is not validated. In
            // place in generation 0: only the protocol version 14 parser produces
            // one
            PropertyReference::Revealed(target) => {
                let ReferenceHolder::Property(property_path) = holder else {
                    continue;
                };
                let hash_key = target
                    .as_any_document_reference()
                    .and_then(|declaration| declaration.lookup)
                    .and_then(|lookup| lookup.hash_key());
                // A replace re-validates a value read as is when it would an
                // identifier's reference (see `binds_a_changed_property`), and a
                // computed key never
                if let Some(changed) = changed_fields {
                    if hash_key.is_some()
                        || (!is_changed_field(changed, path)
                            && !rule_reads.contains_key(path)
                            && !binds_a_changed_property(target, changed, writer_can_change))
                    {
                        continue;
                    }
                }
                // Read as the stored document keeps it: a map on the way holding the
                // property's key more than once is refused (the create's structure
                // validation already refused it for a computed key), never taken for
                // an unset property
                let preimage_invalid = || {
                    Ok(SimpleConsensusValidationResult::new_with_error(
                        DocumentReferencePreimageInvalidError::new(
                            document_type.name().clone(),
                            path.to_string(),
                            path.to_string(),
                            "the value cannot be revealed into its findBy key".to_string(),
                        )
                        .into(),
                    ))
                };
                let value = match read_data_at_path(document_data, property_path) {
                    Ok(Some(value)) => value,
                    Ok(None) => continue,
                    Err(RepeatedKey) => return preimage_invalid(),
                };
                // The value is no id. A computed key is hashed and billed once here,
                // the property named by its path in the key's params, and its hash
                // stands for the value in the errors; the create's structure
                // validation refused a value it could not reveal, before any read. A
                // value read as is has no hash to stand for it, so the errors carry
                // an all-zero id beside its path. The writer is passed in place in
                // generation 0, inert before protocol version 14: only meta-schema v3
                // admits a computed key at all, and the `"$ownerId"` param that reads
                // the writer
                let (referenced_id, billed_key) = match hash_key {
                    Some((_, key)) => {
                        let Some(billed_key) = hash_lookup_key(
                            key,
                            document_type,
                            None,
                            owner_id,
                            document_data,
                            execution_context,
                        ) else {
                            return preimage_invalid();
                        };
                        (billed_key.digest(), Some(billed_key))
                    }
                    None => ([0; 32], None),
                };
                let result = validate_reference_v0(
                    contract,
                    document_type,
                    document_data,
                    owner_id,
                    referring_system,
                    target,
                    referenced_id,
                    Some(RevealedValue { value, billed_key }),
                    path,
                    None,
                    &mut referenced_contracts,
                    &mut fetched_documents,
                    platform,
                    block_info,
                    is_create,
                    consumed_documents,
                    RuleReads::of(&rule_reads, path, referenced_documents),
                    transaction,
                    execution_context,
                    platform_version,
                )?;
                if !result.is_valid() {
                    return Ok(result);
                }
                continue;
            }
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
            if !is_changed_field(changed, path)
                && !bound_property_changed
                && !rule_reads.contains_key(path)
            {
                continue;
            }
            bound_property_changed
        } else {
            false
        };

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
            let kept_value_changed_fields =
                kept_value_changed_fields(changed_fields, stored_values, path, &referenced_id);
            let result = validate_reference_v0(
                contract,
                document_type,
                document_data,
                owner_id,
                referring_system,
                reference_target,
                referenced_id,
                None,
                path,
                kept_value_changed_fields,
                &mut referenced_contracts,
                &mut fetched_documents,
                platform,
                block_info,
                is_create,
                consumed_documents,
                RuleReads::of(&rule_reads, path, referenced_documents),
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
                let kept_value_changed_fields =
                    kept_value_changed_fields(changed_fields, stored_values, path, &referenced_id);
                // No rule reads through the elements of a typed array, which point at
                // several documents (registration refuses it)
                let result = validate_reference_v0(
                    contract,
                    document_type,
                    document_data,
                    owner_id,
                    referring_system,
                    reference_target,
                    referenced_id,
                    None,
                    &element_path,
                    kept_value_changed_fields,
                    &mut referenced_contracts,
                    &mut fetched_documents,
                    platform,
                    block_info,
                    is_create,
                    consumed_documents,
                    None,
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

    // The values of the type's derived index properties (protocol version 14), from the
    // documents this write's references fetched by id, for Drive to key a create by. A
    // reference left out holds none; a document not fetched is left for Drive to read.
    if let Some(derived_index_values) = derived_index_values {
        for (name, derived) in document_type.derived_index_properties() {
            let value = match document_data.get(&derived.reference_property) {
                None | Some(Value::Null) => Value::Null,
                Some(reference) => {
                    let Ok(referenced_id) = reference.to_identifier() else {
                        continue;
                    };
                    let Some(Some(referenced)) = fetched_documents
                        .documents
                        .get(&(contract.id(), referenced_id))
                        .and_then(|by_type| by_type.get(&derived.referenced_document_type_name))
                        .map(|fetched| fetched.document.as_ref())
                    else {
                        continue;
                    };
                    derived.field.value_in(referenced)?
                }
            };
            derived_index_values.insert(name.clone(), value);
        }
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// The documents one write's references fetched by id, by (contract,
/// document id) and then document type name, each with the lists collected
/// from it for its list elements, by list path: a document two references
/// name (a `permanentDocument` reference to a charter and one with `inList`
/// found through the same `$id` property, or the elements of one typed array)
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
/// - a `where` entry binds each referring property;
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
    // A lookup with a computed key is judged on a create only: nothing it reads can change,
    // and the commitment it found may be gone. Only a replace asks, which a document type
    // whose documents are never replaced does not reach. In place in generation 0, reached
    // from protocol version 14 only, the only version whose parser produces one
    if reference_target
        .as_any_document_reference()
        .and_then(|declaration| declaration.lookup)
        .is_some_and(|lookup| lookup.is_checked_on_create_only(true))
    {
        return false;
    }
    // The `where` of a document reference asks a replace about it when a
    // referring property it reads changed, and on every replace when it reads
    // a value that changes without a write: the writer (a writer gate, or a
    // `notEqual` bounded by it), or the time or heights of the last update or
    // transfer bounding an ordering (protocol version 14)
    let where_binds = |declaration: DocumentReferenceDeclaration| {
        declaration.where_is_rechecked_on_every_replace()
            || declaration
                .where_referring_properties()
                .any(|referring_property| is_changed_field(changed_fields, referring_property))
    };
    match reference_target {
        // A moderated reference is kept as a permanent one is: its document
        // leaves state only on a moderator's record, which the reference then
        // resolves to, so nothing but its own `where` asks a replace about it
        DocumentPropertyReferenceTarget::PermanentDocument { .. }
        | DocumentPropertyReferenceTarget::ModeratedDocument { .. } => reference_target
            .as_any_document_reference()
            .is_some_and(where_binds),
        DocumentPropertyReferenceTarget::PermanentDocumentLookup { lookup, .. } => {
            reference_target
                .as_any_document_reference()
                .is_some_and(where_binds)
                || lookup_key_may_have_changed(lookup, changed_fields)
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
        DocumentPropertyReferenceTarget::ListElement(_) => reference_target
            .as_any_document_reference()
            .is_some_and(where_binds),
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

/// What the document type's `propertyConstraints` rules read of the document
/// one reference finds (`{ "referenced": [reference, field] }`), and where the
/// values go once the reference holds: under the reference property, in the map
/// the rules are judged on. Only a single reference to one document, found by
/// id or by a `findBy` without a function, carries one; registration refuses a
/// rule reading through any other.
struct RuleReads<'a> {
    reference: &'a str,
    fields: &'a BTreeSet<&'a str>,
    values: &'a mut BTreeMap<String, Value>,
}

impl<'a> RuleReads<'a> {
    /// What the rules read through the reference at `path`, `None` when they
    /// read nothing through it.
    fn of(
        rule_reads: &'a BTreeMap<&'a str, BTreeSet<&'a str>>,
        path: &'a str,
        values: &'a mut BTreeMap<String, Value>,
    ) -> Option<Self> {
        rule_reads.get(path).map(|fields| RuleReads {
            reference: path,
            fields,
            values,
        })
    }

    /// Records what the rules read of `document`, the document the reference
    /// found.
    fn record_document(self, document: &Document) -> Result<(), Error> {
        let values = referenced_document_values(document, self.fields.iter().copied())
            .map_err(|error| Error::Protocol(error.into()))?;
        self.values.insert(self.reference.to_string(), values);
        Ok(())
    }
}

/// What a string or byte array property carrying a reference hands its lookup
/// in place of an id: its `value`, which `"."` reads, and, when a `findBy`
/// function computes the key, the hash already computed and billed.
#[derive(Clone, Copy)]
struct RevealedValue<'a> {
    value: &'a Value,
    billed_key: Option<BilledLookupKey>,
}

/// Checks one referenced value against its declaration `reference_target`:
/// the value `referenced_id` is an identifier property's value or one
/// element of a typed array of them, or stands for the value of a string or
/// byte array property, `revealed`, which its lookup reads instead, and
/// `path` is how the errors name it (the property path, or the element's list
/// path). A single target (a leaf)
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
    referring_system: &DocumentSystemValues,
    reference_target: &DocumentPropertyReferenceTarget,
    referenced_id: [u8; 32],
    revealed: Option<RevealedValue<'_>>,
    path: &str,
    kept_value_changed_fields: Option<&BTreeSet<String>>,
    referenced_contracts: &mut BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>>,
    fetched_documents: &mut FetchedDocuments,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    is_create: bool,
    consumed_documents: &mut Vec<ConsumedLookupDocument>,
    rule_reads: Option<RuleReads<'_>>,
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
            referring_system,
            reference_target,
            referenced_id,
            revealed,
            path,
            kept_value_changed_fields,
            referenced_contracts,
            fetched_documents,
            platform,
            block_info,
            is_create,
            consumed_documents,
            rule_reads,
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
    // The commitments recorded for consumption before this expression: a
    // branch that fails consumes nothing, even when another branch then lets
    // the write through
    let consumed_before = consumed_documents.len();
    let mut result = SimpleConsensusValidationResult::new();
    for operand in operands.operands() {
        let consumed_before_operand = consumed_documents.len();
        // No rule reads through an expression, which may hold through one
        // document or another (registration refuses it)
        result = validate_reference_v0(
            contract,
            document_type,
            document_data,
            owner_id,
            referring_system,
            operand,
            referenced_id,
            revealed,
            path,
            kept_value_changed_fields,
            referenced_contracts,
            fetched_documents,
            platform,
            block_info,
            is_create,
            consumed_documents,
            None,
            transaction,
            execution_context,
            platform_version,
        )?;
        if !result.is_valid() {
            consumed_documents.truncate(consumed_before_operand);
        }
        let decided = match combinator {
            ReferenceCombinator::AnyOf => result.is_valid(),
            ReferenceCombinator::AllOf => !result.is_valid(),
        };
        if decided {
            // An allOf that fails keeps nothing its earlier operands recorded
            if !result.is_valid() {
                consumed_documents.truncate(consumed_before);
            }
            return Ok(result);
        }
    }
    // Every operand was checked: for an anyOf none held and this is the last
    // one's refusal, each operand's records already dropped; for an allOf all
    // held
    Ok(result)
}

/// Checks one reference against platform state for a single target: the
/// referenced id `referenced_id`, declared by `reference_target`, is an
/// identifier property's value or one element of a typed array of them, or
/// stands for the value of a string or byte array property, `revealed`, which
/// the lookup reads instead (see [`RevealedValue`]), and `path` is how the
/// errors name it (the property path, or the element's list path). The target must exist and meet the declaration's contract
/// requirements, a referenced document's type must be deletable or not as
/// declared, and each `where` entry must hold between
/// `document_data` (or the writer `owner_id`) and the referenced document.
/// Every read is billed to `execution_context`; a foreign contract holding a
/// referenced document type is resolved through `referenced_contracts`,
/// which the caller shares among the elements of one array and the leaves
/// of one reference expression.
///
/// A lookup with a computed key reveals a commitment: it is judged on a
/// create only (`is_create`), and holds on a replace without a read, since
/// registration made everything it reads fixed once written and the
/// commitment it found may have been consumed or deleted since. On a create
/// the document it finds must also meet the reference's `minimumAgeBlocks`,
/// and when the reference declares `consume` the document is pushed onto
/// `consumed_documents` once the leaf holds.
///
/// `kept_value_changed_fields` is `Some` with the fields a replace changed
/// when the value is one the stored document already held, which the replace
/// re-validates for a bound property or a writer gate, and `None` on a create
/// and for a value the write sets. A `moderatedDocument` whose document a
/// moderator removed resolves to its removal record for such a kept value
/// alone.
#[allow(clippy::too_many_arguments)]
fn validate_reference_target_v0(
    contract: &DataContract,
    document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    referring_system: &DocumentSystemValues,
    reference_target: &DocumentPropertyReferenceTarget,
    referenced_id: [u8; 32],
    revealed: Option<RevealedValue<'_>>,
    path: &str,
    kept_value_changed_fields: Option<&BTreeSet<String>>,
    referenced_contracts: &mut BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>>,
    fetched_documents: &mut FetchedDocuments,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    is_create: bool,
    consumed_documents: &mut Vec<ConsumedLookupDocument>,
    rule_reads: Option<RuleReads<'_>>,
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
        | DocumentPropertyReferenceTarget::ModeratedDocument { .. }
        | DocumentPropertyReferenceTarget::ListElement(_) => {
            let Some(DocumentReferenceDeclaration {
                contract_id: referenced_contract_id,
                document_type_name,
                property_agreement,
                where_conditions,
                kind,
                lookup,
                in_list: _,
            }) = reference_target.as_any_document_reference()
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "every document reference target carries a document reference declaration",
                )));
            };
            // A commitment is revealed once, by the create; nothing the lookup reads can
            // change on a replace. In place in generation 0: only the protocol version 14
            // parser produces a lookup judged on the create alone
            if !is_create
                && lookup.is_some_and(|lookup| {
                    lookup.is_checked_on_create_only(document_type.documents_mutable())
                })
            {
                return Ok(SimpleConsensusValidationResult::new());
            }
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
                // Every reference of the write resolves a contract once: the
                // first fetch is billed and the rest reuse it
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
            // whose documents never leave state: what could make them leave
            // it is immutable on contract updates and document types can not
            // be removed, so a reference validated here can never dangle. A
            // `moderatedDocument` reference admits only document types whose
            // documents leave state through a moderator's recorded removal
            // alone, so it resolves to the document or to its removal record.
            // A `deletableDocument` reference makes no promise, and admits
            // every other document type: the referenced document must exist
            // now, and may be deleted later, by its owner, by a moderator
            // leaving no record, or by the platform for a type declaring a
            // `ttl`. As at contract registration
            //
            // A `deletableDocument` reference to a moderated type is the one
            // pairing registration refuses that a write lets through: it
            // promises less than the type keeps, and a contract registered
            // on a protocol version 14 network before the moderated kind
            // existed may hold one, which stays writable
            let admitted_kind = match (kind, referenced_document_type.document_reference_kind()) {
                (DocumentReferenceKind::Deletable, DocumentReferenceKind::Moderated) => {
                    DocumentReferenceKind::Deletable
                }
                (_, admitted_kind) => admitted_kind,
            };
            if let Some(error) = document_reference_kind_mismatch(
                kind,
                admitted_kind,
                effective_contract_id,
                document_type_name,
                path,
            ) {
                return Ok(SimpleConsensusValidationResult::new_with_error(error));
            }

            // The document: the one whose id the value is, or the one the
            // (permanentDocument) lookup finds through a unique index of its
            // type with the value as one key part, or, for a list element, the
            // one whose id the property `findBy` reads `$id` from holds (unset: no document,
            // so no list holds the value). A by-id fetch is shared with every
            // other reference of the same document in this write
            let document_id = match list_reference {
                None => Some(referenced_id),
                Some(list_reference) => {
                    let Some(id_property) = list_reference.document_id_property() else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "a reference with inList carries a findBy $id, which the parser \
                             enforces",
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
                    // `"."` reads the id, or the value of the string or byte array
                    // property carrying the reference, whose computed key was hashed
                    // and billed already
                    let id_value;
                    let reference_value = match revealed {
                        Some(revealed) => revealed.value,
                        None => {
                            id_value = Value::Identifier(document_id);
                            &id_value
                        }
                    };
                    looked_up_document = fetch_document_through_lookup(
                        platform.drive,
                        referenced_contract,
                        referenced_document_type,
                        document_type,
                        lookup,
                        reference_value,
                        document_data,
                        owner_id,
                        revealed.and_then(|revealed| revealed.billed_key),
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

            // A moderated document a moderator removed, named by a value the
            // stored document already held: the reference resolves to the
            // document's removal record, read and billed here, and holds (see
            // `validate_pairs_against_removal`). A value the write sets must
            // name a document in state, as for the other kinds: a removed
            // document can not be newly referred to. A record marked restored
            // describes a document that is live again, so beside a missing
            // document it is no resolution, and the reference fails below as
            // a missing one
            if let (DocumentReferenceKind::Moderated, None, Some(changed_fields)) =
                (kind, referenced_document, kept_value_changed_fields)
            {
                let (removal_fee, removal) =
                    platform.drive.fetch_contract_document_removal_with_fee(
                        effective_contract_id,
                        document_type_name,
                        Identifier::from(referenced_id),
                        &block_info.epoch,
                        transaction,
                        platform_version,
                    )?;
                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(removal_fee));
                if let Some(removal) = removal.filter(|removal| !removal.is_restored()) {
                    return validate_pairs_against_removal(
                        property_agreement,
                        where_conditions,
                        &removal,
                        referenced_document_type,
                        Identifier::from(referenced_id),
                        changed_fields,
                        document_data,
                        owner_id,
                        referring_system,
                        path,
                        rule_reads,
                        execution_context,
                        platform_version,
                    );
                }
            }

            // Property agreement: the referenced document is already in
            // hand for the existence check, so comparing the declared
            // pairs adds no reads. The two sides, which registration made
            // one value kind, are compared as single values
            // (`Value::same_scalar_data`), so an identifier stored as bytes
            // and one carried as an identifier or as an array of `U8`s
            // compare equal, as do an integer carried at one width and
            // stored at another, and a
            // `number` carried as an integer and stored as a float. Not as
            // tree keys: those map the empty string to `"\0"`'s key and
            // hold no value past 255 bytes, which no agreement bounds.
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
                    // property on the referring side, so the two compare as
                    // identifiers below.
                    let Ok(referenced_value) =
                        referenced_document_value(referenced_document, referenced_property)
                    else {
                        return Ok(mismatch());
                    };
                    // Two typed arrays, which registration pairs with nothing
                    // else, are equal as lists: the same elements in the same
                    // order, each compared by value, a list either document
                    // leaves out the empty one. In place in generation 0: a
                    // pair of lists registers from protocol version 14 only
                    let lists = typed_array_property(
                        referenced_document_type.flattened_properties(),
                        referenced_property,
                    )
                    .is_some();
                    if lists {
                        bill_where_list_elements(
                            where_lists_elements_read(
                                referring_value.as_deref(),
                                referenced_value.as_deref(),
                            ),
                            execution_context,
                            platform_version,
                        )?;
                        if !where_lists_equal(
                            referring_value.as_deref(),
                            referenced_value.as_deref(),
                        ) {
                            return Ok(mismatch());
                        }
                        continue;
                    }
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
                    // In place in generation 0, which every table selects: a
                    // `where` only parses from protocol version
                    // 14 (`apply_property_reference` 0), so before it no
                    // document type carries a pair to reach this comparison
                    if !referring_value.same_scalar_data(&referenced_value) {
                        return Ok(mismatch());
                    }
                }
                // The other `where` entries (protocol version 14): a literal the
                // referenced side must equal or be one of, or a bound it must differ
                // from or compare with, read from the document already in hand. A
                // literal reads as the referenced property's kind says, of the type
                // just resolved, since it may be another contract's
                for (referenced_property, condition) in where_conditions {
                    let Ok(referenced_value) =
                        referenced_document_value(referenced_document, referenced_property)
                    else {
                        return Ok(condition_mismatch(path, condition, referenced_property));
                    };
                    let kind = WhereLiteralKind::of_referenced(
                        referenced_document_type,
                        referenced_property,
                    );
                    let Ok(bound) =
                        where_bound_value(condition, document_data, owner_id, referring_system)
                    else {
                        return Ok(condition_mismatch(path, condition, referenced_property));
                    };
                    bill_where_list_elements(
                        condition.list_elements_read(referenced_value.as_deref(), bound.as_deref()),
                        execution_context,
                        platform_version,
                    )?;
                    if !condition.holds(referenced_value.as_deref(), kind, bound.as_deref()) {
                        return Ok(condition_mismatch(path, condition, referenced_property));
                    }
                }
            }

            // The commitment a computed key found: it must be old enough, the first valid one
            // among those to the same values when the reference declares `chooseFirst`, and
            // the create deletes it when the reference consumes it. Its age is judged in
            // blocks, from its `$createdAtBlockHeight` (registration demands the type record
            // one) to the block of the create, so 1 means an earlier block; a document
            // recording none never meets it. Only a lookup the protocol version 14 parser
            // produced has any of them
            if let (Some(lookup), Some(found)) = (lookup, referenced_document) {
                if let Some(blocks) = lookup.minimum_age_blocks {
                    let old_enough = found
                        .created_at_block_height()
                        .and_then(|created_at| block_info.height.checked_sub(created_at))
                        .is_some_and(|age| age >= u64::from(blocks));
                    if !old_enough {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedDocumentRequirementNotMetError::new(
                                found.id(),
                                "minimumAgeBlocks".to_string(),
                                blocks.to_string(),
                                path.to_string(),
                            )
                            .into(),
                        ));
                    }
                }
                // Judged on the create alone, so here a create: registration admits
                // `chooseFirst` only beside a `findBy` function
                if let Some(choose_first) = &lookup.choose_first {
                    let result = validate_choose_first_v0(
                        choose_first,
                        found,
                        referenced_contract,
                        referenced_document_type,
                        document_type,
                        document_data,
                        owner_id,
                        referenced_id,
                        reference_target,
                        path,
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
                if lookup.consume {
                    consumed_documents.push(ConsumedLookupDocument {
                        document: ConsumedDocument {
                            document_id: found.id(),
                            document_type_name: document_type_name.to_string(),
                        },
                        referenced_id: Identifier::from(referenced_id),
                        reference_target: reference_target.clone(),
                        path: path.to_string(),
                    });
                }
            }

            // What the type's rules read of the document, from the document in
            // hand: the reference holds, every check above having passed. A list
            // element reference never carries rule reads (registration refuses it)
            if let (Some(rule_reads), Some(found), None) =
                (rule_reads, referenced_document, list_reference)
            {
                rule_reads.record_document(found)?;
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

/// The most a create's `chooseFirst` competitor scan bills: the credits per competitor of the
/// fee schedule (`FeeDocumentReferenceVersion::choose_first_credits_per_candidate`) times the
/// competitors it may read (`SystemLimits::max_choose_first_candidates`). Check tx reserves it
/// for every such create whose state it does not validate ([`choose_first_reservation`]), so
/// the scan never costs more in a block than the signer was admitted with.
pub(crate) fn choose_first_scan_budget(platform_version: &PlatformVersion) -> Credits {
    platform_version
        .fee_version
        .document_reference
        .choose_first_credits_per_candidate
        .saturating_mul(u64::from(
            platform_version.system_limits.max_choose_first_candidates,
        ))
}

/// What check tx reserves, on top of the estimated fee of `action`, for the `chooseFirst`
/// checks of its creates when it does not validate their state: for every create of a
/// document type declaring `chooseFirst` (registration admits at most one per type), the
/// double SHA-256s of its `findBy` and `validWhen` preimages, priced by the blocks the
/// create's own values make them (a transient param counts, though no stored document keeps
/// it), and [`choose_first_scan_budget`] for the competitor scan. Nothing for anything else,
/// nor for a create whose preimage cannot be assembled, which structure validation refuses
/// before any read. Without it, a signer covering the create's estimated fee but not these
/// would be admitted, then refused unpaid in a block, its nonce left to replay the work.
pub(crate) fn choose_first_reservation(
    action: &StateTransitionAction,
    platform_version: &PlatformVersion,
) -> Vec<ValidationOperation> {
    let StateTransitionAction::BatchAction(batch) = action else {
        return Vec::new();
    };
    let writer_id = batch.owner_id();
    let mut reservation = Vec::new();
    for transition in batch.transitions() {
        let BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(create)) =
            transition
        else {
            continue;
        };
        let base = create.base();
        let Ok(document_type) = base
            .data_contract_fetch_info_ref()
            .contract
            .document_type_for_name(base.document_type_name())
        else {
            continue;
        };
        let Some(lookup) = choose_first_lookup(document_type) else {
            continue;
        };
        // Neither function reads `"."` beside `chooseFirst`, which registration refuses
        for key in lookup.hash_key().map(|(_, key)| key).into_iter().chain(
            lookup
                .choose_first
                .as_ref()
                .map(|choose_first| &choose_first.valid_when),
        ) {
            if let Ok(preimage) = key.preimage(document_type, None, writer_id, create.data()) {
                reservation.push(ValidationOperation::DoubleSha256(
                    key.function.block_count(preimage.len()),
                ));
            }
        }
        reservation.push(ValidationOperation::PrecalculatedOperation(
            FeeResult::new_from_processing_fee(choose_first_scan_budget(platform_version)),
        ));
    }
    reservation
}

/// The lookup through which `document_type` chooses the first of the commitments to a
/// create's values (`chooseFirst`), if it declares one.
fn choose_first_lookup(document_type: DocumentTypeRef<'_>) -> Option<&DocumentReferenceLookup> {
    document_type
        .reference_declarations()
        .filter_map(|(_, reference)| reference.target())
        .flat_map(|target| target.leaves())
        .filter_map(|leaf| leaf.as_any_document_reference())
        .find_map(|declaration| {
            declaration
                .lookup
                .filter(|lookup| lookup.choose_first.is_some())
        })
}

/// What a `chooseFirst` competitor scan has billed, against its budget: each operation is
/// added as it is until the next would pass the budget, which then bills what is left of it,
/// and the rest of the scan bills nothing.
struct ChooseFirstScanBill {
    remaining: Credits,
}

impl ChooseFirstScanBill {
    fn new(platform_version: &PlatformVersion) -> Self {
        Self {
            remaining: choose_first_scan_budget(platform_version),
        }
    }

    fn add(
        &mut self,
        operation: ValidationOperation,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut cost = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(
            slice::from_ref(&operation),
            &mut cost,
            platform_version,
        )?;
        let credits = cost.processing_fee;
        if credits <= self.remaining {
            self.remaining -= credits;
            execution_context.add_operation(operation);
        } else if self.remaining > 0 {
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(
                FeeResult::new_from_processing_fee(self.remaining),
            ));
            self.remaining = 0;
        }
        Ok(())
    }
}

/// The keys `document` is held under by each property of `by`, as its index holds them
/// (its id for `$id`), so comparing them byte by byte orders documents as the index does.
/// Registration demands every such property be recorded on every document, so one missing is
/// an internal error.
fn choose_first_order_keys(
    by: &[ChooseFirstOrder],
    document: &Document,
    document_type: DocumentTypeRef<'_>,
    platform_version: &PlatformVersion,
) -> Result<Vec<Vec<u8>>, Error> {
    by.iter()
        .map(|order| {
            document
                .get_raw_for_document_type(&order.property, document_type, None, platform_version)?
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "registration requires every property chooseFirst orders by on every \
                     commitment",
                )))
        })
        .collect()
}

/// How a commitment held under `keys` compares with one held under `other`, in the order of
/// `by`: the first property they differ on decides, as it ascends or descends.
fn compare_choose_first_order(
    by: &[ChooseFirstOrder],
    keys: &[Vec<u8>],
    other: &[Vec<u8>],
) -> Ordering {
    by.iter()
        .zip(keys.iter().zip(other))
        .map(|(order, (key, other))| {
            let ordering = key.cmp(other);
            if order.ascending {
                ordering
            } else {
                ordering.reverse()
            }
        })
        .find(|ordering| *ordering != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

/// `found`'s value of `property`, the first one its index orders, as a query bounds it: a
/// recorded time or height as the unsigned integer it is stored as.
fn found_order_bound(found: &Document, property: &str) -> Result<Value, Error> {
    let value = referenced_document_value(found, property)
        .ok()
        .flatten()
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "registration requires every property chooseFirst orders by on every commitment",
        )))?;
    Ok(match value.into_owned() {
        Value::I128(system) => Value::U64(u64::try_from(system).map_err(|_| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a recorded time or height is never negative",
            ))
        })?),
        value => value,
    })
}

/// Checks a create's `chooseFirst` (see `LookupChooseFirst`): the commitment `found`, which
/// `lookup` found for the reference declared by `reference_target` at `path`, must be the first
/// valid commitment to the values the create reveals.
///
/// * A commitment past its type's `ttl` at the block time is gone, whether or not the cleanup
///   has reached it: `found` past it finds no document (`ReferencedEntityNotFoundError`,
///   40120, naming `referenced_id`), as a competitor past it precedes nobody.
/// * `found` must be a valid commitment of the writer: it holds, at the property `validWhen`
///   names, that function computed from the create's values with the writer as `"$ownerId"`
///   (billed as the double SHA-256 it is; 40120 otherwise), or a copy of a pending commitment
///   landing first would reveal too.
/// * The documents of `referenced_document_type` holding `found`'s value at `among`, the hash
///   `findBy` found it by, are read in one query
///   in the order `chooseFirst` lists (`by`), through its index, at most
///   `SystemLimits::max_choose_first_candidates`, bounded by `found`'s value of the first
///   property that index orders: one after it there precedes it on no ordering. Each one
///   before `found` in that order, compared by the keys the index holds them under (and by id
///   for a last `$id`), and not past its `ttl`, is a competitor, valid when it holds, at the
///   property `validWhen` names, that function computed from the create's values with the
///   competitor's owner as `"$ownerId"` (billed as the double SHA-256 it is): a copy of
///   another identity's hash is owned by an identity the hash does not name. The first valid
///   competitor refuses the create (`ReferencedDocumentPrecededError`, 40149).
/// * The query and the competitors' hashes are billed as they run, up to the scan's budget
///   ([`choose_first_scan_budget`]), which check tx reserves ([`choose_first_reservation`]).
///
/// `findBy` finds `found` through the referenced type's unique index over `$ownerId` and
/// `among`, so each competitor is another identity's: the writer's only document holding the
/// value is `found`. A document equal to `found` on every ordered property is no
/// competitor (with a last `$id` there is none); the unique index the create writes decides
/// between them. When none of the documents read is a valid competitor the create stands, so
/// documents that are none can make a create cost more but never refuse it. A `found` or a
/// candidate missing an ordered property, or a `found` missing `among`, are what registration
/// rules out, so each is an internal error.
///
/// In place in generation 0: only the protocol version 14 parser produces a `chooseFirst`.
#[allow(clippy::too_many_arguments)]
fn validate_choose_first_v0(
    choose_first: &LookupChooseFirst,
    found: &Document,
    referenced_contract: &DataContract,
    referenced_document_type: DocumentTypeRef<'_>,
    declaring_document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    referenced_id: [u8; 32],
    reference_target: &DocumentPropertyReferenceTarget,
    path: &str,
    platform: &PlatformStateRef,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let not_found = || {
        SimpleConsensusValidationResult::new_with_error(
            ReferencedEntityNotFoundError::new(
                Identifier::from(referenced_id),
                reference_target.clone(),
                path.to_string(),
            )
            .into(),
        )
    };
    if document_is_expired(referenced_document_type, found.created_at(), block_info)? {
        return Ok(not_found());
    }
    let among = choose_first.among.as_str();
    let valid_when_property = choose_first.valid_when_property.as_str();
    // The `validWhen` preimage for the writer: another commitment's is the same bytes with its
    // owner in the `"$ownerId"` slots. The function reads no `"."`, which registration refuses
    let mut valid_when_preimage = choose_first
        .valid_when
        .writer_preimage(declaring_document_type, None, owner_id, document_data)
        .map_err(|_| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "structure validation refuses a create whose validWhen preimage it cannot \
                 assemble",
            ))
        })?;
    // `found` must be a valid commitment itself, the writer's: a copy of a pending one landing
    // first would otherwise reveal too
    let (found_digest, found_blocks) = valid_when_preimage.key_value_for(owner_id);
    execution_context.add_operation(ValidationOperation::DoubleSha256(found_blocks));
    if !holds_key_part(
        referenced_document_type,
        found,
        valid_when_property,
        &Value::Bytes32(found_digest),
    ) {
        return Ok(not_found());
    }
    // The value the commitments to the same values hold, which `findBy` found `found` by
    let group = referenced_document_value(found, among)
        .ok()
        .flatten()
        .map(|value| value.into_owned())
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "findBy found the commitment by the value it holds at among",
        )))?;
    // Where `found` stands in the order, by the keys its index holds it under
    let found_keys = choose_first_order_keys(
        &choose_first.by,
        found,
        referenced_document_type,
        platform_version,
    )?;
    // The commitments read stop at `found`'s value of the first ordered property the index
    // holds: a later one precedes it on no ordering, and with no further property to order
    // by, an equal one does not either
    let index_order: Vec<(String, bool)> = choose_first
        .index_order()
        .iter()
        .map(|order| (order.property.clone(), order.ascending))
        .collect();
    let range = match choose_first.index_order().first() {
        Some(first) => {
            let bound = found_order_bound(found, &first.property)?;
            let only_key = choose_first.by.len() == 1;
            let operator = match (first.ascending, only_key) {
                (true, true) => WhereOperator::LessThan,
                (true, false) => WhereOperator::LessThanOrEquals,
                (false, true) => WhereOperator::GreaterThan,
                (false, false) => WhereOperator::GreaterThanOrEquals,
            };
            Some(WhereClause {
                field: first.property.clone(),
                operator,
                value: bound,
            })
        }
        None => None,
    };
    let (candidates, query_cost) = fetch_documents_holding_in_order(
        platform.drive,
        referenced_contract,
        referenced_document_type,
        among,
        group,
        &index_order,
        range,
        platform_version.system_limits.max_choose_first_candidates,
        &block_info.epoch,
        transaction,
        platform_version,
    )?;
    // The scan bills as it runs, up to its budget, which check tx reserves for a create whose
    // state it does not validate: a block never charges the scan more than the signer was
    // admitted with
    let mut scan_bill = ChooseFirstScanBill::new(platform_version);
    scan_bill.add(
        ValidationOperation::PrecalculatedOperation(query_cost),
        execution_context,
        platform_version,
    )?;
    for candidate in &candidates {
        if candidate.id() == found.id() {
            continue;
        }
        let candidate_keys = choose_first_order_keys(
            &choose_first.by,
            candidate,
            referenced_document_type,
            platform_version,
        )?;
        // Only a commitment strictly before `found` in the order competes; one equal on every
        // ordered property precedes it no more than it precedes that one
        if compare_choose_first_order(&choose_first.by, &candidate_keys, &found_keys)
            != Ordering::Less
        {
            continue;
        }
        if document_is_expired(referenced_document_type, candidate.created_at(), block_info)? {
            continue;
        }
        let (digest, blocks) = valid_when_preimage.key_value_for(candidate.owner_id());
        scan_bill.add(
            ValidationOperation::DoubleSha256(blocks),
            execution_context,
            platform_version,
        )?;
        if holds_key_part(
            referenced_document_type,
            candidate,
            valid_when_property,
            &Value::Bytes32(digest),
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ReferencedDocumentPrecededError::new(
                    found.id(),
                    candidate.id(),
                    among.to_string(),
                    path.to_string(),
                )
                .into(),
            ));
        }
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// The refusal of a write whose referenced document does not meet the `where`
/// entry keyed by `referenced_property` that `condition` declares: a
/// `ReferencedDocumentPropertyMismatchError` (40127), whose referring side is
/// the condition as written (`{ "greaterThan": "$createdAt" }`).
fn condition_mismatch(
    path: &str,
    condition: &WhereCondition,
    referenced_property: &str,
) -> SimpleConsensusValidationResult {
    SimpleConsensusValidationResult::new_with_error(
        ReferencedDocumentPropertyMismatchError::new(
            path.to_string(),
            condition.to_string(),
            referenced_property.to_string(),
        )
        .into(),
    )
}

/// Bills `elements`, the list elements a `where` entry comparing typed arrays
/// reads (`where_lists_elements_read`, `WhereCondition::list_elements_read`),
/// before it compares them: `where_list_credits_per_element` each, of the
/// fee schedule's document reference group. Nothing for an entry comparing
/// single values, which reads none. Every list holds at most
/// `max_typed_array_items` elements, so the product never nears overflow; an
/// overflow is still an error rather than a wrapped bill.
fn bill_where_list_elements(
    elements: usize,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    if elements == 0 {
        return Ok(());
    }
    let credits = u64::try_from(elements)
        .ok()
        .and_then(|elements| {
            elements.checked_mul(
                platform_version
                    .fee_version
                    .document_reference
                    .where_list_credits_per_element,
            )
        })
        .ok_or(Error::Execution(ExecutionError::Overflow(
            "where list comparison fee overflow",
        )))?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(
        FeeResult::new_from_processing_fee(credits),
    ));
    Ok(())
}

/// The value of the bound `condition` compares the referenced side with, read
/// from the referring document: a property of `document_data` (`None` when it
/// leaves it out), the writer `owner_id`, or a time or height of
/// `referring_system` as an integer. `None` for a literal or a list of them,
/// which the condition carries. An error when a path on the way to the property
/// is no object, which the caller refuses as a mismatch, as an equality's
/// unreadable side is, never as absence: a `containsAll` would read an absent
/// list as empty and hold.
fn where_bound_value<'a>(
    condition: &WhereCondition,
    document_data: &'a BTreeMap<String, Value>,
    owner_id: Identifier,
    referring_system: &DocumentSystemValues,
) -> Result<Option<Cow<'a, Value>>, ValueError> {
    let Some(bound_read) = condition.bound_read() else {
        return Ok(None);
    };
    Ok(match bound_read {
        WhereBoundRead::Property(path) => {
            document_data.get_optional_at_path(path)?.map(Cow::Borrowed)
        }
        WhereBoundRead::Writer => Some(Cow::Owned(Value::Identifier(owner_id.to_buffer()))),
        WhereBoundRead::System(property) => referring_system
            .value(property)
            .map(|value| Cow::Owned(Value::I128(value))),
    })
}

/// What a removal record holds of the document a moderator removed, read field
/// by field: its `$id`, its `$ownerId` at the removal, which no transfer can
/// move since (a removed document can not be transferred, and a restore brings
/// it back as it was), and the values of the fields its type keeps under
/// `deleteKeepsFields` (a kept path, or one inside a kept object), decoded under
/// `referenced_document_type` on the first read of one.
struct RemovalRecord<'a> {
    removal: &'a ContractDocumentRemoval,
    document_id: Identifier,
    referenced_document_type: DocumentTypeRef<'a>,
    kept_values: Option<BTreeMap<String, Value>>,
}

impl RemovalRecord<'_> {
    /// The record's value of `field`, `Some(None)` when it keeps the field and
    /// the document held none there, `None` when it keeps no such field. A
    /// record whose values do not read under the type is a corrupted state, an
    /// error rather than a refusal.
    fn value(&mut self, field: &str) -> Result<Option<Option<Value>>, Error> {
        Ok(match field {
            OWNER_ID => Some(Some(Value::Identifier(
                self.removal.document_owner_id.to_buffer(),
            ))),
            ID => Some(Some(Value::Identifier(self.document_id.to_buffer()))),
            field => {
                let kept_paths = self
                    .referenced_document_type
                    .moderator_deletion_kept_fields();
                if !is_path_listed(kept_paths, field) {
                    return Ok(None);
                }
                if self.kept_values.is_none() {
                    self.kept_values =
                        Some(self.removal.kept_values(self.referenced_document_type)?);
                }
                self.kept_values
                    .as_ref()
                    .map(|kept_values| kept_value_at(kept_paths, kept_values, field).flatten())
            }
        })
    }
}

/// A `moderatedDocument` reference a replace keeps, whose document a moderator
/// removed: it holds through `removal`, the document's removal record. The
/// record keeps the document's id and its owner at the removal, which no
/// transfer can move since (a removed document can not be transferred, and a
/// restore brings it back as it was), and the values of the paths its type
/// lists under `moderatorAbilities.deleteKeepsFields`, as the document held
/// them and read under `referenced_document_type` as the document would be, a
/// path it held no value at left out. So a `where` pair
/// the replace asks about again, its referring property changed or the pair a
/// writer gate (`$ownerId` on the referring side), is checked against the
/// record when it compares the referenced `$ownerId`, `$id` or a property the
/// type keeps (a kept path, or a path inside a kept object), with the absence
/// rule of a document in state: both sides absent agree, one absent is a
/// mismatch. It refuses the replace with [`ReferencedDocumentRemovedError`]
/// when it compares anything else: the replace has to repoint the reference at
/// a document in state, or leave the pair's referring property unchanged until
/// the document is restored. The record's kept values are decoded only when
/// a pair asks about one; a record whose values do not read under the type is
/// a corrupted state, an error rather than a refusal. A writer gate never meets that refusal: the parser admits one
/// on a `moderatedDocument` reference against the referenced `$ownerId` or
/// `$id` alone, since a writer gate is asked about on every replace. A pair the replace does not ask about held when it was last
/// checked, against a document whose values the removal froze, and is left as
/// it is, unless a rule reads the document through the reference: then every
/// entry is checked, as the replace checks every entry against a document in
/// state, and registration keeps each to what every record holds. No read: the
/// record was read and billed by the caller. The list elements an entry
/// comparing typed arrays reads are billed before it compares them
/// ([`bill_where_list_elements`]), as against a document in state.
///
/// In place in generation 0: reached from protocol version 14 only, the only
/// version whose parser produces a `moderatedDocument` reference.
#[allow(clippy::too_many_arguments)]
fn validate_pairs_against_removal(
    property_agreement: &BTreeMap<String, String>,
    where_conditions: &BTreeMap<String, WhereCondition>,
    removal: &ContractDocumentRemoval,
    referenced_document_type: DocumentTypeRef,
    document_id: Identifier,
    changed_fields: &BTreeSet<String>,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    referring_system: &DocumentSystemValues,
    path: &str,
    rule_reads: Option<RuleReads<'_>>,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut record = RemovalRecord {
        removal,
        document_id,
        referenced_document_type,
        kept_values: None,
    };
    let removed = |referenced_property: &str| {
        SimpleConsensusValidationResult::new_with_error(
            ReferencedDocumentRemovedError::new(
                document_id,
                path.to_string(),
                referenced_property.to_string(),
            )
            .into(),
        )
    };
    // A rule reading the document has the reference checked on every replace,
    // its `where` whole, as against a document in state; registration keeps
    // every entry of such a reference to what every record holds
    let checks_every_entry = rule_reads.is_some();
    for (referring_property, referenced_property) in property_agreement {
        let rechecked = checks_every_entry
            || is_referring_system_agreement_property(referring_property)
            || is_changed_field(changed_fields, referring_property);
        if !rechecked {
            continue;
        }
        let Some(referenced_value) = record.value(referenced_property)? else {
            return Ok(removed(referenced_property));
        };
        // As against a document in state: the writer for `$ownerId`, a
        // property of the document otherwise, one unreadable a mismatch
        let referring_value = if referring_property == OWNER_ID {
            Some(Cow::Owned(Value::Identifier(owner_id.to_buffer())))
        } else {
            match document_data.get_optional_at_path(referring_property) {
                Ok(referring_value) => referring_value.map(Cow::Borrowed),
                Err(_) => None,
            }
        };
        // Two typed arrays are equal as lists, a list the record holds none of
        // the empty one, as against a document in state
        let lists = typed_array_property(
            referenced_document_type.flattened_properties(),
            referenced_property,
        )
        .is_some();
        let holds = if lists {
            bill_where_list_elements(
                where_lists_elements_read(referring_value.as_deref(), referenced_value.as_ref()),
                execution_context,
                platform_version,
            )?;
            where_lists_equal(referring_value.as_deref(), referenced_value.as_ref())
        } else {
            match (referring_value, referenced_value) {
                (Some(referring_value), Some(referenced_value)) => {
                    referring_value.same_scalar_data(&referenced_value)
                }
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
        };
        if !holds {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ReferencedDocumentPropertyMismatchError::new(
                    path.to_string(),
                    referring_property.clone(),
                    referenced_property.clone(),
                )
                .into(),
            ));
        }
    }
    // The other `where` entries the replace asks about again, as the pairs
    // above: one bounded by a changed property, or by a value that changes
    // without a write (the writer, the last update or transfer), or every one
    // when a rule reads the document. Registration keeps an entry asked about
    // on every replace to what every record holds
    for (referenced_property, condition) in where_conditions {
        let rechecked = checks_every_entry
            || condition.is_rechecked_on_every_replace()
            || condition
                .referring_property()
                .is_some_and(|property| is_changed_field(changed_fields, property));
        if !rechecked {
            continue;
        }
        let Some(referenced_value) = record.value(referenced_property)? else {
            return Ok(removed(referenced_property));
        };
        let kind = WhereLiteralKind::of_referenced(referenced_document_type, referenced_property);
        let Ok(bound) = where_bound_value(condition, document_data, owner_id, referring_system)
        else {
            return Ok(condition_mismatch(path, condition, referenced_property));
        };
        bill_where_list_elements(
            condition.list_elements_read(referenced_value.as_ref(), bound.as_deref()),
            execution_context,
            platform_version,
        )?;
        if !condition.holds(referenced_value.as_ref(), kind, bound.as_deref()) {
            return Ok(condition_mismatch(path, condition, referenced_property));
        }
    }
    // What the type's rules read of the document, from its record: its id, its
    // owner at the removal and the fields its type keeps, the only ones
    // registration lets a rule read through such a reference
    if let Some(rule_reads) = rule_reads {
        let mut values = Value::Map(Vec::new());
        for field in rule_reads.fields.iter().copied() {
            let Some(value) = record.value(field)? else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "a rule reads through a moderatedDocument reference only what every \
                     removal record keeps, which registration enforces",
                )));
            };
            if let Some(value) = value {
                values
                    .set_value_at_full_path(field, value)
                    .map_err(|error| Error::Protocol(error.into()))?;
            }
        }
        rule_reads
            .values
            .insert(rule_reads.reference.to_string(), values);
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
/// The fields a replace changed, when the value `referenced_id` it re-validates
/// at `path` (a reference, or the typed array holding it as an element) is one
/// the stored document already held; `None` on a create and for a value the
/// write sets. A value under a top-level property the replace did not change is
/// the stored one; under a changed one, the replace carries the stored value of
/// that property (`stored_values`), and the value is kept when the stored value
/// at the same path is it, or, for a typed array, holds it. A
/// `moderatedDocument` whose document a moderator removed resolves to its
/// removal record for a kept value alone (see `validate_pairs_against_removal`).
fn kept_value_changed_fields<'a>(
    changed_fields: Option<&'a BTreeSet<String>>,
    stored_values: Option<&BTreeMap<String, Value>>,
    path: &str,
    referenced_id: &[u8; 32],
) -> Option<&'a BTreeSet<String>> {
    let changed = changed_fields?;
    if !is_changed_field(changed, path) {
        return Some(changed);
    }
    let is_referenced_id = |value: &Value| value.to_hash256().is_ok_and(|id| &id == referenced_id);
    let held = match stored_values?.get_optional_at_path(path) {
        Ok(Some(Value::Array(elements))) => elements.iter().any(is_referenced_id),
        Ok(Some(value)) => is_referenced_id(value),
        _ => false,
    };
    held.then_some(changed)
}

/// Whether the write changed the property at `path`, or an object around it.
fn is_changed_field(changed_fields: &BTreeSet<String>, path: &str) -> bool {
    is_path_listed(changed_fields, path)
}
