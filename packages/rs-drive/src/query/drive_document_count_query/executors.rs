//! Per-mode count-query executors on `impl Drive`. Each method:
//!
//! 1. Picks the right covering index for its mode (returns
//!    `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`
//!    if no index covers the where clauses).
//! 2. Builds the appropriate `DriveDocumentCountQuery` /
//!    `DriveDocumentQuery`.
//! 3. Runs the right executor (`execute_no_proof`,
//!    `execute_range_count_no_proof`,
//!    `execute_aggregate_count_with_proof`, or
//!    `execute_with_proof`).
//! 4. Returns either `Vec<SplitCountEntry>` (no-proof modes) or
//!    `Vec<u8>` proof bytes (proof modes).
//!
//! Each per-mode executor is its own narrow contract. Splitting
//! along mode boundaries keeps the dispatcher arms in
//! [`super::drive_dispatcher`] one line each and lets each
//! executor's index-picking + clause-handling logic stay close to
//! the executor it feeds.
//!
//! Module is gated `feature = "server"` via the parent's
//! `pub mod executors;` declaration.

use super::super::conditions::{WhereClause, WhereOperator};
use super::execute_range_count::RangeCountOptions;
use super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Total count for the given where clauses against an exactly-
    /// covering countable index, OR — when the where clauses are
    /// empty and the document type has `documents_countable: true` —
    /// the type's primary-key CountTree (O(1) read at the doctype
    /// tree's root).
    ///
    /// Single summed entry with empty key. Used by
    /// [`super::DocumentCountMode::Total`] dispatch.
    pub fn execute_document_count_total_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        use dpp::data_contract::document_type::accessors::{
            DocumentTypeV0Getters, DocumentTypeV2Getters,
        };

        // Fast path: unfiltered total count on a `documents_countable:
        // true` document type reads the primary-key CountTree directly
        // (O(1)). No index needed — the doctype tree itself carries
        // the count.
        if where_clauses.is_empty() && document_type.documents_countable() {
            let count = self.read_primary_key_count_tree(
                &contract_id,
                &document_type_name,
                transaction,
                platform_version,
            )?;
            return Ok(vec![SplitCountEntry {
                in_key: None,
                key: vec![],
                // `documents_countable` fast path: we read the
                // CountTree directly and got an explicit count, so
                // this is a verified `Some(_)` (possibly `Some(0)`
                // for an empty doctype).
                count: Some(count),
            }]);
        }

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "count query requires a `countable: true` index whose properties \
                     exactly match the where clause fields, or `documentsCountable: \
                     true` on the document type for unfiltered total counts"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        count_query.execute_no_proof(self, transaction, platform_version)
    }

    /// Reads the document-type primary-key tree's `CountTree` element
    /// (`[contract_doc, contract_id, [1], doctype, 0]`) and returns
    /// `count_value_or_default()`. Used by the `documents_countable:
    /// true` fast path on the total-count flows (both no-proof and
    /// prove builder).
    ///
    /// Returns 0 when the element doesn't exist (e.g. fresh contract
    /// with no documents inserted). Caller is responsible for ensuring
    /// `documents_countable` is set on the document type before
    /// calling — without it the element at `[..., doctype, 0]` is a
    /// regular `NormalTree` and `count_value_or_default()` returns 0
    /// regardless of how many documents the type actually has.
    fn read_primary_key_count_tree(
        &self,
        contract_id: &[u8; 32],
        document_type_name: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let drive_version = &platform_version.drive;
        let path = [
            &[crate::drive::RootTree::DataContractDocuments as u8] as &[u8],
            contract_id,
            &[1u8],
            document_type_name.as_bytes(),
        ];
        let mut drive_operations = vec![];
        let element = self.grove_get_raw_optional(
            grovedb_path::SubtreePath::from(path.as_slice()),
            &[0],
            crate::util::grove_operations::DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;
        Ok(element.map_or(0, |e| e.count_value_or_default()))
    }

    /// Per-`In`-value entries: cartesian-fork the single `In` clause
    /// into one Equal-on-each-value sub-query, run each, emit a
    /// `(serialized_value, count)` entry. Used by
    /// [`super::DocumentCountMode::PerInValue`] dispatch.
    ///
    /// `options` (limit / order / distinct) applies to the returned
    /// entry list — split-mode pagination per the proto contract on
    /// `GetDocumentsCountRequestV0.{order_by, limit}` (the dispatcher
    /// derives `RangeCountOptions.order_by_ascending` from the first
    /// `order_by` clause's direction; empty `order_by` → ascending).
    /// The `distinct` flag has no effect here (PerInValue is always
    /// per-value); it's accepted for symmetry with the range-mode
    /// executor.
    ///
    /// Caller has already verified via [`DriveDocumentCountQuery::detect_mode`]
    /// that exactly one `In` clause is present in `where_clauses`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_per_in_value_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
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

    /// Range-count walk against a `range_countable` index. Returns a
    /// summed entry or per-distinct-value entries depending on
    /// `options.distinct`. Used by
    /// [`super::DocumentCountMode::RangeNoProof`] dispatch.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        options: RangeCountOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range count requires a `range_countable: true` index whose last \
                     property matches the range field, with all other clauses covering \
                     its prefix as `==` matches"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        count_query.execute_range_count_no_proof(self, &options, transaction, platform_version)
    }

    /// Range-count proof via grovedb's `AggregateCountOnRange`. Returns
    /// proof bytes that the client verifies via
    /// `GroveDb::verify_aggregate_count_query`. Used by
    /// [`super::DocumentCountMode::RangeProof`] dispatch.
    pub fn execute_document_count_range_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range count requires a `range_countable: true` index whose last \
                     property matches the range field"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        count_query.execute_aggregate_count_with_proof(self, transaction, platform_version)
    }

    /// Distinct-counts-with-proof companion to
    /// [`Self::execute_document_count_range_proof`]. Returns proof
    /// bytes that the client verifies via
    /// [`drive_proof_verifier::verify_distinct_count_proof`], yielding
    /// a `BTreeMap<Vec<u8>, u64>` keyed by serialized property value.
    /// Used by [`super::DocumentCountMode::RangeDistinctProof`]
    /// dispatch.
    ///
    /// `limit` caps the number of distinct in-range values the proof
    /// covers — the dispatcher pre-validates `limit ≤ max_query_limit`
    /// so client-side proof reconstruction can use the exact same
    /// value without divergence. The SDK reads it back off the
    /// request when building the verifier's `PathQuery`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_range_distinct_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        limit: u16,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range count requires a `range_countable: true` index whose last \
                     property matches the range field"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        count_query.execute_distinct_count_with_proof(
            self,
            limit,
            left_to_right,
            transaction,
            platform_version,
        )
    }

    /// Point-lookup count proof against a `countable: true` index for
    /// `prove = true` Equal/`In` count queries, OR — when the where
    /// clauses are empty and the document type has
    /// `documents_countable: true` — a proof of the type's primary-key
    /// CountTree (one merk path proof, O(log n) bytes).
    ///
    /// In both cases the SDK-side verifier extracts each verified
    /// CountTree element's `count_value` directly, no document
    /// materialization.
    ///
    /// Mirrors the no-proof `Total` / `PerInValue` modes' rejection
    /// contract: if no `countable: true` index exactly covers the
    /// where clauses (and the documents_countable fast path doesn't
    /// apply), rejects with `WhereClauseOnNonIndexedProperty`. Same
    /// contract on both prove and no-proof paths — no silent fallback.
    ///
    /// Used by [`super::DocumentCountMode::PointLookupProof`] dispatch.
    pub fn execute_document_count_point_lookup_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;

        // Fast path: unfiltered prove count on a `documents_countable:
        // true` document type proves the primary-key CountTree
        // element directly. Same path-query shape as the index-based
        // case, just rooted at `[..., doctype]` instead of inside an
        // index.
        if where_clauses.is_empty() && document_type.documents_countable() {
            let path_query = DriveDocumentCountQuery::primary_key_count_tree_path_query(
                contract_id,
                &document_type_name,
            );
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
            return Ok(proof);
        }

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "prove count requires a `countable: true` index whose properties \
                 exactly match the where clause fields, or `documentsCountable: \
                 true` on the document type for unfiltered total counts — same \
                 requirement as the no-proof path"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        count_query.execute_point_lookup_count_with_proof(self, transaction, platform_version)
    }
}
