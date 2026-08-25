pub mod v0;

use std::collections::{BTreeMap, BTreeSet};

use dpp::block::block_info::BlockInfo;
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
    /// Validates the document's `refersTo` references against platform state.
    ///
    /// When `changed_fields` is provided (replace transitions), only references on
    /// those fields are validated.
    #[allow(clippy::too_many_arguments)]
    fn validate_document_references(
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

impl DocumentReferenceValidation for DocumentBaseTransitionAction {
    fn validate_document_references(
        &self,
        document_data: &BTreeMap<String, Value>,
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
