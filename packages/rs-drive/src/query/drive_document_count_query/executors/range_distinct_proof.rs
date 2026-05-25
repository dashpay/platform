//! Distinct-range-count proof executor for
//! [`super::super::DocumentCountMode::RangeDistinctProof`]
//! dispatch — `prove = true` count queries with a range clause
//! and non-empty `group_by`. Emits per-distinct-value `KVCount`
//! ops the client verifies via
//! [`drive_proof_verifier::verify_distinct_count_proof`].

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
    /// Distinct-counts-with-proof companion to
    /// [`Self::execute_document_count_range_proof`]. Returns
    /// proof bytes that the client verifies via
    /// [`drive_proof_verifier::verify_distinct_count_proof`],
    /// yielding a `BTreeMap<Vec<u8>, u64>` keyed by serialized
    /// property value.
    ///
    /// `limit` caps the number of distinct in-range values the
    /// proof covers — the dispatcher pre-validates
    /// `limit ≤ max_query_limit` so client-side proof
    /// reconstruction can use the exact same value without
    /// divergence. The SDK reads it back off the request when
    /// building the verifier's `PathQuery`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_range_distinct_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_range_fields: &[String],
        limit: u16,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            resolved_time_range_fields,
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
}
