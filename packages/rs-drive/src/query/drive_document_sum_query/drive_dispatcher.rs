//! Sum-query dispatcher entry point.
//!
//! Parallels [`crate::query::drive_document_count_query::drive_dispatcher`]
//! for the sum surface. Routes a parsed [`DocumentSumRequest`] to one of
//! the per-mode executors based on the (where × mode × prove) triple,
//! exactly the way count's dispatcher does.
//!
//! `where_clauses_from_value` / `order_clauses_from_value` are wire-shape
//! adapters that the bench and the gRPC handler both use to convert the
//! CBOR-decoded `Value::Array` input into structured `Vec<WhereClause>` /
//! `Vec<OrderClause>`. Identical input contract to count.

use crate::drive::Drive;
use crate::error::Error;
use crate::query::drive_document_sum_query::{
    DocumentSumMode, DocumentSumRequest, DocumentSumResponse, RangeSumOptions, SumMode,
};
use crate::query::{OrderClause, WhereClause};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

#[cfg(feature = "server")]
impl Drive {
    /// Server-side entry point for the sum surface. Routes a
    /// [`DocumentSumRequest`] to the appropriate executor based on the
    /// where-shape, requested mode, and `prove` flag.
    ///
    /// Mirrors [`Drive::execute_document_count_request`].
    pub fn execute_document_sum_request(
        &self,
        request: DocumentSumRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentSumResponse, Error> {
        let resolved_mode = detect_sum_mode(&request)?;

        let contract_id = request.contract.id().to_buffer();
        let document_type_name = request.document_type.name().to_string();
        let where_clauses = request.where_clauses;
        let sum_property = request.sum_property;
        // Default direction is ascending; the first order clause's
        // direction (if any) wins. Mirrors count's analog.
        let order_by_ascending = request
            .order_clauses
            .first()
            .map(|c| c.ascending)
            .unwrap_or(true);

        match resolved_mode {
            DocumentSumMode::Total => {
                let entries = self.execute_document_sum_total_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    transaction,
                    platform_version,
                )?;
                let total = entries.first().and_then(|e| e.sum).unwrap_or(0);
                Ok(DocumentSumResponse::Aggregate(total))
            }
            DocumentSumMode::PerInValue => {
                let options = RangeSumOptions {
                    return_distinct_sums_in_range: false,
                    carrier_outer_limit: None,
                    left_to_right: order_by_ascending,
                };
                Ok(DocumentSumResponse::Entries(
                    self.execute_document_sum_per_in_value_no_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        sum_property,
                        options,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentSumMode::RangeNoProof => {
                let return_distinct = matches!(
                    request.mode,
                    SumMode::GroupByRange | SumMode::GroupByCompound
                );
                let options = RangeSumOptions {
                    return_distinct_sums_in_range: return_distinct,
                    carrier_outer_limit: None,
                    left_to_right: order_by_ascending,
                };
                let entries = self.execute_document_sum_range_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    options,
                    transaction,
                    platform_version,
                )?;
                if matches!(request.mode, SumMode::Aggregate) {
                    let total = entries.first().and_then(|e| e.sum).unwrap_or(0);
                    Ok(DocumentSumResponse::Aggregate(total))
                } else {
                    Ok(DocumentSumResponse::Entries(entries))
                }
            }
            DocumentSumMode::RangeProof => Ok(DocumentSumResponse::Proof(
                self.execute_document_sum_range_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentSumMode::RangeDistinctProof => {
                let effective_limit = request
                    .limit
                    .unwrap_or(crate::config::DEFAULT_QUERY_LIMIT as u32);
                let limit_u16 = effective_limit as u16;
                Ok(DocumentSumResponse::Proof(
                    self.execute_document_sum_range_distinct_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        sum_property,
                        limit_u16,
                        order_by_ascending,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentSumMode::PointLookupProof => Ok(DocumentSumResponse::Proof(
                self.execute_document_sum_point_lookup_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentSumMode::RangeAggregateCarrierProof => {
                let limit_u16 = request.limit.map(|l| l as u16);
                Ok(DocumentSumResponse::Proof(
                    self.execute_document_sum_range_aggregate_carrier_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        sum_property,
                        limit_u16,
                        order_by_ascending,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
        }
    }
}

/// Determine which executor to dispatch to based on the request's
/// where-shape × mode × prove combination. Pure function; no I/O.
///
/// Returns `Err(WhereClauseOnNonIndexedProperty)` (via
/// [`crate::error::query::QuerySyntaxError`]) if no covering index can
/// be found — same strict-coverage contract count uses, with the
/// addition that the request's `sum_property` must match the chosen
/// index's `summable` declaration.
pub fn detect_sum_mode(request: &DocumentSumRequest) -> Result<DocumentSumMode, Error> {
    use crate::query::drive_document_sum_query::is_range_operator;

    let has_range = request
        .where_clauses
        .iter()
        .any(|wc| is_range_operator(wc.operator));
    let has_in = request
        .where_clauses
        .iter()
        .any(|wc| wc.operator == crate::query::WhereOperator::In);

    // Cross-validate sum property name. The dispatcher rejects up front
    // when the request's `sum_property` doesn't match the doctype-level
    // `documents_summable` (when set) — saves the executor from having
    // to re-check the invariant.
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
    if let Some(doctype_sum) = request.document_type.documents_summable() {
        if doctype_sum != request.sum_property {
            return Err(Error::Drive(crate::error::drive::DriveError::NotSupported(
                "request `sum_property` doesn't match the document type's \
                 `documents_summable`. Sum trees aggregate `i64` per merk node; \
                 mixing property names would produce a meaningless aggregation. \
                 Define a separate index whose `summable: \"<the other name>\"` \
                 covers the alternate aggregation surface.",
            )));
        }
    }

    Ok(match (request.mode, has_range, has_in, request.prove) {
        // No range / no In / no-proof — Total fast path. Covers both
        // empty-where (documents_summable) and Equal-only-fully-
        // covered (summable index lookup) — the executor branches on
        // where_clauses internally. Mirrors count's mapping.
        (SumMode::Aggregate, false, false, false) => DocumentSumMode::Total,
        // No range / has In / no-proof — per-In fan-out.
        (SumMode::Aggregate, false, true, false) => DocumentSumMode::PerInValue,
        (SumMode::Aggregate, true, _, false) => DocumentSumMode::RangeNoProof,
        (SumMode::Aggregate, true, false, true) => DocumentSumMode::RangeProof,
        // Aggregate + no-range + prove: routes to PointLookupProof for
        // both empty-where (documents_summable fast path) AND Equal/In
        // covered cases. The executor branches on `where_clauses`
        // internally.
        (SumMode::Aggregate, false, _, true) => DocumentSumMode::PointLookupProof,
        // GroupByIn: no range — falls back to PerInValue (no-proof)
        // or PointLookupProof (prove). Mirrors count's mapping.
        (SumMode::GroupByIn, false, _, false) => DocumentSumMode::PerInValue,
        (SumMode::GroupByIn, false, _, true) => DocumentSumMode::PointLookupProof,
        // GroupByIn + range: the carrier-ACOR shape (count's
        // RangeAggregateCarrierProof). Same routing on the sum side.
        (SumMode::GroupByIn, true, true, true) => DocumentSumMode::RangeAggregateCarrierProof,
        (SumMode::GroupByIn, true, _, false) => DocumentSumMode::RangeNoProof,
        (SumMode::GroupByRange, true, _, true) => DocumentSumMode::RangeDistinctProof,
        (SumMode::GroupByRange, true, _, false) => DocumentSumMode::RangeNoProof,
        (SumMode::GroupByCompound, true, true, true) => DocumentSumMode::RangeAggregateCarrierProof,
        (SumMode::GroupByCompound, true, true, false) => DocumentSumMode::RangeNoProof,
        _ => {
            return Err(Error::Drive(crate::error::drive::DriveError::NotSupported(
                "sum-query dispatcher: where-shape × mode × prove combination is not \
                 supported; see book/src/drive/document-sum-trees.md's `Choosing What \
                 to Set` table for valid shapes.",
            )));
        }
    })
}

/// Parse the wire-CBOR `Value::Array` shape into structured
/// `Vec<WhereClause>`. Delegates to count's parser.
pub fn where_clauses_from_value(value: &Value) -> Result<Vec<WhereClause>, Error> {
    crate::query::drive_document_count_query::drive_dispatcher::where_clauses_from_value(value)
}

/// Parse the wire-CBOR `Value::Array` shape into structured
/// `Vec<OrderClause>`. Delegates to count's parser.
pub fn order_clauses_from_value(value: &Value) -> Result<Vec<OrderClause>, Error> {
    crate::query::drive_document_count_query::drive_dispatcher::order_clauses_from_value(value)
}
