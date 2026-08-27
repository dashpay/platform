//! Per-`In`-value executor for
//! [`super::super::DocumentCountMode::PerInValue`] dispatch —
//! `prove = false` count queries with exactly one `In` clause and
//! no range clause.

use super::super::super::conditions::{WhereClause, WhereOperator};
use super::super::execute_range_count::RangeCountOptions;
use super::super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::ResolvedTimeRange;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Per-`In`-value entries: cartesian-fork the single `In`
    /// clause into one Equal-on-each-value sub-query, run each,
    /// emit a `(serialized_value, count)` entry.
    ///
    /// `options` (limit / order / distinct) applies to the
    /// returned entry list — split-mode pagination per the proto
    /// contract on `GetDocumentsCountRequestV0.{order_by, limit}`
    /// (the dispatcher derives `RangeCountOptions.order_by_ascending`
    /// from the first `order_by` clause's direction; empty
    /// `order_by` → ascending). The `distinct` flag has no effect
    /// here (PerInValue is always per-value); it's accepted for
    /// symmetry with the range-mode executor.
    ///
    /// Caller has already verified via
    /// [`DriveDocumentCountQuery::detect_mode`] that exactly one
    /// `In` clause is present in `where_clauses`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_per_in_value_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_ranges: &[ResolvedTimeRange],
        options: RangeCountOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let in_clause = where_clauses
            .iter()
            .find(|wc| wc.operator == WhereOperator::In)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "execute_document_count_per_in_value_no_proof requires exactly one `in` clause",
                ))
            })?
            .clone();
        // `in_values()` enforces non-empty, ≤100, no-duplicates — the
        // same shape validation `WhereClause::from_clause` would have
        // applied on the regular query path. Without it the executor
        // below performs one GroveDB walk per value with no input cap,
        // which lets a single 64 MiB gRPC request schedule arbitrarily
        // many backend reads (request-amplification DoS). Inheriting
        // the existing 100-cap is the same defensive bound the other
        // `In` consumers (mod.rs:1246, conditions.rs:852) use.
        let in_values = in_clause.in_values().into_data_with_error()??;

        let other_clauses: Vec<WhereClause> = where_clauses
            .iter()
            .filter(|wc| wc.operator != WhereOperator::In)
            .cloned()
            .collect();

        // Aggregate first into a key-ordered map (dedupes duplicate
        // `In` values via the same canonical-byte rule as the range
        // walker uses; BTreeMap ordering matches `RangeCountOptions`'s
        // ascending convention). Order, cursor, and limit get applied
        // after.
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
        let mut merged: std::collections::BTreeMap<Vec<u8>, u64> =
            std::collections::BTreeMap::new();
        for value in in_values.iter() {
            let key_bytes = document_type.serialize_value_for_key(
                in_clause.field.as_str(),
                value,
                platform_version,
            )?;
            if merged.contains_key(&key_bytes) {
                // Duplicate `In` values resolve to the same indexed path,
                // so the count is the same — no need to re-query.
                continue;
            }

            let mut clauses_for_value = other_clauses.clone();
            clauses_for_value.push(WhereClause {
                field: in_clause.field.clone(),
                operator: WhereOperator::Equal,
                value: value.clone(),
            });

            let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                document_type.indexes(),
                &clauses_for_value,
                resolved_time_ranges,
            )
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "count query requires a countable index on the document type that \
                     matches the where clause properties"
                        .to_string(),
                ))
            })?;

            let count_query = DriveDocumentCountQuery {
                document_type,
                contract_id,
                document_type_name: document_type_name.clone(),
                index,
                where_clauses: clauses_for_value,
            };
            let results = count_query.execute_no_proof(self, transaction, platform_version)?;
            // Per-In fan-out: each sub-query returns one entry with
            // its branch count (or empty if the branch doesn't exist
            // in the index). Treat missing-entry as 0 here — the
            // no-proof path is enumerating known-In values and a
            // missing entry means "no docs at this value" which the
            // executor verified.
            let count = results.first().and_then(|entry| entry.count).unwrap_or(0);
            merged.insert(key_bytes, count);
        }

        // Apply order, then cursor, then limit — same shape as the
        // range walker. BTreeMap iteration is already ascending; flip
        // the vec if descending was requested.
        //
        // PerInValue mode splits by the `In` dimension itself, so
        // the In value goes in `key` (the split-key field) and
        // `in_key` is `None`. The `in_key` field is reserved for
        // compound queries where the `In` is on a prefix property
        // distinct from the value being counted.
        let mut entries: Vec<SplitCountEntry> = merged
            .into_iter()
            .map(|(key, count)| SplitCountEntry {
                in_key: None,
                key,
                // The no-proof per-In fan-out enumerates the caller's
                // In array and produces an explicit count per branch
                // (zero or otherwise) — always `Some(_)`.
                count: Some(count),
            })
            .collect();
        if !options.order_by_ascending {
            entries.reverse();
        }
        // For pagination, callers chunk the `In` array client-side
        // (the values are caller-supplied to begin with); no
        // server-side cursor is needed or supported.
        if let Some(limit) = options.limit {
            entries.truncate(limit as usize);
        }
        Ok(entries)
    }
}
