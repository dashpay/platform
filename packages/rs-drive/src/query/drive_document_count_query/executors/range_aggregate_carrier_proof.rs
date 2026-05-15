//! Carrier-ACOR proof executor for
//! [`super::super::DocumentCountMode::RangeAggregateCarrierProof`]
//! dispatch — `prove = true` count queries with both an `In`
//! clause and a range clause, where the caller asks for one
//! aggregate per In branch via `group_by = [in_field]`.
//!
//! Uses grovedb's carrier-subquery composition (introduced in
//! [PR #663](https://github.com/dashpay/grovedb/pull/663)): one
//! outer Key per In value, each terminating in an
//! `AggregateCountOnRange` subquery over the per-branch range
//! subtree. Returns proof bytes that the client verifies via
//! [`grovedb::GroveDb::verify_aggregate_count_query_per_key`],
//! producing `Vec<(in_key, u64)>` — same per-key aggregate
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
    /// Carrier-ACOR proof for `In + range` with
    /// `group_by = [in_field]`. Returns proof bytes that the
    /// client verifies via
    /// [`grovedb::GroveDb::verify_aggregate_count_query_per_key`].
    pub fn execute_document_count_range_aggregate_carrier_proof(
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
                "carrier-aggregate count requires a `range_countable: true` index whose first \
                 property matches the In field and last property matches the range field"
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
        count_query.execute_carrier_aggregate_count_with_proof(self, transaction, platform_version)
    }
}
