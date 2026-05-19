use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::document::query::query_contested_documents_storage::QueryContestedDocumentsOutcomeV0Methods;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::drive_contested_document_query::{
    DriveContestedDocumentQuery, PrimaryContestedInternalClauses,
};
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};

/// Fetches the documents and bills the `query_documents` cost directly
/// to the passed-in `execution_context` (gated by `transform_into_action`).
///
/// PROTOCOL_VERSION_11 consensus-safety:
/// - On `transform_into_action: 0` the function passes `epoch=None` to
///   `query_documents` (cost hard-coded to 0) and skips `add_operation`.
///   Identical to pre-PR — pre-PR also passed `None` and never billed.
/// - On `transform_into_action: 1` (PROTOCOL_VERSION_12+) it passes
///   `Some(epoch)` to get the real grovedb cost and adds it via
///   `add_operation`.
///
/// Either way the DOCUMENTS returned are unchanged — `query_documents`
/// is epoch-independent for the documents/skipped fields, only the
/// `cost` field varies.
pub(crate) fn fetch_documents_for_transitions_knowing_contract_and_document_type(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    transitions: &[&DocumentTransition],
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    if transitions.is_empty() {
        return Ok(ConsensusValidationResult::new_with_data(vec![]));
    }

    let ids: Vec<Value> = transitions
        .iter()
        .map(|dt| Value::Identifier(dt.get_id().to_buffer()))
        .collect();

    let drive_query = DriveDocumentQuery {
        contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(ids),
            }),
            primary_key_equal_clause: None,
            in_clause: None,
            range_clause: None,
            equal_clauses: Default::default(),
        },
        offset: None,
        limit: Some(transitions.len() as u16),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    let epoch_arg = match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .transform_into_action
    {
        0 => None,
        1 => Some(epoch),
        version => {
            return Err(Error::Execution(
                crate::error::execution::ExecutionError::UnknownVersionMismatch {
                    method:
                        "fetch_documents_for_transitions_knowing_contract_and_document_type: \
                         transform_into_action gate"
                            .to_string(),
                    known_versions: vec![0, 1],
                    received: version,
                },
            ));
        }
    };

    let documents_outcome = drive.query_documents(
        drive_query,
        epoch_arg,
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;

    // Bill only on v1. On v0 the cost is 0 anyway (epoch was None), but
    // we still skip the add_operation call so the v0 path is also a
    // syntactic no-op for the execution_context — matching pre-PR.
    if epoch_arg.is_some() {
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(FeeResult {
            storage_fee: 0,
            processing_fee: documents_outcome.cost(),
            fee_refunds: Default::default(),
            removed_bytes_from_system: 0,
        }));
    }

    Ok(ConsensusValidationResult::new_with_data(
        documents_outcome.documents_owned(),
    ))
}

/// Returns the document (if any) and bills the `query_documents` cost
/// directly to the passed-in `execution_context` (gated by
/// `transform_into_action`).
///
/// PROTOCOL_VERSION_11 consensus-safety:
/// - On `transform_into_action: 0` the function passes `epoch=None`
///   (cost hard-coded to 0) and skips `add_operation`. Pre-PR called
///   `query_documents` with `None` too and the caller did
///   `add_operation` with a zero `FeeResult` (no-op-fee). Net effect:
///   identical to pre-PR.
/// - On `transform_into_action: 1` (PROTOCOL_VERSION_12+) it passes
///   `Some(epoch)` for the real grovedb cost and adds it via
///   `add_operation`.
pub(crate) fn fetch_document_with_id(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    id: Identifier,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<Document>, Error> {
    let drive_query = DriveDocumentQuery {
        contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(id.to_buffer()),
            }),
            in_clause: None,
            range_clause: None,
            equal_clauses: Default::default(),
        },
        offset: None,
        limit: Some(1),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    let epoch_arg = match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .transform_into_action
    {
        0 => None,
        1 => Some(epoch),
        version => {
            return Err(Error::Execution(
                crate::error::execution::ExecutionError::UnknownVersionMismatch {
                    method: "fetch_document_with_id: transform_into_action gate".to_string(),
                    known_versions: vec![0, 1],
                    received: version,
                },
            ));
        }
    };

    let documents_outcome = drive.query_documents(
        drive_query,
        epoch_arg,
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;

    // Bill only on v1. Same reasoning as
    // `fetch_documents_for_transitions_knowing_contract_and_document_type`.
    if epoch_arg.is_some() {
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(FeeResult {
            storage_fee: 0,
            processing_fee: documents_outcome.cost(),
            fee_refunds: Default::default(),
            removed_bytes_from_system: 0,
        }));
    }

    let mut documents = documents_outcome.documents_owned();

    if documents.is_empty() {
        Ok(None)
    } else {
        Ok(Some(documents.remove(0)))
    }
}

pub(crate) fn has_contested_document_with_document_id<'a>(
    drive: &Drive,
    contract: &'a DataContract,
    document_type: DocumentTypeRef<'a>,
    document_id: Identifier,
    epoch: Option<&Epoch>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<(bool, FeeResult), Error> {
    let drive_query = DriveContestedDocumentQuery {
        contract,
        document_type,
        internal_clauses: PrimaryContestedInternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: Some(document_id),
        },
    };

    let documents_outcome = drive.query_contested_documents(
        drive_query,
        epoch,
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;

    let fee = documents_outcome.cost();
    let fee_result = FeeResult {
        storage_fee: 0,
        processing_fee: fee,
        fee_refunds: Default::default(),
        removed_bytes_from_system: 0,
    };
    let documents = documents_outcome.documents_owned();

    if documents.is_empty() {
        Ok((false, fee_result))
    } else {
        Ok((true, fee_result))
    }
}
