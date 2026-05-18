//! Total-sum executor for [`super::super::DocumentSumMode::Total`]
//! dispatch — `prove = false` sum queries without a range clause.
//!
//! Mirror of count's `executors/total.rs` with the substitutions
//! documented in `executors/mod.rs` (count → sum, u64 → i64,
//! count_value_or_default → sum_value_or_default).

use super::super::index_picker::find_summable_index_for_where_clauses;
use super::super::{DriveDocumentSumQuery, SumEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::WhereClause;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Total sum for the given where clauses against an exactly-
    /// covering summable index, OR — when the where clauses are
    /// empty and the document type has `documents_summable: Some(_)`
    /// — the type's primary-key SumTree (O(1) read at the doctype
    /// tree's root).
    ///
    /// Single summed entry with empty key.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_sum_total_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SumEntry>, Error> {
        use dpp::data_contract::document_type::accessors::{
            DocumentTypeV0Getters, DocumentTypeV2Getters,
        };

        // Fast path: unfiltered total sum on a `documents_summable:
        // Some(matching_property)` doctype reads the primary-key
        // SumTree directly (O(1)). No index needed — the doctype tree
        // itself carries the sum.
        if where_clauses.is_empty()
            && document_type
                .documents_summable()
                .map(|p| p == sum_property)
                .unwrap_or(false)
        {
            let sum = self.read_primary_key_sum_tree(
                &contract_id,
                &document_type_name,
                transaction,
                platform_version,
            )?;
            return Ok(vec![SumEntry {
                in_key: None,
                key: vec![],
                sum: Some(sum),
            }]);
        }

        let index = find_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "sum query requires a `summable: \"<prop>\"` index whose properties \
                 exactly match the where clause fields and whose summed property \
                 matches the request's `sum_property`, or `documentsSummable: \
                 \"<prop>\"` on the document type for unfiltered total sums"
                    .to_string(),
            ))
        })?;
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            sum_property,
        };
        sum_query.execute_no_proof(self, transaction, platform_version)
    }

    /// Reads the document-type primary-key tree's `SumTree` element
    /// (`[contract_doc, contract_id, [1], doctype, 0]`) and returns
    /// `sum_value_or_default()`. Used by the `documents_summable:
    /// Some(_)` fast path on the total-sum flow.
    ///
    /// `insert_contract_operations_v0` unconditionally creates a
    /// sum-bearing tree at `[..., doctype, 0]` for every applied
    /// document type whose `documents_summable` is set, so a missing
    /// element here indicates contract-state corruption or a
    /// mis-applied contract — fail fast rather than silently
    /// returning 0.
    pub(super) fn read_primary_key_sum_tree(
        &self,
        contract_id: &[u8; 32],
        document_type_name: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<i64, Error> {
        let drive_version = &platform_version.drive;
        let path = [
            &[crate::drive::RootTree::DataContractDocuments as u8] as &[u8],
            contract_id,
            &[1u8],
            document_type_name.as_bytes(),
        ];
        let mut drive_operations = vec![];
        let element = self
            .grove_get_raw_optional(
                grovedb_path::SubtreePath::from(path.as_slice()),
                &[0],
                crate::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                drive_version,
            )?
            .ok_or_else(|| {
                Error::Drive(crate::error::drive::DriveError::CorruptedCodeExecution(
                    "missing primary-key sum tree for an applied document type — \
                     insert_contract_operations_v0 must have created it",
                ))
            })?;
        Ok(element.sum_value_or_default())
    }
}
