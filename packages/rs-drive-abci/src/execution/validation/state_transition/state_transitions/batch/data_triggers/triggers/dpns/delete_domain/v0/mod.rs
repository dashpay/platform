use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::fee::fee_result::FeeResult;
use dpp::system_data_contracts::dpns_contract::v1::document_types::domain;
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::query::DriveDocumentQuery;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};

/// The DPNS `domain` delete trigger: a contested name can not be deleted.
///
/// DPNS v3 lets a name's owner give the name up two ways. A delete removes the
/// `domain` document, and anybody may register the name again. A `tombstone` create
/// consumes the `domain` document and keeps the name from ever being registered
/// while the tombstone exists.
///
/// A contested name, one whose normalized label matches the pattern of the
/// `parentNameAndLabel` index's contest, was awarded by a masternode vote, and the
/// record of that vote stays for good: a name the vote awarded is never contested
/// again. So its owner can not free it with a delete, which this trigger refuses:
/// it can only be tombstoned. A consume is no delete transition and runs no
/// trigger.
///
/// The name is read from the stored document (a billed Drive query by id), as a
/// delete transition carries only the document's id.
///
/// # Returns
///
/// A `DataTriggerExecutionResult` holding a `DataTriggerConditionError` when the
/// stored name is contested or is not found, or none. A dry run (`check_tx`)
/// runs the query to bill it and judges nothing.
#[inline(always)]
pub(super) fn delete_domain_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
    context: &mut DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let is_dry_run = context.state_transition_execution_context.in_dry_run();
    let DocumentTransitionAction::DeleteAction(_) = document_transition else {
        return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
            format!(
                "the Document Transition {} isn't 'DELETE",
                document_transition.base().id()
            ),
        )));
    };

    let document_type = data_contract.document_type_for_name(domain::NAME)?;

    let domain_outcome = context.platform.drive.query_documents(
        DriveDocumentQuery::new_primary_key_single_item_query(
            data_contract,
            document_type,
            document_transition.base().id(),
        ),
        Some(&context.block_info.epoch),
        is_dry_run,
        context.transaction,
        Some(platform_version.protocol_version),
    )?;
    context.state_transition_execution_context.add_operation(
        ValidationOperation::PrecalculatedOperation(FeeResult {
            processing_fee: domain_outcome.cost(),
            ..Default::default()
        }),
    );

    let mut result = DataTriggerExecutionResult::default();
    if is_dry_run {
        return Ok(result);
    }

    let documents = domain_outcome.documents_owned();
    let Some(stored_domain) = documents.first() else {
        result.add_error(DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "Domain is not present".to_string(),
        ));
        return Ok(result);
    };

    if document_type
        .contested_vote_poll_for_document(stored_domain, platform_version)?
        .is_some()
    {
        result.add_error(DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "A contested name can not be deleted, only tombstoned".to_string(),
        ));
    }

    Ok(result)
}
