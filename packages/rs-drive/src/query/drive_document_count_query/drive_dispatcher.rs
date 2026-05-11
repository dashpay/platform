//! Drive-level dispatcher for the unified `GetDocumentsCount` request.
//!
//! Two layers live here:
//!
//! 1. **Per-mode `impl Drive` executors** — `execute_document_count_*`
//!    methods that pick a covering index for their specific mode and
//!    run the matching `DriveDocumentCountQuery::*` executor. Each
//!    one collapses index-picking + executor invocation into a single
//!    call so the dispatcher's match arms stay one line per mode.
//!
//! 2. **Top-level `execute_document_count_request`** that owns the
//!    whole pipeline: mode detection → per-mode executor → response
//!    wrapping. The drive-abci handler just builds a
//!    [`DocumentCountRequest`] and calls this; everything past CBOR
//!    decode + contract lookup lives in drive.
//!
//! Both `DocumentCountRequest` and `DocumentCountResponse` are the
//! abi for this dispatcher; they're public so drive-abci can name
//! the input/output types without reaching into the executor surface.
//!
//! Whole module is gated `feature = "server"` via the parent's
//! `pub mod drive_dispatcher;` declaration.

use super::super::conditions::{WhereClause, WhereOperator};
use super::super::ordering::OrderClause;
use super::execute_range_count::RangeCountOptions;
use super::{DocumentCountMode, DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    //! Per-mode count-query executors. Each method:
    //!   1. Picks the right covering index for its mode (returns
    //!      `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`
    //!      if no index covers the where clauses).
    //!   2. Builds the appropriate `DriveDocumentCountQuery` /
    //!      `DriveDocumentQuery`.
    //!   3. Runs the right executor (`execute_no_proof`,
    //!      `execute_range_count_no_proof`,
    //!      `execute_aggregate_count_with_proof`, or
    //!      `execute_with_proof`).
    //!   4. Returns either `Vec<SplitCountEntry>` (no-proof modes)
    //!      or `Vec<u8>` proof bytes (proof modes).
    //!
    //! Each per-mode executor is its own narrow contract — splitting
    //! along mode boundaries keeps the dispatcher arms one line each
    //! and lets each executor's index-picking + clause-handling logic
    //! stay close to the executor it feeds.

    /// Total count for the given where clauses against the best
    /// covering countable index. Single summed entry with empty key.
    /// Used by [`DocumentCountMode::Total`] dispatch.
    pub fn execute_document_count_total_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
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
            document_type_name,
            index,
            where_clauses,
        };
        count_query.execute_no_proof(self, transaction, platform_version)
    }

    /// Per-`In`-value entries: cartesian-fork the single `In` clause
    /// into one Equal-on-each-value sub-query, run each, emit a
    /// `(serialized_value, count)` entry. Used by
    /// [`DocumentCountMode::PerInValue`] dispatch.
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
            let count = results.first().map_or(0, |entry| entry.count);
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
                count,
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
    /// `options.distinct`. Used by [`DocumentCountMode::RangeNoProof`]
    /// dispatch.
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
    /// [`DocumentCountMode::RangeProof`] dispatch.
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
    /// Used by [`DocumentCountMode::RangeDistinctProof`] dispatch.
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
    /// `prove = true` Equal/`In` count queries. Returns proof bytes of
    /// the CountTree elements covering the requested branches — the
    /// SDK-side verifier extracts each branch's `count_value` directly,
    /// no document materialization.
    ///
    /// Requires a covering countable index, mirroring the no-proof
    /// `Total` / `PerInValue` modes: if no `countable: true` index
    /// covers the where clauses, rejects with
    /// `WhereClauseOnNonIndexedProperty`. Same contract on both prove
    /// and no-proof paths — no silent fallback.
    ///
    /// Used by [`DocumentCountMode::PointLookupProof`] dispatch.
    pub fn execute_document_count_point_lookup_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "prove count requires a `countable: true` index on the \
                 document type that matches the where clause properties — \
                 same requirement as the no-proof path"
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

