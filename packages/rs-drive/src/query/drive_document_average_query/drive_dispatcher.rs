//! Average-query dispatcher entry point.
//!
//! Implementation strategy: **compose** count + sum into the
//! `(count, sum)` pair the client divides. Both executors are real
//! and live in `drive_document_count_query` /
//! `drive_document_sum_query` respectively; the average dispatcher
//! issues both requests under the same `transaction` and zips their
//! responses together by `(in_key, key)` for grouped shapes.
//!
//! ## Why compose instead of using a single PCPS traversal?
//!
//! grovedb's `AggregateCountAndSumOnRange` primitive returns both
//! metrics from one root-hash-committed traversal — cheaper on the
//! wire and atomic — but it only fires when the chosen index has
//! a `ProvableCountProvableSumTree` terminator (i.e. `rangeCountable
//! + rangeSummable`). For doctypes/indexes that lack PCPS-eligibility
//! (just `documentsSummable` without `rangeCountable`, for example)
//! the no-prove path has to compose two reads instead:
//!
//! - **No-prove paths**: count + sum are read within the same
//!   grovedb snapshot, so they see identical state (no block-
//!   boundary race, no off-by-one). When the caller passes a
//!   `TransactionArg::None` (the drive-abci query path), the
//!   dispatcher opens a short-lived read transaction internally and
//!   reuses it across both sub-calls so the atomicity guarantee
//!   holds regardless of caller plumbing. The internal transaction
//!   is rolled back at the end (read-only, never commits).
//! - **Prove path**: dispatched to
//!   [`Drive::execute_document_average_prove`] (defined below),
//!   which routes to one of the PCPS / direct-read prove executors
//!   based on `(mode, where_clauses)`:
//!     - empty-where + `documentsCountable + documentsSummable`
//!       doctype → primary-key count-sum tree direct read
//!     - range AVG on a `rangeAverageable` index → PCPS
//!       `AggregateCountAndSumOnRange` proof
//!     - In + range AVG on a `rangeAverageable` index → carrier-PCPS
//!       proof
//!     - GroupByRange / GroupByCompound + range on a
//!       `rangeAverageable` index → per-distinct-key
//!       count-and-sum proof (walks `ProvableCountProvableSumTree`
//!       terminators)
//!     - Equal/In + no range on a summable + countable index →
//!       point-lookup count-and-sum proof (walks count-sum-bearing
//!       terminator elements)
//!   The client verifies with the matching
//!   `verify_*_count_and_sum_proof` helpers in `drive-proof-verifier`.

