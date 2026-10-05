pub mod v0;

use std::collections::{BTreeMap, BTreeSet};

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentPropertyReferenceTarget;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::ConsumedDocument;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::v0::DocumentReferenceValidationV0;
use crate::platform_types::platform::PlatformStateRef;

/// A document a create consumes: the commitment a `refersTo` lookup with a computed key
/// declaring `consume` found, with what a refusal naming the reference needs should the
/// batch touch the same document elsewhere.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConsumedLookupDocument {
    /// The document the create deletes.
    pub(crate) document: ConsumedDocument,
    /// The value the reference was checked for: the writer, the creator, or the property's
    /// value (one element's for a typed array).
    pub(crate) referenced_id: Identifier,
    /// The leaf declaring the lookup.
    pub(crate) reference_target: DocumentPropertyReferenceTarget,
    /// How the reference errors name the reference.
    pub(crate) path: String,
}

pub(crate) trait DocumentReferenceValidation {
    /// Whether the top-level `property` of this document's type is a
    /// `refersTo: deletableDocument` reference whose target `referenced_id`
    /// is no longer in state. `false` for any other property.
    ///
    /// Read by the immutable-property check on replaces: clearing an
    /// `immutable` deletableDocument reference is the one way left to
    /// rewrite a document whose target was deleted, and it is only allowed
    /// once the target really is gone.
    #[allow(clippy::too_many_arguments)]
    fn deletable_document_reference_target_is_gone(
        &self,
        property: &str,
        referenced_id: Identifier,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;

    /// Validates the document's `refersTo` references against platform state:
    /// an identifier property's value, and each element of a typed array whose
    /// `items` declare one, which is refused with the error a single reference
    /// would give and named by its list path (`reasons[2]`). A value declared
    /// by a reference expression is checked operand by operand in declared
    /// order: an `anyOf` holds when one operand does and is otherwise refused
    /// with the last operand's error, an `allOf` holds when every operand does
    /// and is otherwise refused with the first failing operand's error.
    ///
    /// When `changed_fields` is provided (replace transitions), only references on
    /// those fields are validated. A reference also counts as changed when a
    /// property bound to it changed: a `where` referring value
    /// or an `identityPublicKey` key id property. A writer gate, an agreement
    /// keyed by `$ownerId`, is validated on every replace regardless, and so
    /// is a `contract` reference whose `contractRequirements` carry an
    /// `owner` requirement on a document type whose documents can be
    /// transferred or traded, since the requirement is judged against the
    /// writer.
    /// `stored_values` (replace transitions) holds the stored value of each
    /// changed property: a changed typed array of references re-validates
    /// only the elements the stored list did not hold, unless a bound
    /// property changed, a writer gate or such an owner requirement applies
    /// or its target is deletable.
    ///
    /// `owner_id` is the writer, the transition's owner: a `where` entry whose
    /// referring value is `$ownerId` compares it, an `identityPublicKey`
    /// reference on a key id property with `identityProperty: $ownerId` names
    /// its key, and the document type's `ownerRefersTo` declaration is checked
    /// with it as the value (under the replace rules of its target), since it
    /// lives on the transition rather than in `document_data`.
    /// `creator_id` is the document's creator for the `$creatorId` form and
    /// the value of the document type's `creatorRefersTo`: the writer on a
    /// create, the stored creator on a replace. It is `None` on a replace of a
    /// document that records none: its type records no creator ids
    /// (registration then admits neither), or it was written before its type
    /// did: a `$creatorId` key id property a contract update added may then
    /// not be set, and an update cannot add a `creatorRefersTo`.
    ///
    /// A lookup with a computed key reveals a commitment and is judged on a create only
    /// (`changed_fields` is `None`): the document it finds must meet the reference's
    /// `minimumAgeBlocks`, and when the reference declares `consume` the document is pushed onto
    /// `consumed_documents` for the caller to delete with the create once it accepts it. A
    /// replace leaves it alone, since nothing it reads can have changed.
    ///
    /// When `derived_index_values` is given (a create, protocol version 14), an accepted write
    /// records there the value of each derived index property of the document type, read from
    /// the document its reference points at, which this validation fetched: Drive keys the
    /// new document by them without reading those documents again. A value whose document it
    /// did not fetch is left out, for Drive to read.
    #[allow(clippy::too_many_arguments)]
    fn validate_document_references(
        &self,
        document_data: &BTreeMap<String, Value>,
        owner_id: Identifier,
        creator_id: Option<Identifier>,
        changed_fields: Option<&BTreeSet<String>>,
        stored_values: Option<&BTreeMap<String, Value>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        derived_index_values: Option<&mut BTreeMap<String, Value>>,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentReferenceValidation for DocumentBaseTransitionAction {
    fn deletable_document_reference_target_is_gone(
        &self,
        property: &str,
        referenced_id: Identifier,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_reference_validation
        {
            0 => self.deletable_document_reference_target_is_gone_v0(
                property,
                referenced_id,
                platform,
                block_info,
                transaction,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentBaseTransitionAction::deletable_document_reference_target_is_gone"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_document_references(
        &self,
        document_data: &BTreeMap<String, Value>,
        owner_id: Identifier,
        creator_id: Option<Identifier>,
        changed_fields: Option<&BTreeSet<String>>,
        stored_values: Option<&BTreeMap<String, Value>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        derived_index_values: Option<&mut BTreeMap<String, Value>>,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_reference_validation
        {
            0 => self.validate_document_references_v0(
                document_data,
                owner_id,
                creator_id,
                changed_fields,
                stored_values,
                platform,
                block_info,
                consumed_documents,
                derived_index_values,
                transaction,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentBaseTransitionAction::validate_document_references".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
