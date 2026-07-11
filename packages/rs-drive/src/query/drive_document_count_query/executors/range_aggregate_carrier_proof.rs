//! Carrier-ACOR proof executor for
//! [`super::super::DocumentCountMode::RangeAggregateCarrierProof`]
//! dispatch — `prove = true` count queries where the caller asks
//! for one aggregate per outer-key branch via
//! `group_by = [outer_field]`. The outer dimension is either:
//!
//! - An `In` clause on the index's first property (G7 shape —
//!   `brand IN[...] AND color > floor`). One Key per In value.
//! - A range clause on the index's first property (G8 shape —
//!   `brand > X AND color > floor`, optional `limit`). Single
//!   QueryItem bounding the outer walk; `SizedQuery::limit` caps
//!   how many outer matches the carrier walks before stopping.
//!
//! Uses grovedb's carrier-subquery composition introduced in
//! [PR #663](https://github.com/dashpay/grovedb/pull/663) and
//! extended in [PR #664](https://github.com/dashpay/grovedb/pull/664)
//! (the latter relaxed the `SizedQuery::limit` rejection for
//! carriers, which is what unblocks the G8 outer-range shape with
//! a useful upper bound). Returns proof bytes that the client
//! verifies via
//! [`grovedb::GroveDb::verify_aggregate_count_query_per_key`],
//! producing `Vec<(outer_key, u64)>` — same per-key aggregate
//! semantics as the no-proof per-In fan-out, just verifiable.

use super::super::super::conditions::WhereClause;
use super::super::DriveDocumentCountQuery;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Carrier-ACOR proof for `(In OR outer Range) + inner range`
    /// with `group_by = [outer_field]`. Returns proof bytes that
    /// the client verifies via
    /// [`grovedb::GroveDb::verify_aggregate_count_query_per_key`].
    ///
    /// `limit` caps the outer walk for the Range-outer (G8) shape;
    /// for the In-outer (G7) shape the `|In|` array already bounds
    /// the result and `limit` is typically `None`.
    ///
    /// Sibling executors (`range_distinct_proof`, `range_no_proof`,
    /// `per_in_value`) use the same `#[allow]` here — the 8-arg
    /// boundary (contract id, doc type, doc type name, where clauses,
    /// limit, transaction, platform version) is the established
    /// executor signature; refactoring it into a struct would just
    /// move the same fields one indirection away.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_range_aggregate_carrier_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        limit: Option<u16>,
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
                "carrier-aggregate count requires a `range_countable: true` index whose first \
                 property carries the outer In or range clause and whose last property \
                 carries the inner ACOR range clause"
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
        count_query.execute_carrier_aggregate_count_with_proof(
            self,
            limit,
            left_to_right,
            transaction,
            platform_version,
        )
    }
}