use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_average_query::{
    AverageEntry, AverageMode, DocumentAverageRequest, DocumentAverageResponse,
};
use crate::query::drive_document_count_query::{
    CountMode, DocumentCountRequest, DocumentCountResponse,
};
use crate::query::drive_document_sum_query::index_picker::{
    find_range_summable_index_for_where_clauses, find_summable_index_for_where_clauses,
};
use crate::query::drive_document_sum_query::{
    is_range_operator, DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumMode,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

#[cfg(feature = "server")]
impl Drive {
    /// Server-side entry point for the average surface. Composes the
    /// count + sum executors and zips their outputs into the
    /// `(count, sum)` pair the client divides.
    ///
    /// See the module docstring for the rationale on composition vs.
    /// a single PCPS traversal.
    pub fn execute_document_average_request(
        &self,
        request: DocumentAverageRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        if request.prove {
            return self.execute_document_average_prove(request, transaction, platform_version);
        }

        // Map `AverageMode` → matching `CountMode` / `SumMode`. The
        // three enums are structurally identical (same four variants);
        // each pair just lives in its own namespace.
        let (count_mode, sum_mode) = match request.mode {
            AverageMode::Aggregate => (CountMode::Aggregate, SumMode::Aggregate),
            AverageMode::GroupByIn => (CountMode::GroupByIn, SumMode::GroupByIn),
            AverageMode::GroupByRange => (CountMode::GroupByRange, SumMode::GroupByRange),
            AverageMode::GroupByCompound => (CountMode::GroupByCompound, SumMode::GroupByCompound),
        };

        // Build parallel sub-requests. Both consume the same
        // `where_clauses` + `order_clauses` + `limit` + (false) `prove`
        // — the average's shape contract is "two reads of the same
        // grovedb snapshot, zipped after."
        //
        // Architectural follow-up: tracked at
        // [dashpay/platform#3687](https://github.com/dashpay/platform/issues/3687).
        // The two-sub-request shape will collapse into a single
        // `DocumentCountSumRequest` + a unified
        // `execute_document_count_and_sum_request` that walks
        // grovedb once and reads both metrics from each visited PCPS
        // element via `count_sum_value_or_default()`. The prove path
        // at `execute_document_average_prove` below already does
        // this (one PCPS walk yields both fields); the no-proof
        // path currently double-walks. The current two-request
        // shape is correct (the local transaction below guarantees
        // atomicity); it just does more grovedb work than strictly
        // necessary, and the dual-routing requires count's and sum's
        // routing tables to stay in lock-step for AVG composition to
        // work (already caught one routing divergence). Issue #3687
        // captures the full scope including the four joint per-mode
        // no-proof executors that need to land.
        let count_request = DocumentCountRequest {
            contract: request.contract,
            document_type: request.document_type,
            where_clauses: request.where_clauses.clone(),
            order_clauses: request.order_clauses.clone(),
            mode: count_mode,
            limit: request.limit,
            prove: false,
            drive_config: request.drive_config,
        };
        let sum_request = DocumentSumRequest {
            contract: request.contract,
            document_type: request.document_type,
            sum_property: request.sum_property,
            where_clauses: request.where_clauses,
            order_clauses: request.order_clauses,
            mode: sum_mode,
            limit: request.limit,
            prove: false,
            drive_config: request.drive_config,
        };

        // Atomicity: both sub-reads must see the same grovedb root. If
        // the caller didn't provide a transaction we open a short-lived
        // read transaction here and reuse it across both executors so
        // a concurrent block commit can't slip between the count and
        // sum reads (the attacker-steerable race documented in the
        // module-level docstring). The local transaction is read-only
        // and dropped without commit at the end of this function.
        let local_tx;
        let effective_transaction: TransactionArg = if transaction.is_some() {
            transaction
        } else {
            local_tx = self.grove.start_transaction();
            Some(&local_tx)
        };

        let count_response = self.execute_document_count_request(
            count_request,
            effective_transaction,
            platform_version,
        )?;
        let sum_response = self.execute_document_sum_request(
            sum_request,
            effective_transaction,
            platform_version,
        )?;

        // Combine. The two executors emit either Aggregate or Entries
        // (Proof is unreachable here since `prove=false` above). The
        // mode-pair is symmetric so they must agree on which shape
        // they emit — mismatches indicate a routing bug, surface as
        // CorruptedCodeExecution.
        match (count_response, sum_response) {
            (DocumentCountResponse::Aggregate(count), DocumentSumResponse::Aggregate(sum)) => {
                Ok(DocumentAverageResponse::Aggregate { count, sum })
            }
            (
                DocumentCountResponse::Entries(count_entries),
                DocumentSumResponse::Entries(sum_entries),
            ) => Ok(DocumentAverageResponse::Entries(zip_entries(
                count_entries,
                sum_entries,
            )?)),
            // Mismatched shapes — count executor and sum executor
            // disagreed on whether the result fits in a single row.
            // Should be impossible because they share the same mode
            // and `validate_and_canonicalize_where_clauses` runs the
            // same checks on both.
            _ => Err(Error::Drive(
                crate::error::drive::DriveError::CorruptedCodeExecution(
                    "average composition: count and sum executors emitted disagreeing \
                     response shapes — both should agree on Aggregate vs Entries given \
                     identical mode + where + group_by",
                ),
            )),
        }
    }

    /// Prove path of [`Self::execute_document_average_request`].
    ///
    /// Routes the `(where_clauses × mode)` pair to one of the
    /// available PCPS / direct-read prove executors and returns
    /// proof bytes the client verifies with the matching
    /// `verify_*_count_and_sum_proof` helper.
    ///
    /// Supported prove shapes:
    /// - `Aggregate` + empty where + doctype's primary key tree is a
    ///   count-sum-bearing variant (`CountSumTree` /
    ///   `ProvableCountSumTree` /
    ///   `ProvableCountProvableSumTree`) — proves the primary-key
    ///   element directly via `primary_key_sum_path_query`. Client
    ///   verifies with `verify_primary_key_count_sum_tree_proof`.
    /// - `Aggregate` + range clause on a PCPS-eligible index
    ///   (`rangeCountable: true` AND `rangeSummable: true`) — proves
    ///   via `execute_aggregate_count_and_sum_with_proof`. Client
    ///   verifies with `verify_aggregate_count_and_sum_proof`.
    /// - `Aggregate` + Equal/In, no range, on a count+sum index
    ///   (or doctype's count-sum primary key) — proves via
    ///   `execute_point_lookup_sum_with_proof`. Client verifies
    ///   with `verify_point_lookup_count_and_sum_proof`.
    /// - `GroupByIn` + In + range on a PCPS-eligible index — proves
    ///   via `execute_carrier_aggregate_count_and_sum_with_proof`.
    ///   Client verifies with
    ///   `verify_carrier_aggregate_count_and_sum_proof`.
    /// - `GroupByRange` / `GroupByCompound` + range on a PCPS-
    ///   eligible index — proves via
    ///   `execute_distinct_sum_with_proof` against a path query
    ///   whose terminator value trees are
    ///   `ProvableCountProvableSumTree`. Client verifies with
    ///   `verify_distinct_count_and_sum_proof`.
    fn execute_document_average_prove(
        &self,
        request: DocumentAverageRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        let contract_id = request.contract.id().to_buffer();
        let document_type_name = request.document_type.name().to_string();
        let has_range = request
            .where_clauses
            .iter()
            .any(|wc| is_range_operator(wc.operator));
        let order_by_ascending = request
            .order_clauses
            .first()
            .map(|c| c.ascending)
            .unwrap_or(true);

        // Empty-where AVG fast path: prove the primary-key
        // count-sum-bearing element directly when the doctype
        // declares both `documentsCountable: true` (implied by
        // having a CountSumTree primary key) and a matching
        // `documents_summable`. The verifier extracts `(count,
        // sum)` from one element.
        if matches!(request.mode, AverageMode::Aggregate)
            && request.where_clauses.is_empty()
            && request.document_type.documents_countable()
            && request
                .document_type
                .documents_summable()
                .map(|p| p == request.sum_property)
                .unwrap_or(false)
        {
            let path_query =
                DriveDocumentSumQuery::primary_key_sum_path_query(contract_id, &document_type_name);
            let proof = self
                .grove
                .get_proved_path_query(
                    &path_query,
                    None,
                    transaction,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Range AVG: pick a PCPS-eligible index (range_countable
        // AND range_summable) covering the where clauses. Mirror of
        // sum's `find_range_summable_index_for_where_clauses` with
        // an additional `range_countable` filter.
        if has_range
            && matches!(
                request.mode,
                AverageMode::Aggregate | AverageMode::GroupByIn
            )
        {
            let index = find_range_summable_index_for_where_clauses(
                request.document_type.indexes(),
                &request.where_clauses,
                &request.sum_property,
            )
            .filter(|idx| idx.range_countable)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "prove AVG requires an index that declares BOTH `rangeCountable: \
                     true` AND `rangeSummable: true` (a `rangeAverageable: true` \
                     index is the shorthand) whose last property matches the range \
                     field and whose summable property matches the request's \
                     `sum_property`"
                        .to_string(),
                ))
            })?;
            let sum_query = DriveDocumentSumQuery {
                document_type: request.document_type,
                contract_id,
                document_type_name,
                index,
                where_clauses: request.where_clauses.clone(),
                sum_property: request.sum_property.clone(),
            };

            let proof = match request.mode {
                AverageMode::Aggregate => sum_query.execute_aggregate_count_and_sum_with_proof(
                    self,
                    transaction,
                    platform_version,
                )?,
                AverageMode::GroupByIn => {
                    // Carrier-PCPS: one (count, sum) per In branch.
                    // Validate-don't-clamp limit policy on the prove
                    // path — `SizedQuery::limit` is bytes-of-proof
                    // material; silent clamping would byte-differ the
                    // SDK's reconstruction and break verification.
                    // Same contract as sum's `RangeAggregateCarrierProof`
                    // arm. `None` stays `None` (unbounded outer walk).
                    let limit_u16 = request
                        .limit
                        .map(|l| {
                            if l > request.drive_config.max_query_limit as u32 {
                                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                                    "limit {} exceeds max_query_limit {} on the prove + \
                                         carrier-aggregate path (GROUP BY In + range, AVG); \
                                         reduce the requested limit or use prove = false",
                                    l, request.drive_config.max_query_limit
                                ))));
                            }
                            u16::try_from(l).map_err(|_| {
                                Error::Query(QuerySyntaxError::Unsupported(format!(
                                    "limit {} exceeds u16::MAX for carrier-aggregate \
                                     count+sum (AVG) proof",
                                    l
                                )))
                            })
                        })
                        .transpose()?;
                    sum_query.execute_carrier_aggregate_count_and_sum_with_proof(
                        self,
                        limit_u16,
                        order_by_ascending,
                        transaction,
                        platform_version,
                    )?
                }
                _ => unreachable!("outer matches! gate filters out non-Aggregate/GroupByIn"),
            };
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Distinct AVG (GroupByRange / GroupByCompound + range) —
        // per-distinct-key (count, sum) proof against a PCPS-
        // eligible index (rangeCountable + rangeSummable, i.e. a
        // `rangeAverageable: true` index). The prover uses sum's
        // `execute_distinct_sum_with_proof` against a path query
        // whose terminators are `ProvableCountProvableSumTree`; the
        // verifier extracts `count_sum_value_or_default()` from
        // each emitted element.
        if has_range
            && matches!(
                request.mode,
                AverageMode::GroupByRange | AverageMode::GroupByCompound
            )
        {
            let index = find_range_summable_index_for_where_clauses(
                request.document_type.indexes(),
                &request.where_clauses,
                &request.sum_property,
            )
            .filter(|idx| idx.range_countable)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "prove distinct AVG requires an index that declares BOTH \
                     `rangeCountable: true` AND `rangeSummable: true` (a \
                     `rangeAverageable: true` index is the shorthand) whose last \
                     property matches the range field and whose summable property \
                     matches the request's `sum_property`"
                        .to_string(),
                ))
            })?;
            // Validate-don't-clamp limit policy on the prove path —
            // see sum's `RangeDistinctProof` arm for the full
            // rationale. Limit fallback uses
            // [`crate::config::DEFAULT_QUERY_LIMIT`] (compile-time
            // constant) so the SDK's reconstruction lands on the same
            // `SizedQuery::limit` value; `max_query_limit` still
            // gates as a DoS ceiling.
            let effective_limit = request
                .limit
                .unwrap_or(crate::config::DEFAULT_QUERY_LIMIT as u32);
            if effective_limit > request.drive_config.max_query_limit as u32 {
                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                    "limit {} exceeds max_query_limit {} on the prove + distinct-walk \
                     path (GROUP BY a range field, AVG); reduce the requested limit \
                     or use prove = false",
                    effective_limit, request.drive_config.max_query_limit
                ))));
            }
            let limit_u16 = u16::try_from(effective_limit).map_err(|_| {
                Error::Query(QuerySyntaxError::Unsupported(format!(
                    "limit {} exceeds u16::MAX for distinct AVG proof",
                    effective_limit
                )))
            })?;
            let sum_query = DriveDocumentSumQuery {
                document_type: request.document_type,
                contract_id,
                document_type_name,
                index,
                where_clauses: request.where_clauses.clone(),
                sum_property: request.sum_property.clone(),
            };
            let proof = sum_query.execute_distinct_sum_with_proof(
                self,
                limit_u16,
                order_by_ascending,
                transaction,
                platform_version,
            )?;
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Point-lookup AVG: `Aggregate` + Equal/In on a count+sum
        // index (whose `summable.is_some()` AND `countable.is_countable()`)
        // OR doctype-level documentsSummable + documentsCountable
        // for the empty-where case (which is the fast path above —
        // this arm handles the non-empty-where Equal/In shape).
        // Server uses sum's `execute_point_lookup_sum_with_proof`
        // against an index whose terminator value trees are count-
        // sum-bearing; the verifier extracts
        // `count_sum_value_or_default()` from each element.
        if !has_range && matches!(request.mode, AverageMode::Aggregate) {
            let index = find_summable_index_for_where_clauses(
                request.document_type.indexes(),
                &request.where_clauses,
                &request.sum_property,
            )
            .filter(|idx| idx.countable.is_countable())
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "prove point-lookup AVG requires an index that declares BOTH \
                     `summable: \"<prop>\"` AND a countable terminator (`countable: \
                     \"countable\"` or `\"countableAllowingOffset\"`) whose properties \
                     exactly match the where clause fields"
                        .to_string(),
                ))
            })?;
            let sum_query = DriveDocumentSumQuery {
                document_type: request.document_type,
                contract_id,
                document_type_name,
                index,
                where_clauses: request.where_clauses.clone(),
                sum_property: request.sum_property.clone(),
            };
            let proof = sum_query.execute_point_lookup_sum_with_proof(
                self,
                transaction,
                platform_version,
            )?;
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Unreachable in practice — the matches!() gates above
        // cover every (mode × has_range) combination today. Kept as
        // a typed error in case a future AverageMode variant lands
        // without a corresponding prove arm.
        Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "execute_document_average_request prove=true: the (mode = {:?}, has_range \
             = {}) combination is not yet supported on the prove path. \
             This is likely a new AverageMode variant that hasn't been wired \
             into the prove dispatcher.",
            request.mode, has_range,
        ))))
    }
}

