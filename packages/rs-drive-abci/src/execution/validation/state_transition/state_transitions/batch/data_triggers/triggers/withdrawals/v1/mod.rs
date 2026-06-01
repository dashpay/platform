//! PROTOCOL_VERSION_12+ version of the withdrawals delete trigger.
//!
//! Mirrors `delete_withdrawal_data_trigger_v0` but passes `Some(epoch)`
//! to `query_documents` so the real grovedb cost is computed, and
//! bills it via
//! `context.state_transition_execution_context.add_operation(...)` so
//! the user pays for the read.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use dpp::system_data_contracts::withdrawals_contract;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::version::PlatformVersion;
use dpp::{document, ProtocolError};
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use std::collections::BTreeMap;

#[inline(always)]
pub(super) fn delete_withdrawal_data_trigger_v1(
    document_transition: &DocumentTransitionAction,
    context: &mut DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let mut result = DataTriggerExecutionResult::default();

    let DocumentTransitionAction::DeleteAction(dt_delete) = document_transition else {
        return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
            format!(
                "the Document Transition {} isn't 'DELETE",
                document_transition.base().id()
            ),
        )));
    };

    let document_type = data_contract.document_type_for_name(withdrawal::NAME)?;

    let drive_query = DriveDocumentQuery {
        contract: data_contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: Some(WhereClause {
                field: document::property_names::ID.to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(dt_delete.base().id().to_buffer()),
            }),
            in_clause: None,
            range_clause: None,
            equal_clauses: BTreeMap::default(),
        },
        offset: None,
        limit: Some(100),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    // Diff vs `_v0` (withdrawal-document lookup):
    //
    //   _v0: `.query_documents(drive_query, None, false, ...)` —
    //        `epoch=None` short-circuits cost to 0. The outcome's
    //        documents drive the status-check; the cost is discarded.
    //
    //   _v1: `.query_documents(drive_query, Some(epoch), ...)` and
    //        immediately bill via `add_operation`.
    //
    // Why the change: every withdrawal-document delete ran a grovedb
    // lookup on PV11 that the user didn't pay for. The fetched
    // document is epoch-independent so changing `None` to
    // `Some(epoch)` only affects the cost field — not the validation
    // outcome.
    let withdrawals_outcome = context.platform.drive.query_documents(
        drive_query,
        Some(&context.block_info.epoch),
        false,
        context.transaction,
        Some(platform_version.protocol_version),
    )?;
    let query_fee = FeeResult {
        processing_fee: withdrawals_outcome.cost(),
        ..Default::default()
    };
    context
        .state_transition_execution_context
        .add_operation(ValidationOperation::PrecalculatedOperation(query_fee));
    let withdrawals = withdrawals_outcome.documents_owned();

    let Some(withdrawal) = withdrawals.first() else {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            dt_delete.base().id(),
            "Withdrawal document was not found".to_string(),
        );

        result.add_error(err);

        return Ok(result);
    };

    let status: u8 = withdrawal
        .properties()
        .get_integer("status")
        .map_err(ProtocolError::ValueError)?;

    if status != withdrawals_contract::WithdrawalStatus::COMPLETE as u8 {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            dt_delete.base().id(),
            "withdrawal deletion is allowed only for COMPLETE statuses".to_string(),
        );

        result.add_error(err);

        return Ok(result);
    }

    Ok(result)
}
