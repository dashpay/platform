use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::document::lifecycle::DocumentLifecycleState;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

#[allow(clippy::too_many_arguments)]
pub(super) fn fetch_keep_history_document_lifecycle_v0(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    id: Identifier,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<DocumentLifecycleState, Error> {
    let (state, fee_result) = drive
        .fetch_document_lifecycle(
            contract,
            document_type,
            id,
            Some(epoch),
            transaction,
            platform_version,
        )
        .map_err(Error::Drive)?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
    Ok(state)
}