/// Merge per-`(in_key, key)` count entries and sum entries into average
/// entries via a strict two-pointer merge keyed on `(in_key, key)`.
///
/// Both inputs are emitted by the same executor family with identical
/// `where_clauses` / `order_clauses` / `mode` against the same grovedb
/// snapshot, so they MUST emit the same set of keys in the same
/// ascending `(in_key, key)` order. Any divergence (key on one side
/// only, or different ordering) indicates an executor bug and is
/// surfaced as `CorruptedCodeExecution` rather than silently zeroed at
/// the wire layer — the previous defensive `None`-preservation pattern
/// was indistinguishable from "this key matched zero documents but the
/// sum is nonzero" once the wire mapping flattened `Option<u64>` →
/// `u64`, which let attacker-timed inserts between the two reads
/// produce a `count=0, sum=V` bucket that crashed naive `sum / count`
/// clients with a divide-by-zero. With atomicity now enforced inside
/// `execute_document_average_request` (see module docstring), the only
/// remaining cause of divergence is a real executor bug — treating it
/// as fatal is correct.
///
/// Output is always strictly ascending by `(in_key, key)` (same order
/// the inputs are required to be in).
#[cfg(feature = "server")]
fn zip_entries(
    count_entries: Vec<crate::query::SplitCountEntry>,
    sum_entries: Vec<crate::query::SumEntry>,
) -> Result<Vec<AverageEntry>, Error> {
    use crate::error::drive::DriveError;

    let mut out = Vec::with_capacity(count_entries.len().max(sum_entries.len()));
    let mut c_iter = count_entries.into_iter();
    let mut s_iter = sum_entries.into_iter();
    let mut next_c = c_iter.next();
    let mut next_s = s_iter.next();

    loop {
        match (&next_c, &next_s) {
            (Some(c), Some(s)) => {
                let c_key = (&c.in_key, &c.key);
                let s_key = (&s.in_key, &s.key);
                match c_key.cmp(&s_key) {
                    std::cmp::Ordering::Equal => {
                        let c = next_c.take().expect("checked Some above");
                        let s = next_s.take().expect("checked Some above");
                        out.push(AverageEntry {
                            in_key: c.in_key,
                            key: c.key,
                            count: c.count,
                            sum: s.sum,
                        });
                        next_c = c_iter.next();
                        next_s = s_iter.next();
                    }
                    std::cmp::Ordering::Less => {
                        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "average composition: count executor emitted a (in_key, key) the \
                             sum executor didn't — both executors run identical inputs against \
                             the same grovedb snapshot, so divergence indicates an executor bug",
                        )));
                    }
                    std::cmp::Ordering::Greater => {
                        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "average composition: sum executor emitted a (in_key, key) the \
                             count executor didn't — both executors run identical inputs against \
                             the same grovedb snapshot, so divergence indicates an executor bug",
                        )));
                    }
                }
            }
            (Some(_), None) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "average composition: count executor produced more entries than sum executor \
                     — both executors run identical inputs against the same grovedb snapshot, \
                     so divergence indicates an executor bug",
                )));
            }
            (None, Some(_)) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "average composition: sum executor produced more entries than count executor \
                     — both executors run identical inputs against the same grovedb snapshot, \
                     so divergence indicates an executor bug",
                )));
            }
            (None, None) => break,
        }
    }
    Ok(out)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::query::{SplitCountEntry, SumEntry};

    fn cc(in_key: Option<&[u8]>, key: &[u8], count: u64) -> SplitCountEntry {
        SplitCountEntry {
            in_key: in_key.map(|b| b.to_vec()),
            key: key.to_vec(),
            count: Some(count),
        }
    }
    fn ss(in_key: Option<&[u8]>, key: &[u8], sum: i64) -> SumEntry {
        SumEntry {
            in_key: in_key.map(|b| b.to_vec()),
            key: key.to_vec(),
            sum: Some(sum),
        }
    }

    #[test]
    fn zip_entries_merges_aligned_streams_in_ascending_order() {
        let count_entries = vec![cc(None, b"a", 1), cc(None, b"b", 2), cc(None, b"c", 3)];
        let sum_entries = vec![ss(None, b"a", 10), ss(None, b"b", 20), ss(None, b"c", 30)];
        let out = zip_entries(count_entries, sum_entries).expect("aligned streams must merge");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].key, b"a");
        assert_eq!(out[0].count, Some(1));
        assert_eq!(out[0].sum, Some(10));
        assert_eq!(out[2].key, b"c");
        assert_eq!(out[2].count, Some(3));
        assert_eq!(out[2].sum, Some(30));
    }

    #[test]
    fn zip_entries_errors_when_count_has_an_extra_key() {
        // count has `b` but sum doesn't — strict merge must reject.
        let count_entries = vec![cc(None, b"a", 1), cc(None, b"b", 2)];
        let sum_entries = vec![ss(None, b"a", 10)];
        let err = zip_entries(count_entries, sum_entries)
            .expect_err("divergent streams must surface as CorruptedCodeExecution");
        assert!(
            matches!(err, Error::Drive(DriveError::CorruptedCodeExecution(_))),
            "expected CorruptedCodeExecution, got {err:?}",
        );
    }

    #[test]
    fn zip_entries_errors_when_sum_has_an_extra_key() {
        let count_entries = vec![cc(None, b"a", 1)];
        let sum_entries = vec![ss(None, b"a", 10), ss(None, b"b", 20)];
        let err = zip_entries(count_entries, sum_entries)
            .expect_err("divergent streams must surface as CorruptedCodeExecution");
        assert!(
            matches!(err, Error::Drive(DriveError::CorruptedCodeExecution(_))),
            "expected CorruptedCodeExecution, got {err:?}",
        );
    }

    #[test]
    fn zip_entries_errors_when_streams_disagree_on_a_key_in_the_middle() {
        // count has `b`, sum has `c` between the matching `a` and `d`.
        let count_entries = vec![cc(None, b"a", 1), cc(None, b"b", 2), cc(None, b"d", 4)];
        let sum_entries = vec![ss(None, b"a", 10), ss(None, b"c", 30), ss(None, b"d", 40)];
        let err = zip_entries(count_entries, sum_entries)
            .expect_err("middle-of-stream divergence must surface as CorruptedCodeExecution");
        assert!(matches!(
            err,
            Error::Drive(DriveError::CorruptedCodeExecution(_))
        ));
    }

    #[test]
    fn zip_entries_handles_compound_in_key_ordering() {
        // (Some("X"), "a") < (Some("X"), "b") < (Some("Y"), "a") in
        // lexicographic order — verify the merge follows it.
        let count_entries = vec![
            cc(Some(b"X"), b"a", 1),
            cc(Some(b"X"), b"b", 2),
            cc(Some(b"Y"), b"a", 3),
        ];
        let sum_entries = vec![
            ss(Some(b"X"), b"a", 10),
            ss(Some(b"X"), b"b", 20),
            ss(Some(b"Y"), b"a", 30),
        ];
        let out = zip_entries(count_entries, sum_entries).expect("aligned compound merge");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].in_key.as_deref(), Some(b"X".as_ref()));
        assert_eq!(out[0].key, b"a");
        assert_eq!(out[2].in_key.as_deref(), Some(b"Y".as_ref()));
        assert_eq!(out[2].key, b"a");
    }

    // ── Dispatcher limit-policy regression tests ───────────────────
    //
    // AVG-side analogs of count's
    // `test_range_distinct_proof_uses_compile_time_default_query_limit_not_operator_config`
    // and the sum-side tests in `drive_document_sum_query/tests.rs`'s
    // `limit_policy_regression` module. The AVG dispatcher's
    // `RangeDistinctProof` arm mirrors the same validate-don't-clamp
    // policy on the prove path; these tests pin that the dispatcher
    // uses [`crate::config::DEFAULT_QUERY_LIMIT`] (compile-time
    // constant) rather than the operator-tunable
    // `drive_config.default_query_limit`, AND that an explicit
    // `limit > max_query_limit` returns a typed
    // `QuerySyntaxError::InvalidLimit` instead of silently clamping.
    //
    // The AVG distinct path internally calls
    // `execute_distinct_sum_with_proof` (the same primitive sum's
    // RangeDistinctProof uses — see `drive_document_average_query/
    // drive_dispatcher.rs::execute_document_average_prove`); the
    // distinction is the index requirement (`rangeCountable +
    // rangeSummable`, i.e. PCPS / `rangeAverageable`) and the
    // verifier helper (`verify_aggregate_count_and_sum_query`).

    use crate::config::{DriveConfig, DEFAULT_QUERY_LIMIT};
    use crate::drive::Drive;
    use crate::error::query::QuerySyntaxError;
    use crate::query::drive_document_average_query::{
        AverageMode, DocumentAverageRequest, DocumentAverageResponse,
    };
    use crate::query::{WhereClause, WhereOperator};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use grovedb::GroveDb;
    use std::borrow::Cow;
    use std::collections::BTreeMap as StdBTreeMap;

    const PROTOCOL_VERSION_V12: u32 = 12;

    /// v12 contract with a `widget` doctype carrying a single
    /// `(color, amount)` `rangeAverageable: true` (= `rangeCountable +
    /// rangeSummable`) index. The PCPS combined `byColor` index is
    /// what the AVG `RangeDistinctProof` arm walks.
    fn build_widget_contract_pcps() -> dpp::data_contract::DataContract {
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                // rangeAverageable is shorthand for rangeCountable +
                // rangeSummable on the same summable property. The
                // DPP parser desugars it into both flags; the picker
                // routes it through the PCPS path.
                "summable":        "amount",
                "rangeSummable":   true,
                "countable":       "countable",
                "rangeCountable":  true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned()
    }

    fn insert_widget(
        drive: &Drive,
        contract: &dpp::data_contract::DataContract,
        i: usize,
        color: &str,
        amount: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget type exists");
        let mut properties = StdBTreeMap::new();
        properties.insert("color".to_string(), Value::Text(color.to_string()));
        properties.insert("amount".to_string(), Value::U64(amount));
        let document: Document = DocumentV0 {
            id: Identifier::from([(i + 1) as u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert widget");
    }

    /// AVG mirror of the SUM/count regression: with
    /// `drive_config.default_query_limit = 1` and a `limit = None`
    /// request, the dispatcher must use `DEFAULT_QUERY_LIMIT` (= 100)
    /// for the prove path's `SizedQuery::limit`. If it regressed to
    /// using the runtime `default_query_limit`, the reconstructed
    /// path query would byte-differ and `verify_aggregate_count_and_sum_query`
    /// would return Err — exactly the silent-verify-failure surface
    /// this test guards.
    #[test]
    fn range_distinct_avg_proof_uses_compile_time_default_query_limit_not_operator_config() {
        const OPERATOR_TUNED_LIMIT: u16 = 1;
        assert_ne!(
            DEFAULT_QUERY_LIMIT, OPERATOR_TUNED_LIMIT,
            "test invariant: OPERATOR_TUNED_LIMIT must differ from DEFAULT_QUERY_LIMIT"
        );

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("red", 5),
            ("green", 7),
            ("green", 7),
            ("green", 7),
            ("blue", 2),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");

        let drive_config = DriveConfig {
            default_query_limit: OPERATOR_TUNED_LIMIT,
            ..Default::default()
        };

        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue.clone()],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: None,
            prove: true,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("dispatcher should succeed on distinct AVG path");
        let proof_bytes = match response {
            DocumentAverageResponse::Proof(p) => p,
            other => panic!("expected Proof response, got {:?}", other),
        };
        assert!(!proof_bytes.is_empty(), "non-empty proof bytes expected");

        // Reconstruct the path query the way the SDK verifier does
        // — anchored to DEFAULT_QUERY_LIMIT.
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&color_gt_blue),
            "amount",
        )
        .filter(|idx| idx.range_countable)
        .expect("byColor rangeAverageable index covers `color > blue`");
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: vec![color_gt_blue],
            sum_property: "amount".to_string(),
        };
        let verifier_path_query = sum_query
            .distinct_sum_path_query(Some(DEFAULT_QUERY_LIMIT), true, platform_version)
            .expect("path query builder accepts the same shape the prover used");

        // AVG distinct path's proof verifies via the same
        // `GroveDb::verify_query` shape sum uses — the difference is
        // the PCPS terminator the proof commits, and the SDK extracts
        // (count, sum) from each via `count_sum_value_or_default()`.
        // For this regression test we only need to confirm root-hash
        // recomputation succeeds against the DEFAULT_QUERY_LIMIT-anchored
        // path query; any limit mismatch surfaces as Err here.
        let (_root_hash, _elements) = GroveDb::verify_query(
            &proof_bytes,
            &verifier_path_query,
            &platform_version.drive.grove_version,
        )
        .expect(
            "expected proof to verify against a path query rebuilt with DEFAULT_QUERY_LIMIT; \
             a failure here means the dispatcher signed the AVG proof with the \
             operator-tunable default_query_limit — a consensus-adjacent silent-verify \
             regression",
        );
    }

    /// AVG `RangeDistinctProof` over-max rejection: explicit
    /// `limit > max_query_limit` MUST surface as `InvalidLimit`,
    /// not a silent clamp.
    #[test]
    fn range_distinct_avg_proof_rejects_limit_over_max() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        insert_widget(&drive, &data_contract, 0, "red", 5);

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();
        let over_max = drive_config.max_query_limit as u32 + 1;

        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: Some(over_max),
            prove: true,
            drive_config: &drive_config,
        };

        let err = drive
            .execute_document_average_request(request, None, platform_version)
            .expect_err("limit > max_query_limit must reject, not clamp");

        assert!(
            matches!(err, Error::Query(QuerySyntaxError::InvalidLimit(_))),
            "expected QuerySyntaxError::InvalidLimit, got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("exceeds max_query_limit"),
            "error must name the rejected limit; got: {msg}"
        );
    }
}
