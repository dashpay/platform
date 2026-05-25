//! Joint count-and-sum dispatcher entry point for the AVG no-prove
//! path.
//!
//! Consumes a [`DocumentAverageRequest`] with `prove = false` and
//! dispatches to one of three per-mode executors (`Total` /
//! `PerInValue` / `RangeNoProof`) based on sum's versioned mode-
//! detection table — see [the routing
//! table](crate::query::drive_document_sum_query::mode_detection) for
//! the per-shape contract.
//!
//! # Per-shape cost
//!
//! - `Total` / `PerInValue`: point-lookup walk against the index;
//!   one grovedb read per In branch yielding `(count, sum)` together
//!   via [`grovedb::Element::count_sum_value_or_default`].
//! - `RangeNoProof` distinct (`GroupByRange` / `GroupByCompound` +
//!   range): one grovedb walk against the
//!   `ProvableCountProvableSumTree` terminators of
//!   `distinct_sum_path_query`, bounded by the request's `limit`
//!   (default `drive_config.default_query_limit`, capped at
//!   `drive_config.max_query_limit`).
//! - `RangeNoProof` aggregate (`Aggregate` / `GroupByIn` + range):
//!   grovedb's combined `query_aggregate_count_and_sum` accumulator
//!   (O(log n), yielding `(u64, i64)` in one traversal). Compound
//!   `In + range` per-In fans out (≤100 branches × 1 accumulator
//!   call) under a shared read transaction.
//!
//! # Atomicity
//!
//! Executors that issue multiple grovedb reads (`PerInValue`, the
//! aggregate `RangeNoProof` branch, and its compound `In + range`
//! per-In fan-out) open a short-lived shared read transaction
//! internally when `transaction.is_none()` so the sub-reads see a
//! consistent grovedb snapshot. Executors that issue exactly one
//! read get atomicity for free.

use super::super::drive_document_average_query::{
    AverageMode, DocumentAverageRequest, DocumentAverageResponse,
};
use super::super::drive_document_count_query::drive_dispatcher::validate_and_canonicalize_where_clauses;
use super::super::drive_document_sum_query::mode_detection::detect_sum_mode_from_inputs;
use super::super::drive_document_sum_query::{DocumentSumMode, SumMode};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