/// All inputs required for the unified document-count entry point
/// [`Drive::execute_document_count_request`]. Built by the gRPC
/// handler from a `GetDocumentsCountRequestV0` after CBOR-decoding +
/// contract lookup; drive owns everything past this point including
/// mode detection, index picking, and per-mode dispatch.
///
/// Both `raw_where_value` and parsed `Vec<WhereClause>` (built
/// internally by the dispatcher) are needed because
/// `DriveDocumentQuery::from_decomposed_values` (used by the
/// materialize-and-count fallback for `prove=true` point lookups)
/// takes the raw `Value` while every other path consumes the parsed
/// clauses. Same dual-shape applies to `raw_order_by_value`.
pub struct DocumentCountRequest<'a> {
    /// Live contract (already loaded by the handler).
    pub contract: &'a dpp::data_contract::DataContract,
    /// Resolved document type within `contract`.
    pub document_type: DocumentTypeRef<'a>,
    /// Decoded `where` value as it came off the wire (after CBOR
    /// decode). The dispatcher parses this into `Vec<WhereClause>`
    /// internally for mode detection + per-mode executors that
    /// consume structured clauses, and forwards the raw value as-is
    /// to the materialize-and-count fallback (`PointLookupProof`)
    /// which uses `DriveDocumentQuery::from_decomposed_values`.
    ///
    /// Mirrors how the regular `query_documents_v0` handler delegates
    /// where-clause decomposition to drive: the abci layer just CBOR-
    /// decodes and hands the raw value down.
    pub raw_where_value: dpp::platform_value::Value,
    /// Decoded `order_by` value as it came off the wire. Same dual-
    /// purpose role as `raw_where_value`: parsed into structured
    /// `OrderClause`s for split-mode entry direction (per-`In`-value /
    /// per-distinct-value-in-range / per-distinct-prove), and
    /// forwarded raw to `DriveDocumentQuery::from_decomposed_values`
    /// for the `PointLookupProof` walk-order requirement.
    ///
    /// `Value::Null` (empty `order_by` field on the wire) → no
    /// clauses. The dispatcher synthesizes a default direction of
    /// "ascending" for split-mode response ordering when no clauses
    /// are present; the materialize path rejects empty `order_by`
    /// when the where clause has an `In`/range operator (proof
    /// determinism requires an explicit walk order).
    pub raw_order_by_value: dpp::platform_value::Value,
    /// `return_distinct_counts_in_range` flag from the request.
    pub return_distinct_counts_in_range: bool,
    /// Limit cap from the request. Callers SHOULD pre-clamp against
    /// their server-side `max_query_limit` policy, but Drive also
    /// enforces a defense-in-depth clamp before forwarding to the
    /// distinct-mode walk: an `Option::None` here is normalized to
    /// `drive_config.default_query_limit` and any `Some(value)` is
    /// reduced to `drive_config.max_query_limit` if larger. After
    /// dispatch, the limit forwarded to
    /// [`RangeCountOptions::limit`] is always `Some(_)` ≤ system cap.
    pub limit: Option<u32>,
    /// Whether to produce a proof (vs. raw counts).
    pub prove: bool,
    /// Drive-side query config — only consumed by the materialize-and-
    /// count fallback.
    pub drive_config: &'a crate::config::DriveConfig,
}

/// Output shape of [`Drive::execute_document_count_request`]. Three
/// variants mirror the proto's `CountResults.variant` oneof (for
/// no-proof responses) plus the outer `Proof` arm:
///
/// - `Aggregate(u64)` — total-count modes (`Total` and
///   `RangeNoProof` with `return_distinct_counts_in_range = false`).
///   The abci handler maps this to `CountResults.aggregate_count`.
/// - `Entries(Vec<SplitCountEntry>)` — per-key modes (`PerInValue`
///   and `RangeNoProof` with `return_distinct_counts_in_range =
///   true`). The abci handler maps this to `CountResults.entries`.
/// - `Proof(Vec<u8>)` — grovedb proof bytes the client verifies via
///   either `verify_aggregate_count_query` (for `RangeProof`),
///   `verify_distinct_count_proof` (for `RangeDistinctProof`), or
///   the `DriveDocumentQuery` proof verifier (for
///   `PointLookupProof`).
#[derive(Debug, Clone)]
pub enum DocumentCountResponse {
    /// Single aggregate count — total across the matching set.
    Aggregate(u64),
    /// Per-key entries.
    Entries(Vec<SplitCountEntry>),
    /// Grovedb proof bytes.
    Proof(Vec<u8>),
}

