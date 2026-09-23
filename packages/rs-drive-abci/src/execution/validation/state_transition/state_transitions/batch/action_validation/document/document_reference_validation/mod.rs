pub mod v0;

use std::collections::{BTreeMap, BTreeSet};

use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::v0::DocumentReferenceValidationV0;
use crate::platform_types::platform::PlatformStateRef;

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
    /// would give and named by its list path (`reasons[2]`).
    ///
    /// When `changed_fields` is provided (replace transitions), only references on
    /// those fields are validated. A reference also counts as changed when a
    /// property bound to it changed: a `propertyAgreement` referring property
    /// or an `identityPublicKey` key id property. A writer gate, an agreement
    /// keyed by `$ownerId`, is validated on every replace regardless.
    ///
    /// `owner_id` is the writer, the transition's owner: a `propertyAgreement`
    /// whose referring side is `$ownerId` compares it, and an `identityPublicKey`
    /// reference on a key id property with `identityProperty: $ownerId` names
    /// its key, since it lives on the transition rather than in `document_data`.
    /// `creator_id` is the document's creator for the `$creatorId` form: the
    /// writer on a create, the stored creator on a replace, `None` when the
    /// document type records none (registration then admits no such form).
    #[allow(clippy::too_many_arguments)]
    fn validate_document_references(
        &self,
        document_data: &BTreeMap<String, Value>,
        owner_id: Identifier,
        creator_id: Option<Identifier>,
        changed_fields: Option<&BTreeSet<String>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
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
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
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
                platform,
                block_info,
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
