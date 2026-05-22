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

// ============================================================================
// fetch_documents_for_transitions_knowing_contract_and_document_type
// ============================================================================

/// Versioned facade for `fetch_documents_for_transitions_knowing_contract_and_document_type`.
///
/// Dispatches on the per-helper version field on
/// `DriveAbciDocumentsStateTransitionValidationVersions`:
/// - v0 (PROTOCOL_VERSION_11 and below) — byte-identical to pre-PR.
///   `epoch` and `execution_context` arguments are unused. No fees billed.
/// - v1 (PROTOCOL_VERSION_12+) — passes `Some(epoch)` to `query_documents`
///   and bills the cost via `execution_context.add_operation`.
#[allow(clippy::too_many_arguments)]
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
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .fetch_documents_for_transitions_knowing_contract_and_document_type
    {
        0 => fetch_documents_for_transitions_knowing_contract_and_document_type_v0(
            drive,
            contract,
            document_type,
            transitions,
            transaction,
            platform_version,
        ),
        1 => fetch_documents_for_transitions_knowing_contract_and_document_type_v1(
            drive,
            contract,
            document_type,
            transitions,
            epoch,
            execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(
            crate::error::execution::ExecutionError::UnknownVersionMismatch {
                method: "fetch_documents_for_transitions_knowing_contract_and_document_type"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            },
        )),
    }
}

/// PROTOCOL_VERSION_11 byte-identical implementation. Passes `epoch=None`
/// to `query_documents`, ignores `execution_context`, never bills.
/// Body matches pre-PR (v3.1-dev).
fn fetch_documents_for_transitions_knowing_contract_and_document_type_v0(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    transitions: &[&DocumentTransition],
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

    // todo: deal with cost of this operation
    let documents_outcome = drive.query_documents(
        drive_query,
        None,
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;

    Ok(ConsensusValidationResult::new_with_data(
        documents_outcome.documents_owned(),
    ))
}

/// PROTOCOL_VERSION_12+ implementation. Passes `Some(epoch)` to
/// `query_documents` so the real grovedb cost is computed; bills it via
/// `execution_context.add_operation`. Documents returned are
/// epoch-independent — same as v0.
#[allow(clippy::too_many_arguments)]
fn fetch_documents_for_transitions_knowing_contract_and_document_type_v1(
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

    // Diff vs `_v0`: epoch is `Some(...)` and the cost is billed via
    // add_operation on the outer execution_context.
    let documents_outcome = drive.query_documents(
        drive_query,
        Some(epoch),
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(FeeResult {
        storage_fee: 0,
        processing_fee: documents_outcome.cost(),
        fee_refunds: Default::default(),
        removed_bytes_from_system: 0,
    }));

    Ok(ConsensusValidationResult::new_with_data(
        documents_outcome.documents_owned(),
    ))
}

// ============================================================================
// fetch_document_with_id
// ============================================================================

/// Versioned facade for `fetch_document_with_id`.
///
/// Dispatches on the per-helper version field on
/// `DriveAbciDocumentsStateTransitionValidationVersions`:
/// - v0 (PROTOCOL_VERSION_11 and below) — byte-identical to pre-PR.
///   Calls `_v0` (returns `(Option<Document>, FeeResult)` with a zero-cost
///   FeeResult), adds the zero-fee FeeResult to `execution_context` via
///   `add_operation` — matching the pre-PR caller's behavior exactly.
/// - v1 (PROTOCOL_VERSION_12+) — calls `_v1` which passes `Some(epoch)`
///   for the real grovedb cost and bills internally.
#[allow(clippy::too_many_arguments)]
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
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .fetch_document_with_id
    {
        0 => {
            // Preserve pre-PR caller semantics: the caller used to call
            // `add_operation(PrecalculatedOperation(fee_result))` with
            // the (always-zero) FeeResult returned by the old fn. We
            // emulate that here so the operations_slice on v0 has the
            // same shape as pre-PR.
            let (document, fee_result) = fetch_document_with_id_v0(
                drive,
                contract,
                document_type,
                id,
                transaction,
                platform_version,
            )?;
            execution_context
                .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
            Ok(document)
        }
        1 => fetch_document_with_id_v1(
            drive,
            contract,
            document_type,
            id,
            epoch,
            execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(
            crate::error::execution::ExecutionError::UnknownVersionMismatch {
                method: "fetch_document_with_id".to_string(),
                known_versions: vec![0, 1],
                received: version,
            },
        )),
    }
}

/// PROTOCOL_VERSION_11 byte-identical implementation. Passes `epoch=None`
/// to `query_documents` (cost hard-coded to 0). Returns `(Option<Document>,
/// FeeResult)` with `processing_fee=0` — matches pre-PR (v3.1-dev) signature
/// and behavior exactly.
fn fetch_document_with_id_v0(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    id: Identifier,
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

    // todo: deal with cost of this operation
    let documents_outcome = drive.query_documents(
        drive_query,
        None,
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
    let mut documents = documents_outcome.documents_owned();

    if documents.is_empty() {
        Ok((None, fee_result))
    } else {
        Ok((Some(documents.remove(0)), fee_result))
    }
}

/// PROTOCOL_VERSION_12+ implementation. Passes `Some(epoch)` to
/// `query_documents` for the real grovedb cost; bills it via
/// `execution_context.add_operation`. Document returned is
/// epoch-independent — same as v0.
#[allow(clippy::too_many_arguments)]
fn fetch_document_with_id_v1(
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

    // Diff vs `_v0`: epoch is `Some(...)` and the cost is billed via
    // add_operation on the outer execution_context.
    let documents_outcome = drive.query_documents(
        drive_query,
        Some(epoch),
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(FeeResult {
        storage_fee: 0,
        processing_fee: documents_outcome.cost(),
        fee_refunds: Default::default(),
        removed_bytes_from_system: 0,
    }));

    let mut documents = documents_outcome.documents_owned();

    if documents.is_empty() {
        Ok(None)
    } else {
        Ok(Some(documents.remove(0)))
    }
}

// ============================================================================
// has_contested_document_with_document_id — unchanged
// ============================================================================

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
