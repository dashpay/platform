//! [`DocumentHavingRequest`] / [`DocumentHavingResponse`] and the
//! having-range dispatcher on `impl Drive` — the ABI drive-abci's
//! routing layer names.

use super::super::drive_document_ranked_query::{RankedEntry, RankedPaginationInputs};
use super::mode_detection::detect_having_mode;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::HavingClause;
use crate::query::projection::SelectProjection;
use crate::query::{OrderClause, ResolvedTimeRange, WhereClause};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// All inputs required by [`Drive::execute_document_having_request`].
/// Built by the gRPC handler from a `GetDocumentsRequestV1` after
/// wire-decoding + contract lookup — the same construction pattern as
/// [`super::super::drive_document_ranked_query::DocumentRankedRequest`].
///
/// `offset` and `start_at` are carried even though a having-range
/// request must leave both empty: drive owns the rejection, so the
/// contract is enforced identically no matter which upstream path built
/// the request. See [`super::mode_detection::detect_having_mode_v0`]
/// for why each is refused rather than ignored.
pub struct DocumentHavingRequest<'a> {
    /// Live contract (already loaded by the handler).
    pub contract: &'a DataContract,
    /// Resolved document type within `contract`.
    pub document_type: DocumentTypeRef<'a>,
    /// The single `GROUP BY` property. Must be the ranked index's only
    /// property.
    pub group_by: &'a [String],
    /// The projection whose aggregate the `having` clause bounds:
    /// `COUNT(*)`, `SUM(field)` or `AVG(field)`.
    pub select: SelectProjection,
    /// The `HAVING` clauses. Exactly one, bounding the selected
    /// aggregate.
    pub having: &'a [HavingClause],
    /// The `ORDER BY` clauses. Empty (ascending default) or exactly
    /// one, naming the selected aggregate.
    pub order_by: &'a [OrderClause],
    /// Structured `where` clauses. Empty for the single-property form;
    /// pins on the covering compound index's leading properties for
    /// the pinned-prefix form: one equality pin per property, of which
    /// at most one may instead be a bounded `IN` (one branch per
    /// element, merged; entries then carry `in_key`).
    pub where_clauses: &'a [WhereClause],
    /// The fields among `where_clauses` whose equality clause was produced by
    /// `IN_TIME_RANGE` resolution (see
    /// [`crate::query::DriveDocumentQuery::resolved_time_ranges`]).
    /// Must be empty: HAVING's equality prefixes pin plain ranked indexes
    /// (its picker excludes transformed ones), so a resolved bucket-start
    /// equality would authenticate raw-timestamp matches at the bucket
    /// boundary instead of window membership. Carried (and rejected) here
    /// for the same reason the ranked request carries it: drive owns the
    /// rejection regardless of which upstream path built the request.
    pub resolved_time_ranges: &'a [ResolvedTimeRange],
    /// Request `limit`. **Required**; `1 ..= MAX_HAVING_LIMIT`.
    pub limit: Option<u32>,
    /// Request `offset`. Must be `None` — the range walk has no skip.
    pub offset: Option<u32>,
    /// Whether the request carried a `start_at` / `start_after` cursor.
    /// Must be `false`.
    pub has_start_at: bool,
    /// Whether to produce a proof instead of materializing entries.
    pub prove: bool,
}

/// Output shape of [`Drive::execute_document_having_request`].
///
/// - `Entries` — the matching groups **in axis order in the walk
///   direction**; the abci handler maps this straight onto the wire's
///   ranked-entries shape (with no rank base) without re-sorting.
/// - `Proof(Vec<u8>)` — grovedb indexed-axis range proof bytes the
///   client verifies with
///   [`DriveDocumentHavingQuery::verify_having_range_proof`](crate::query::DriveDocumentHavingQuery::verify_having_range_proof),
///   which recovers the same entry list.
#[derive(Debug, Clone)]
pub enum DocumentHavingResponse {
    /// The groups whose aggregate falls inside the bound, cut at the
    /// request's limit.
    Entries(Vec<RankedEntry>),
    /// Grovedb indexed-axis range proof bytes.
    Proof(Vec<u8>),
}

impl Drive {
    /// Single entry point for a having-range document request.
    ///
    /// 1. [`detect_having_mode`] validates the request shape and
    ///    resolves the `(bounds, descending, limit, group property,
    ///    aggregate field)` tuple.
    /// 2. The matching executor picks the covering ranked index and runs
    ///    the read or the proof.
    /// 3. The result is wrapped in [`DocumentHavingResponse`].
    ///
    /// Errors:
    /// - Request-shape failures (wrong `group_by` arity, a clause on an
    ///   aggregate the select does not project, an untranslatable
    ///   operator, a missing or out-of-range `limit`, a `where`, an
    ///   `offset`) come back as `Error::Query(QuerySyntaxError::*)` —
    ///   see [`super::mode_detection::detect_having_mode_v0`] for the
    ///   full grammar.
    /// - "No index declares this axis" comes back as
    ///   `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`
    ///   naming the missing contract keyword.
    /// - Everything else (grovedb, versioning) surfaces as its native
    ///   `Error` variant.
    pub fn execute_document_having_request(
        &self,
        request: DocumentHavingRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentHavingResponse, Error> {
        // Before mode detection reads `where_clauses`: a resolved bucket
        // equality is indistinguishable from a hand-written equality pin, and
        // the pinned-prefix form would accept it against a plain ranked index
        // over raw timestamps — a validly-proven answer to a different
        // question. See `DocumentHavingRequest::resolved_time_ranges`.
        if !request.resolved_time_ranges.is_empty() {
            return Err(Error::Query(QuerySyntaxError::Unsupported(
                "a HAVING query cannot carry a time-range (IN_TIME_RANGE) selection: its \
                 equality prefixes pin plain ranked indexes, so a resolved bucket-start \
                 equality would match raw timestamps at the bucket boundary instead of the \
                 selected window"
                    .to_string(),
            )));
        }
        let mode = detect_having_mode(
            &request.select,
            request.group_by,
            request.having,
            request.order_by,
            request.where_clauses,
            RankedPaginationInputs {
                limit: request.limit,
                offset: request.offset,
                has_start_at: request.has_start_at,
            },
            platform_version,
        )?;

        let contract_id = request.contract.id_ref().to_buffer();
        let document_type_name = request.document_type.name().to_string();

        if request.prove {
            Ok(DocumentHavingResponse::Proof(
                self.execute_document_having_range_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    &mode,
                    transaction,
                    platform_version,
                )?,
            ))
        } else {
            Ok(DocumentHavingResponse::Entries(
                self.execute_document_having_range_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    &mode,
                    transaction,
                    platform_version,
                )?,
            ))
        }
    }
}
