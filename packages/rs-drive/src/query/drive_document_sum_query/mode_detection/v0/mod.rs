//! v0 of [`super::detect_sum_mode`]. Routing table extracted
//! verbatim from the previous inline implementation in
//! `drive_document_sum_query::drive_dispatcher` so the v1
//! cutover is a pure code move with no semantic change.

use crate::error::Error;
use crate::query::drive_document_sum_query::{
    is_range_operator, DocumentSumMode, DocumentSumRequest, SumMode,
};
use crate::query::WhereOperator;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;

pub(super) fn detect_sum_mode_v0(request: &DocumentSumRequest) -> Result<DocumentSumMode, Error> {
    let has_range = request
        .where_clauses
        .iter()
        .any(|wc| is_range_operator(wc.operator));
    let has_in = request
        .where_clauses
        .iter()
        .any(|wc| wc.operator == WhereOperator::In);

    // Cross-validate sum property name. The dispatcher rejects up front
    // when the request's `sum_property` doesn't match the doctype-level
    // `documents_summable` (when set) — saves the executor from having
    // to re-check the invariant.
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
