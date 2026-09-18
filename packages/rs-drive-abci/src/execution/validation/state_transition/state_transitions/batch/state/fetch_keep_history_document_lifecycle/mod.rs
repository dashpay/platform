mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::document::lifecycle::DocumentLifecycleState;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use self::v0::fetch_keep_history_document_lifecycle_v0;

/// Classifies a keep-history document and bills the reads it performs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fetch_keep_history_document_lifecycle(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    id: Identifier,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<DocumentLifecycleState, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .fetch_keep_history_document_lifecycle
    {
        Some(0) => fetch_keep_history_document_lifecycle_v0(
            drive,
            contract,
            document_type,
            id,
            epoch,
            execution_context,
            transaction,
            platform_version,
        ),
        Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "fetch_keep_history_document_lifecycle".to_string(),
            known_versions: vec![0],
            received: version,
        })),
        None => Err(Error::Execution(ExecutionError::VersionNotActive {
            method: "fetch_keep_history_document_lifecycle".to_string(),
            known_versions: vec![0],
        })),
    }
}
