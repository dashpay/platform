//! The `refersTo` lookup of document reference validation 1 (protocol version 15).

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::{
    fetch_document_through_lookup, hash_lookup_key, holds_key_part, BilledLookupKey,
};
use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{DocumentReferenceLookup, DocumentTypeRef};
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsWithFlagsOutcomeV0Methods;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use drive::util::storage_flags::StorageFlags;
use std::collections::BTreeMap;

/// What `fetch_document_through_lookup` (batch state 0) finds. For a lookup declaring `consume`,
/// it also returns the storage flags of the document's stored element. That is the same path
/// query, read through `query_documents_with_flags` instead of `query_documents` and billed the
/// same way, and the create that consumes the document deletes it from this read without
/// reading it again.
///
/// The two GroveDB calls behind those queries differ in how they treat a missing intermediate
/// tree, so a consuming lookup that finds nothing may cost differently from generation 0's.
/// A lookup that does not consume needs no flags, so it calls generation 0's read and keeps
/// its cost.
#[allow(clippy::too_many_arguments)]
pub(super) fn fetch_document_with_flags_through_lookup(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    declaring_document_type: DocumentTypeRef,
    lookup: &DocumentReferenceLookup,
    reference_value: &Value,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    billed_key: Option<BilledLookupKey>,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<(Document, Option<StorageFlags>)>, Error> {
    if !lookup.consume {
        return Ok(fetch_document_through_lookup(
            drive,
            contract,
            document_type,
            declaring_document_type,
            lookup,
            reference_value,
            document_data,
            owner_id,
            billed_key,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?
        .map(|document| (document, None)));
    }
    // Registration resolved the index its `findBy` names, and a type's indexes
    // never change, so this only guards a declaration no registration admits:
    // one naming no unique index finds no document
    if lookup.resolve_index(document_type).is_err() {
        return Ok(None);
    }
    // A function's `"."` param reads an identifier (an element, the writer or
    // the creator); a property carrying the reference is named by its path.
    // The writer is passed in place, inert before protocol version 14: only
    // meta-schema v3 admits a computed key, and the `"$ownerId"` param that
    // reads the writer
    let reference_id = match reference_value {
        Value::Identifier(id) => Some(Identifier::from(*id)),
        _ => None,
    };
    let billed_key = billed_key.or_else(|| {
        lookup.hash_key().and_then(|(_, key)| {
            hash_lookup_key(
                key,
                declaring_document_type,
                reference_id,
                owner_id,
                document_data,
                execution_context,
            )
        })
    });
    let computed_key = billed_key.map(|key| Value::Bytes32(key.digest()));
    let Some(key_values) = lookup.key_values(
        reference_value,
        document_data,
        owner_id,
        computed_key.as_ref(),
    ) else {
        return Ok(None);
    };
    // A key part no index entry can hold finds no document. Serializing it is
    // what the query does first, and its failure is the value's, not the state's
    if key_values.iter().any(|(index_property, value)| {
        document_type
            .serialize_value_for_key(index_property, value, platform_version)
            .is_err()
    }) {
        return Ok(None);
    }
    let equal_clauses = key_values
        .iter()
        .map(|(field, value)| {
            (
                field.clone(),
                WhereClause {
                    field: field.clone(),
                    operator: WhereOperator::Equal,
                    value: value.clone(),
                },
            )
        })
        .collect();

    let drive_query = DriveDocumentQuery {
        contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: None,
            in_clauses: Vec::new(),
            range_clause: None,
            equal_clauses,
        },
        offset: None,
        // The index is unique and every one of its properties is fixed, so at
        // most one document matches
        limit: Some(1),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
        sub_queries: vec![],
    };

    let documents_outcome = drive.query_documents_with_flags(
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
        lifetime_storage_fees: Default::default(),
    }));

    Ok(documents_outcome
        .documents_owned()
        .into_iter()
        .next()
        .filter(|(document, _)| {
            key_values.iter().all(|(index_property, value)| {
                holds_key_part(document_type, document, index_property, value)
            })
        }))
}
