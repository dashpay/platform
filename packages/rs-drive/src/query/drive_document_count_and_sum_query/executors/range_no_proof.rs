//! Joint count-and-sum `RangeNoProof` executor for
//! [`DocumentSumMode::RangeNoProof`] dispatch on the AVG no-prove path.
//!
//! Mirrors [`crate::query::drive_document_sum_query::executors::range_no_proof`]
//! + [`crate::query::drive_document_sum_query::execute_range_sum::execute_range_sum_no_proof`]
//! with one important structural difference vs. sum:
//!
//! ## The "flat summed" branch can't use the engine-side accumulator
//!
//! Sum's flat-summed branch collapses to a single
//! `grove.query_aggregate_sum` call against the merk-internal
//! `AggregateSumOnRange` primitive (O(log n) — the accumulator returns
//! a single `i64`). The pinned grovedb rev exposes that primitive AND
//! `query_aggregate_count`, but **does not** expose a combined no-prove
//! `query_aggregate_count_and_sum`. The proof-side analog
//! `AggregateCountAndSumOnRange` exists (and is what the AVG prove
//! path uses — see [`Drive::execute_document_average_prove`]'s range
//! arm) but the no-prove engine-side variant is a future grovedb
//! optimization tracked under #3687.
//!
//! As a result, the joint flat-summed branch walks PCPS terminator
//! elements via `grove_get_raw_path_query` against the same
//! `distinct_sum_path_query` builder the distinct branch uses, and
//! folds `(count, sum)` in Rust. This is still **one** grovedb walk
//! per AVG query — exactly the win #3687 captures — versus the
//! pre-#3687 path of issuing parallel `query_aggregate_sum` +
//! `query_aggregate_count` calls and zipping post-hoc. The
//! optimization opportunity (collapse the Rust-side fold into a single
//! engine-side accumulator) lives in grovedb, not here.
//!
//! ## Distinct branch
//!
//! For the distinct shapes (`GroupByRange` / `GroupByCompound` + range)
//! the walk is structurally identical to sum's: same
//! `distinct_sum_path_query` builder, same
//! `QueryPathKeyElementTrioResultType` shape, same `in_key` /
//! `base_path_len` heuristic. The only difference is that each emitted
//! element is decoded via `count_sum_value_or_default()` and produces
//! an [`AverageEntry { count: Some, sum: Some }`] rather than a
//! `SumEntry { sum: Some }`.

use super::super::super::drive_document_average_query::{AverageEntry, DocumentAverageResponse};
use super::super::super::drive_document_sum_query::index_picker::find_range_summable_index_for_where_clauses;
use super::super::super::drive_document_sum_query::{DriveDocumentSumQuery, RangeSumOptions};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

impl Drive {
    /// Range-aware joint count-and-sum walk against a
    /// `rangeSummable + rangeCountable` (PCPS-eligible) index.
    ///
    /// Returns either a one-pair [`DocumentAverageResponse::Aggregate`]
    /// (flat / compound aggregate shapes) or per-distinct-value
    /// [`DocumentAverageResponse::Entries`] (`GroupByRange` /
    /// `GroupByCompound`) depending on
    /// `options.return_distinct_sums_in_range`. The dispatcher sets that
    /// flag based on the request's [`AverageMode`].
    ///
    /// Perf: one grovedb walk per query in the flat / distinct branches
    /// — halving the work vs. the pre-#3687 composition.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_and_sum_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        options: RangeSumOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
        )
        .filter(|idx| idx.range_countable)
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "average range query requires an index that declares BOTH \
                 `rangeSummable: true` AND `rangeCountable: true` (a \
                 `rangeAverageable: true` index is the shorthand) whose last \
                 property matches the range field"
                    .to_string(),
            ))
        })?;

        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses: where_clauses.clone(),
            sum_property,
        };

        let drive_version = &platform_version.drive;
        let has_in_on_prefix = where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);

        // One unified walk for all three sub-shapes: the
        // `distinct_sum_path_query` builder is the right shape
        // regardless of whether we're folding into a single aggregate
        // pair (flat / compound-aggregate) or emitting per-distinct
        // entries — both cases visit the same PCPS terminator
        // elements; only the output shaping differs.
        //
        // For the compound-aggregate shape (In + range, non-distinct),
        // we still need atomicity across the multiple In-branch
        // sub-walks the distinct query naturally fans into — the
        // distinct path query's outer-keys walk under one grovedb call
        // already provides this since it's a single
        // `grove_get_raw_path_query` invocation against a multi-Key
        // outer query. No separate per-In fan-out required.
        let path_query = sum_query.distinct_sum_path_query(
            None::<u16>,
            options.left_to_right,
            platform_version,
        )?;
        let base_path_len = path_query.path.len();

        let mut drive_operations = vec![];
        let result = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut drive_operations,
            drive_version,
        );
        let elements = match result {
            Ok((elements, _)) => elements,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                return Ok(if options.return_distinct_sums_in_range {
                    DocumentAverageResponse::Entries(Vec::new())
                } else {
                    DocumentAverageResponse::Aggregate { count: 0, sum: 0 }
                });
            }
            Err(e) => return Err(e),
        };

        if !options.return_distinct_sums_in_range {
            // Flat / compound-aggregate: fold every visited PCPS
            // element's `(count, sum)` into a single pair. `checked_add`
            // on both axes mirrors the per-In-value and total executors.
            let mut count_acc: u64 = 0;
            let mut sum_acc: i64 = 0;
            for triple in elements.to_path_key_elements() {
                let (_path, _key, element) = triple;
                let (c, s) = element.count_sum_value_or_default();
                count_acc = count_acc.checked_add(c).ok_or_else(|| {
                    Error::Query(QuerySyntaxError::Unsupported(
                        "range count-and-sum overflowed u64 on the count axis while \
                         folding visited PCPS elements. Narrow the range or use \
                         multiple queries."
                            .to_string(),
                    ))
                })?;
                sum_acc = sum_acc.checked_add(s).ok_or_else(|| {
                    Error::Query(QuerySyntaxError::Unsupported(
                        "range count-and-sum overflowed i64 on the sum axis while \
                         folding visited PCPS elements. Narrow the range or use \
                         multiple queries."
                            .to_string(),
                    ))
                })?;
            }
            return Ok(DocumentAverageResponse::Aggregate {
                count: count_acc,
                sum: sum_acc,
            });
        }

        // Distinct (GroupByRange / GroupByCompound): emit one entry
        // per visited element. `(in_key, key)` shaping matches sum's
        // distinct branch.
        let mut entries: Vec<AverageEntry> = Vec::new();
        for triple in elements.to_path_key_elements() {
            let (path, key, element) = triple;
            let (count, sum) = element.count_sum_value_or_default();
            // Drop empty groups so the output matches sum's distinct
            // contract (which drops `sum == 0` rows). For AVG an empty
            // group has no averageable signal AND no count signal,
            // making the row uninformative.
            if count == 0 && sum == 0 {
                continue;
            }
            let in_key = if has_in_on_prefix && path.len() > base_path_len {
                Some(path[base_path_len].clone())
            } else {
                None
            };
            entries.push(AverageEntry {
                in_key,
                key,
                count: Some(count),
                sum: Some(sum),
            });
        }

        Ok(DocumentAverageResponse::Entries(entries))
    }
}