#[cfg(feature = "server")]
impl Drive {
    /// Joint count-and-sum no-prove dispatcher.
    ///
    /// Consumes a [`DocumentAverageRequest`] with `prove = false` and
    /// produces a [`DocumentAverageResponse`] in the appropriate shape
    /// (`Aggregate { count, sum }` for non-grouped modes,
    /// `Entries(Vec<AverageEntry>)` for grouped modes).
    ///
    /// The dispatcher returns `Proof(_)`-shape responses only when
    /// invoked via [`Drive::execute_document_average_request`]'s
    /// `prove = true` arm; the no-prove path here never produces proof
    /// bytes.
    ///
    /// # Routing
    ///
    /// Reuses sum's versioned routing table via
    /// [`detect_sum_mode_from_inputs`] with the request's
    /// [`AverageMode`] converted 1:1 to [`SumMode`] (the two enums are
    /// structurally identical — same four variants). `prove = false`
    /// is passed unconditionally because the prove path never reaches
    /// this function. The resolved [`DocumentSumMode`] is one of three
    /// no-prove values: `Total`, `PerInValue`, or `RangeNoProof`; the
    /// four prove-mode values are unreachable. The match arm therefore
    /// returns `CorruptedCodeExecution` on those branches rather than
    /// silently misbehaving — if the routing table is ever extended
    /// with a new prove-only mode that accidentally fires here, we
    /// want to fail loudly.
    pub fn execute_document_count_and_sum_request(
        &self,
        request: DocumentAverageRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        // No-prove precondition. Direct callers (i.e. anything reaching
        // this method without going through
        // `execute_document_average_request`'s prove/no-prove split)
        // must not slip a `prove = true` request past — this dispatcher
        // hard-routes with `prove = false` and would otherwise silently
        // hand back a no-prove response to a caller that asked for a
        // proof. Reject up front rather than honoring the routing
        // mismatch.
        if request.prove {
            return Err(Error::Drive(
                crate::error::drive::DriveError::CorruptedCodeExecution(
                    "execute_document_count_and_sum_request only serves the no-prove path. \
                     For a proven AVG response use \
                     `execute_document_average_request` (which routes prove=true to the \
                     prove-side dispatcher).",
                ),
            ));
        }

        // Validate + canonicalize the structured `where_clauses` —
        // same rejections / canonicalization the count and document-
        // query surfaces apply, kept here so the AVG no-prove surface
        // shares the same accept/reject contract. Must run BEFORE
        // `detect_sum_mode_from_inputs` because the canonicalizer
        // collapses `[> A, < B]` pairs into a single `between*`
        // clause whose presence changes the routing decision. See
        // [`validate_and_canonicalize_where_clauses`]'s docstring for
        // the catalog of rejections.
        let where_clauses =
            validate_and_canonicalize_where_clauses(request.where_clauses, platform_version)?;
        let resolved_time_range_fields = request.resolved_time_range_fields;
        crate::query::validate_resolved_time_range_clause_shapes(
            &where_clauses,
            &resolved_time_range_fields,
        )?;

        // Convert AverageMode → SumMode (1:1 by construction); sum's
        // routing table is the single source of truth for the
        // `(where_clauses × mode × prove=false)` triple. Forking a
        // parallel "count_and_sum mode" table here is what #3687
        // explicitly warned against — PR #3661 caught one drift bug
        // (count's `GroupByCompound` row had diverged from sum's),
        // and the joint executor's existence is itself the reason a
        // single source of truth is now mandatory.
        let sum_mode = match request.mode {
            AverageMode::Aggregate => SumMode::Aggregate,
            AverageMode::GroupByIn => SumMode::GroupByIn,
            AverageMode::GroupByRange => SumMode::GroupByRange,
            AverageMode::GroupByCompound => SumMode::GroupByCompound,
        };

        let resolved_mode =
            detect_sum_mode_from_inputs(&where_clauses, sum_mode, false, platform_version)?;

        let contract_id = request.contract.id().to_buffer();
        let document_type_name = request.document_type.name().to_string();
        let sum_property = request.sum_property;
        let order_by_ascending = request
            .order_clauses
            .first()
            .map(|c| c.ascending)
            .unwrap_or(true);

        match resolved_mode {
            DocumentSumMode::Total => self.execute_document_count_and_sum_total_no_proof(
                contract_id,
                request.document_type,
                document_type_name,
                where_clauses,
                &resolved_time_range_fields,
                sum_property,
                transaction,
                platform_version,
            ),
            DocumentSumMode::PerInValue => {
                // PerInValue's per-In fan-out is bounded by the
                // `In::in_values()` 100-cap. The caller's `limit`
                // applies to the returned entry list per
                // [`DocumentAverageRequest::limit`]'s documented
                // no-proof contract: unset → fall back to
                // `drive_config.default_query_limit`, explicit > max
                // clamps to `drive_config.max_query_limit`. Count's
                // PerInValue arm hard-codes `MAX_LIMIT_AS_FAILSAFE`
                // (which contradicts its own documented contract);
                // the joint dispatcher honors the AVG contract
                // instead.
                let per_in_limit = request
                    .limit
                    .unwrap_or(request.drive_config.default_query_limit as u32)
                    .min(request.drive_config.max_query_limit as u32);
                self.execute_document_count_and_sum_per_in_value_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    &resolved_time_range_fields,
                    sum_property,
                    order_by_ascending,
                    per_in_limit as u16,
                    transaction,
                    platform_version,
                )
            }
            DocumentSumMode::RangeNoProof => {
                // Distinct flag set for the GroupByRange /
                // GroupByCompound shapes (per-distinct-value entries);
                // cleared for the Aggregate / GroupByIn shapes (single
                // folded pair).
                let return_distinct = matches!(
                    request.mode,
                    AverageMode::GroupByRange | AverageMode::GroupByCompound
                );
                // Limit applies only to the distinct branches — the
                // aggregate branches return a single collapsed pair
                // bounded by grovedb's combined merk-internal
                // accumulator (`query_aggregate_count_and_sum`)
                // regardless of how many documents match. For distinct
                // shapes, follow [`DocumentAverageRequest::limit`]'s
                // documented no-proof contract: fall back to
                // `drive_config.default_query_limit` when unset, clamp
                // to `drive_config.max_query_limit` when over. Both
                // `default_query_limit` and `max_query_limit` are
                // `u16` in `DriveConfig`, so the `min()` keeps the
                // result in `u16::MAX` regardless of caller input.
                let limit_u16 = if return_distinct {
                    Some(
                        request
                            .limit
                            .unwrap_or(request.drive_config.default_query_limit as u32)
                            .min(request.drive_config.max_query_limit as u32)
                            as u16,
                    )
                } else {
                    None
                };
                let response = self.execute_document_count_and_sum_range_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    &resolved_time_range_fields,
                    sum_property,
                    return_distinct,
                    order_by_ascending,
                    limit_u16,
                    transaction,
                    platform_version,
                )?;
                // Sum's dispatcher applies a second-stage shape on the
                // RangeNoProof path: `Aggregate` mode collapses to a
                // single value, every other mode (GroupByIn /
                // GroupByRange / GroupByCompound) returns `Entries`.
                // GroupByIn + range specifically returns
                // `Entries(vec![one_entry])` with the In axis folded
                // (sum's executor itself emits exactly one entry in
                // that case — see its `execute_range_sum_no_proof`
                // compound-summed branch). The joint executor returns
                // `Aggregate { count, sum }` for the !distinct shape
                // because it has both metrics on hand; we re-shape
                // into `Entries(vec![one_entry])` here when the
                // caller asked for a non-Aggregate mode so the wire
                // shape matches what an independent sum + count
                // GroupByIn dispatch would produce.
                // Strict mode×shape pairing. Every legal combination
                // is named explicitly; any other pairing surfaces as
                // `CorruptedCodeExecution` rather than silently
                // forwarding a wrong-shape response. A future executor
                // change that emits, say, `Entries` from an
                // `AverageMode::Aggregate` request would fail loudly
                // here instead of leaking the wrong wire shape to the
                // gRPC handler.
                match (request.mode, response) {
                    (AverageMode::Aggregate, resp @ DocumentAverageResponse::Aggregate { .. }) => {
                        Ok(resp)
                    }
                    (AverageMode::GroupByIn, DocumentAverageResponse::Aggregate { count, sum }) => {
                        Ok(DocumentAverageResponse::Entries(vec![
                            super::super::drive_document_average_query::AverageEntry {
                                in_key: None,
                                key: Vec::new(),
                                count: Some(count),
                                sum: Some(sum),
                            },
                        ]))
                    }
                    (
                        AverageMode::GroupByRange | AverageMode::GroupByCompound,
                        resp @ DocumentAverageResponse::Entries(_),
                    ) => Ok(resp),
                    (mode, _) => Err(Error::Drive(
                        crate::error::drive::DriveError::CorruptedCodeExecution(match mode {
                            AverageMode::Aggregate => {
                                "execute_document_count_and_sum_request: \
                                     RangeNoProof executor emitted a non-Aggregate \
                                     response for AverageMode::Aggregate — joint \
                                     range executor's shape contract violated"
                            }
                            AverageMode::GroupByIn => {
                                "execute_document_count_and_sum_request: \
                                     RangeNoProof executor emitted a non-Aggregate \
                                     response for AverageMode::GroupByIn — the \
                                     in-axis fold should yield a single (count, sum) \
                                     pair the dispatcher re-wraps as Entries"
                            }
                            AverageMode::GroupByRange | AverageMode::GroupByCompound => {
                                "execute_document_count_and_sum_request: \
                                     RangeNoProof executor emitted a non-Entries \
                                     response for a distinct grouped mode — joint \
                                     range executor's shape contract violated"
                            }
                        }),
                    )),
                }
            }
            // The four prove-mode resolutions are unreachable here —
            // the dispatcher passes `prove = false` to the routing
            // table, and sum's v0 table has no row that maps a
            // `prove=false` input to any of these four resolutions.
            // Surface as `CorruptedCodeExecution` so a future routing-
            // table change that breaks this assumption fails loudly
            // rather than producing a runtime panic.
            DocumentSumMode::RangeProof
            | DocumentSumMode::RangeDistinctProof
            | DocumentSumMode::PointLookupProof
            | DocumentSumMode::RangeAggregateCarrierProof => Err(Error::Drive(
                crate::error::drive::DriveError::CorruptedCodeExecution(
                    "execute_document_count_and_sum_request: sum's routing table \
                         resolved a prove-only mode from a `prove = false` request — \
                         this indicates the routing table has been extended without \
                         updating the joint dispatcher's match arms",
                ),
            )),
        }
    }
}
