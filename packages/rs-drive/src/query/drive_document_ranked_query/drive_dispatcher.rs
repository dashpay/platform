//! Top-level dispatcher for the ranked (`HAVING … TOP(n)`) request.
//!
//! Owns the pipeline: validate → resolve the ranking → pick the covering
//! index → execute → wrap. The drive-abci handler builds a
//! [`DocumentRankedRequest`] and calls
//! [`Drive::execute_document_ranked_request`]; everything past contract
//! lookup lives here.
//!
//! Both [`DocumentRankedRequest`] and [`DocumentRankedResponse`] are the
//! ABI for this dispatcher — public so drive-abci can name the
//! input/output types without reaching into the executor surface.
//!
//! Module is gated `feature = "server"` via the parent's
//! `pub mod drive_dispatcher;` declaration.

use super::mode_detection::detect_ranked_mode;
use super::{RankedEntry, RankedPaginationInputs};
use crate::drive::Drive;
use crate::error::Error;
use crate::query::having::HavingClause;
use crate::query::projection::SelectProjection;
use crate::query::WhereClause;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// All inputs required by [`Drive::execute_document_ranked_request`].
/// Built by the gRPC handler from a `GetDocumentsRequestV1` after
/// wire-decoding + contract lookup.
///
/// The list-shaped inputs are borrowed slices rather than owned `Vec`s
/// because the dispatcher only ever reads them — nothing here is
/// canonicalized or rewritten the way count's where-clauses are, so
/// taking ownership would just force the handler into a clone.
///
/// `where_clauses`, `limit`, `offset` and `start_at` are carried even
/// though a ranked request must leave all of them empty: drive owns the
/// rejection, so the contract is enforced identically no matter which
/// upstream path built the request. See
/// [`super::mode_detection::detect_ranked_mode_v0`] for why each is
/// refused rather than ignored.
pub struct DocumentRankedRequest<'a> {
    /// Live contract (already loaded by the handler).
    pub contract: &'a DataContract,
    /// Resolved document type within `contract`.
    pub document_type: DocumentTypeRef<'a>,
    /// The single `GROUP BY` property. Must be the ranked index's only
    /// property.
    pub group_by: &'a [String],
    /// The projection being ranked: `COUNT(*)`, `SUM(field)` or
    /// `AVG(field)`.
    pub select: SelectProjection,
    /// The `HAVING` clauses. Exactly one, carrying a
    /// [`crate::query::HavingRanking`] right-operand whose aggregate
    /// matches `select`.
    pub having: &'a [HavingClause],
    /// Structured `where` clauses. Must be empty.
    pub where_clauses: &'a [WhereClause],
    /// Request `limit`. Must be unset — `n` comes from the ranking.
    pub limit: Option<u32>,
    /// Request `offset`. Must be unset.
    pub offset: Option<u32>,
    /// Whether the request carried a `start_at` / `start_after` cursor.
    /// Must be `false`.
    pub has_start_at: bool,
    /// Whether to produce a proof instead of materializing entries.
    pub prove: bool,
}

/// Output shape of [`Drive::execute_document_ranked_request`].
///
/// - `Entries(Vec<RankedEntry>)` — **in ranking order**; the abci
///   handler maps this straight onto the wire's repeated entry field
///   without re-sorting.
/// - `Proof(Vec<u8>)` — grovedb indexed-axis proof bytes the client
///   verifies with
///   [`DriveDocumentRankedQuery::verify_ranked_top_k_proof`](crate::query::DriveDocumentRankedQuery::verify_ranked_top_k_proof).
///
/// There is deliberately no `Aggregate` variant: even `MAX` / `MIN`
/// return a one-element entry list, because the caller needs the *group*
/// as much as the value ("which restaurant is best", not just "what the
/// best score is").
#[derive(Debug, Clone)]
pub enum DocumentRankedResponse {
    /// Ranked groups, best-first for `TOP` / `MAX` and worst-first for
    /// `BOTTOM` / `MIN`.
    Entries(Vec<RankedEntry>),
    /// Grovedb indexed-axis top-k proof bytes.
    Proof(Vec<u8>),
}

impl Drive {
    /// Single entry point for a ranked document request.
    ///
    /// 1. [`detect_ranked_mode`] validates the request shape and resolves
    ///    `(axis, descending, k, group property, aggregate field)`.
    /// 2. The matching executor picks the covering ranked index and runs
    ///    the read or the proof.
    /// 3. The result is wrapped in [`DocumentRankedResponse`].
    ///
    /// Errors:
    /// - Request-shape failures (wrong `group_by` arity, `having` that
    ///   does not match the `select`, `n` out of range, a `where` clause,
    ///   a `limit`) come back as `Error::Query(QuerySyntaxError::*)` —
    ///   see [`super::mode_detection::detect_ranked_mode_v0`] for the
    ///   full grammar.
    /// - "No index declares this ranking axis" comes back as
    ///   `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`
    ///   naming the missing contract keyword.
    /// - Everything else (grovedb, versioning) surfaces as its native
    ///   `Error` variant.
    pub fn execute_document_ranked_request(
        &self,
        request: DocumentRankedRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentRankedResponse, Error> {
        let mode = detect_ranked_mode(
            &request.select,
            request.group_by,
            request.having,
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
            Ok(DocumentRankedResponse::Proof(
                self.execute_document_ranked_top_k_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    &mode,
                    transaction,
                    platform_version,
                )?,
            ))
        } else {
            Ok(DocumentRankedResponse::Entries(
                self.execute_document_ranked_top_k_no_proof(
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
