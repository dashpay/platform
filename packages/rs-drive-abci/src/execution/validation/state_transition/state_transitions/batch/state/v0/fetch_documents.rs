use crate::error::Error;
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

/// Returns the fetched documents plus the `FeeResult` for the underlying
/// `query_documents` operation. The caller decides whether to bill the
/// `FeeResult` to the `StateTransitionExecutionContext` — gated by the
/// `transform_into_action` field on
/// `DriveAbciDocumentsStateTransitionValidationVersions` (`0` discards
/// the cost for PROTOCOL_VERSION_11 chain replay, `1` bills it).
///
/// PROTOCOL_VERSION_11 consensus-safety: the function signature
/// changed from pre-PR (added `epoch: &Epoch`, return type now a
/// tuple) but the DOCUMENTS returned are unchanged — `query_documents`
/// is epoch-independent for the documents/skipped fields, only the
/// `cost` field varies. The cost is discarded on `transform_into_action: 0`
/// at the caller, so net PV11 user-visible behavior matches pre-PR.
///
/// `query_documents` only computes a non-zero cost when an `Epoch` is
/// provided; the legacy `None` epoch resulted in a hard-coded zero cost
/// that was discarded anyway.
pub(crate) fn fetch_documents_for_transitions_knowing_contract_and_document_type(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    transitions: &[&DocumentTransition],
    epoch: &Epoch,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<(ConsensusValidationResult<Vec<Document>>, FeeResult), Error> {
    if transitions.is_empty() {
        return Ok((
            ConsensusValidationResult::new_with_data(vec![]),
            FeeResult::default(),
        ));
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

    let documents_outcome = drive.query_documents(
        drive_query,
        Some(epoch),
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;

    let fee_result = FeeResult {
        storage_fee: 0,
        processing_fee: documents_outcome.cost(),
        fee_refunds: Default::default(),
        removed_bytes_from_system: 0,
    };

    Ok((
        ConsensusValidationResult::new_with_data(documents_outcome.documents_owned()),
        fee_result,
    ))
}

/// Returns the document (if any) plus the `FeeResult` for the underlying
/// `query_documents` operation.
///
/// The cost computation is gated by `transform_into_action` on
/// `DriveAbciDocumentsStateTransitionValidationVersions`:
/// - `0` (PROTOCOL_VERSION_11 and below): pass `epoch=None` to
///   `query_documents`, which hard-codes the cost to 0. The returned
///   `FeeResult` has `processing_fee=0` and callers' `add_operation`
///   becomes a no-op-fee. Byte-identical to pre-PR behavior on v11.
/// - `1` (PROTOCOL_VERSION_12+): pass `Some(epoch)` so the real grovedb
///   cost is computed and returned. Callers bill it via the existing
///   `execution_context.add_operation` call site.
///
/// PROTOCOL_VERSION_11 consensus-safety: signature changed from pre-PR
/// (added `epoch: &Epoch` parameter) but the v0 arm forces epoch=None
/// inside `query_documents`, producing the exact same zero-cost
/// `FeeResult` that pre-PR produced. The DOCUMENT returned is
/// epoch-independent. Callers (`document_create_transition_action`,
/// `document_delete_transition_action`) always called
/// `add_operation(PrecalculatedOperation(fee_result))` pre-PR — that
/// call survives unchanged but receives a zero-cost FeeResult on PV11,
/// same net effect (no fees added).
pub(crate) fn fetch_document_with_id(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    id: Identifier,
    epoch: &Epoch,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<(Option<Document>, FeeResult), Error> {
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

    let fee_result = FeeResult {
        storage_fee: 0,
        processing_fee: documents_outcome.cost(),
        fee_refunds: Default::default(),
        removed_bytes_from_system: 0,
    };
    let mut documents = documents_outcome.documents_owned();

    if documents.is_empty() {
        Ok((None, fee_result))
    } else {
        Ok((Some(documents.remove(0)), fee_result))
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