/// Parse the decoded `where` value into structured [`WhereClause`]s.
///
/// Mirrors the per-clause loop the regular `query_documents_v0`
/// handler delegates to `DriveDocumentQuery::from_decomposed_values`:
/// the abci layer just CBOR-decodes the wire bytes into a `Value` and
/// hands the raw value down. Drive owns the parsing so a future
/// per-clause validation (e.g. forbidding operators in distinct mode)
/// can live next to the executors instead of being scattered across
/// abci handlers.
///
/// `Value::Null` (empty `where` field) → no clauses. Any other shape
/// must be an outer array of inner arrays-of-components.
fn where_clauses_from_value(value: &dpp::platform_value::Value) -> Result<Vec<WhereClause>, Error> {
    match value {
        dpp::platform_value::Value::Null => Ok(Vec::new()),
        dpp::platform_value::Value::Array(clauses) => clauses
            .iter()
            .map(|wc| match wc {
                dpp::platform_value::Value::Array(components) => {
                    WhereClause::from_components(components)
                }
                _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                    "where clause must be an array",
                ))),
            })
            .collect(),
        _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
            "where clause must be an array",
        ))),
    }
}

/// Parse the decoded `order_by` value into structured [`OrderClause`]s.
///
/// Same shape as [`where_clauses_from_value`] for `order_by`:
/// `Value::Null` (empty `order_by` field on the wire) → no clauses;
/// any other shape must be an outer array of `[field, direction]`
/// inner arrays. Direction is `"asc"` / `"desc"` per
/// `OrderClause::from_components`.
fn order_clauses_from_value(value: &dpp::platform_value::Value) -> Result<Vec<OrderClause>, Error> {
    match value {
        dpp::platform_value::Value::Null => Ok(Vec::new()),
        dpp::platform_value::Value::Array(clauses) => clauses
            .iter()
            .map(|oc| match oc {
                dpp::platform_value::Value::Array(components) => {
                    // `OrderClause::from_components` returns
                    // `grovedb::Error`; wrap as drive's query-syntax
                    // error so the dispatcher's error contract stays
                    // uniform with the where-clause parser above.
                    OrderClause::from_components(components).map_err(|_e| {
                        Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                            "order_by clause must have [field, \"asc\"|\"desc\"] shape",
                        ))
                    })
                }
                _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                    "order_by clause must be an array",
                ))),
            })
            .collect(),
        _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
            "order_by clause must be an array",
        ))),
    }
}

impl Drive {
    /// Single entry point for the unified `GetDocumentsCount` request.
    ///
    /// Owns the whole pipeline:
    /// 1. [`DriveDocumentCountQuery::detect_mode`] classifies the
    ///    query shape from the where clauses + flags.
    /// 2. The matching `Drive::execute_document_count_*` per-mode
    ///    method picks an index and runs the executor.
    /// 3. The result is wrapped in [`DocumentCountResponse`] —
    ///    `Counts(...)` for no-proof modes, `Proof(...)` for proof
    ///    modes.
    ///
    /// Errors:
    /// - Mode-detection failures (multiple range clauses, range +
    ///   `In`, distinct on prove path, …) come back as
    ///   `Error::Query(QuerySyntaxError::InvalidWhereClauseComponents)`.
    /// - "No covering index" failures come back as
    ///   `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`.
    /// - All other failures (grovedb, cost calculation, …) surface
    ///   as their native `Error` variants.
    ///
    /// The handler maps both `Error::Query(...)` cases to its own
    /// `QueryError::Query(...)` variant uniformly.
    pub fn execute_document_count_request(
        &self,
        request: DocumentCountRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentCountResponse, Error> {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;

        // Parse where clauses out of the raw decoded `Value` once,
        // then thread them through the per-mode executors. Mirrors
        // how the regular `query_documents_v0` handler delegates this
        // to `DriveDocumentQuery::from_decomposed_values` —
        // where-clause decomposition is a drive concern, not abci's.
        let where_clauses = where_clauses_from_value(&request.raw_where_value)?;
        let order_clauses = order_clauses_from_value(&request.raw_order_by_value)?;

        // Split-mode entry direction is whatever the first orderBy
        // clause specifies. Empty orderBy → ascending default. The
        // raw `order_by` value is also threaded through to the
        // materialize path (`PointLookupProof`) for proof-walk
        // determinism — see the executor.
        let order_by_ascending = order_clauses.first().map(|c| c.ascending).unwrap_or(true);

        let mode = DriveDocumentCountQuery::detect_mode(
            &where_clauses,
            request.return_distinct_counts_in_range,
            request.prove,
        )?;

        let contract_id = request.contract.id_ref().to_buffer();
        let document_type_name = request.document_type.name().to_string();

        match mode {
            DocumentCountMode::Total => {
                // Total mode → single aggregate. The executor returns
                // at most one entry (with empty key); collapse to
                // `Aggregate(count)` here so the response is a u64
                // with no per-key wrapping. Empty result (indexed
                // path doesn't exist yet) → `Aggregate(0)`.
                let entries = self.execute_document_count_total_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    transaction,
                    platform_version,
                )?;
                let total = entries.first().map(|e| e.count).unwrap_or(0);
                Ok(DocumentCountResponse::Aggregate(total))
            }
            DocumentCountMode::PerInValue => {
                // Per-`In`-value → entries. The proto contract on
                // `GetDocumentsCountRequestV0.{order_by, limit}`
                // applies; clamp `limit` defensively (the abci handler
                // passes raw, see `DocumentCountRequest::limit` doc).
                let effective_limit = request
                    .limit
                    .unwrap_or(request.drive_config.default_query_limit as u32)
                    .min(request.drive_config.max_query_limit as u32);
                let options = RangeCountOptions {
                    distinct: false, // ignored by PerInValue executor
                    limit: Some(effective_limit),
                    order_by_ascending,
                };
                Ok(DocumentCountResponse::Entries(
                    self.execute_document_count_per_in_value_no_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        options,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentCountMode::RangeNoProof => {
                // Range no-proof → either aggregate (sum) or entries
                // (per-distinct-value), based on
                // `return_distinct_counts_in_range`. Clamp limit
                // defense-in-depth.
                let effective_limit = request
                    .limit
                    .unwrap_or(request.drive_config.default_query_limit as u32)
                    .min(request.drive_config.max_query_limit as u32);
                let options = RangeCountOptions {
                    distinct: request.return_distinct_counts_in_range,
                    limit: Some(effective_limit),
                    order_by_ascending,
                };
                let entries = self.execute_document_count_range_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    options,
                    transaction,
                    platform_version,
                )?;
                if request.return_distinct_counts_in_range {
                    Ok(DocumentCountResponse::Entries(entries))
                } else {
                    // !distinct: executor returns a single empty-key
                    // entry containing the sum (or empty vec if the
                    // path doesn't exist). Collapse to `Aggregate`.
                    let total = entries.first().map(|e| e.count).unwrap_or(0);
                    Ok(DocumentCountResponse::Aggregate(total))
                }
            }
            DocumentCountMode::RangeProof => Ok(DocumentCountResponse::Proof(
                self.execute_document_count_range_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentCountMode::RangeDistinctProof => {
                // Validate-don't-clamp limit policy on the prove
                // path: client-side proof reconstruction needs the
                // exact same limit value the server applied to the
                // path query (so the merk-root recomputation
                // matches). Silent clamping would invisibly break
                // verification on any request with `limit >
                // max_query_limit`. Default to `default_query_limit`
                // when `None` (the SDK and server share the same
                // `DEFAULT_QUERY_LIMIT` constant in
                // `drive::config`).
                let effective_limit = request
                    .limit
                    .unwrap_or(request.drive_config.default_query_limit as u32);
                if effective_limit > request.drive_config.max_query_limit as u32 {
                    return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "limit {} exceeds max_query_limit {} on the prove + \
                         return_distinct_counts_in_range path; reduce the requested \
                         limit or use prove = false",
                        effective_limit, request.drive_config.max_query_limit
                    ))));
                }
                let limit_u16 = effective_limit as u16;
                // Default to ascending if the request didn't specify
                // — matches the no-proof default. The verifier reads
                // the same field to reconstruct the matching path
                // query (see SDK's
                // `FromProof<DocumentCountQuery>` for
                // `DocumentSplitCounts`); both sides MUST land on the
                // same `left_to_right` value or the merk-root
                // recomputation fails.
                let left_to_right = order_by_ascending;
                Ok(DocumentCountResponse::Proof(
                    self.execute_document_count_range_distinct_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        limit_u16,
                        left_to_right,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentCountMode::PointLookupProof => Ok(DocumentCountResponse::Proof(
                self.execute_document_count_point_lookup_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    transaction,
                    platform_version,
                )?,
            )),
        }
    }
}
