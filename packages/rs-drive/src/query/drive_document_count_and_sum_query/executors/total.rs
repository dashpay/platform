//! Joint count-and-sum Total executor for [`DocumentSumMode::Total`]
//! dispatch on the AVG no-prove path.
//!
//! Mirrors [`crate::query::drive_document_sum_query::executors::total`]
//! one-to-one with two substitutions:
//!
//! 1. **Element decoding**: the sum-side executor reads
//!    [`Element::sum_value_or_default()`]; this one reads
//!    [`Element::count_sum_value_or_default()`] so a single visited
//!    element yields both metrics. The terminator's value tree is a
//!    `CountSumTree` / `ProvableCountSumTree` /
//!    `ProvableCountProvableSumTree` (the count-sum-bearing family),
//!    so the joint decoder is well-defined on every shape the picker
//!    accepts.
//! 2. **Index selection**: the picker is
//!    [`find_summable_index_for_where_clauses`] (sum's), but with an
//!    additional `.filter(|idx| idx.countable.is_countable())` so the
//!    chosen index also carries the `countable` declaration the count
//!    side needs. The AVG prove path's point-lookup arm does the same
//!    filter (see `drive_document_average_query::drive_dispatcher::
//!    execute_document_average_prove`'s no-range arm).
//!
//! Routing semantics match sum's: this executor handles both the
//! empty-where case (doctype's `documents_summable + documents_countable`
//! primary-key fast path) AND the Equal-only fully-covered Equal/In
//! point-lookup case. The branch on `where_clauses.is_empty()` inside
//! the executor body is the same one sum's `total.rs` carries.

use super::super::super::drive_document_average_query::DocumentAverageResponse;
use super::super::super::drive_document_sum_query::index_picker::find_summable_index_for_where_clauses;
use super::super::super::drive_document_sum_query::DriveDocumentSumQuery;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::WhereClause;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::TransactionArg;

impl Drive {
    /// Joint count-and-sum total for the empty-where /
    /// Equal-only-fully-covered point-lookup shape — the AVG no-prove
    /// analog of [`Drive::execute_document_sum_total_no_proof`].
    ///
    /// Returns [`DocumentAverageResponse::Aggregate { count, sum }`]
    /// directly (collapsed to a single pair across all matched
    /// documents). The PerInValue branch is the sibling executor that
    /// emits one entry per In branch.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_and_sum_total_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        use dpp::data_contract::document_type::accessors::{
            DocumentTypeV0Getters, DocumentTypeV2Getters,
        };

        // Empty-where fast path: unfiltered (count, sum) on a doctype
        // that declares BOTH `documents_summable: Some(matching_prop)`
        // AND `documents_countable: true` reads the primary-key
        // count-sum-bearing tree directly (O(1)). Mirrors the sum-side
        // analog with the additional `documents_countable` predicate;
        // the AVG prove path's empty-where fast path enforces the same
        // pair.
        if where_clauses.is_empty()
            && document_type.documents_countable()
            && document_type
                .documents_summable()
                .map(|p| p == sum_property)
                .unwrap_or(false)
        {
            let (count, sum) = self.read_primary_key_count_sum_tree(
                &contract_id,
                &document_type_name,
                transaction,
                platform_version,
            )?;
            return Ok(DocumentAverageResponse::Aggregate { count, sum });
        }

        let index = find_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
        )
        .filter(|idx| idx.countable.is_countable())
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "average query requires an index that declares BOTH \
                 `summable: \"<prop>\"` AND a countable terminator \
                 (`countable: \"countable\"` or `\"countableAllowingOffset\"`) \
                 whose properties exactly match the where clause fields, \
                 OR `documentsSummable: \"<prop>\"` AND `documentsCountable: true` \
                 on the document type for the unfiltered total case"
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

        // Reuse sum's `point_lookup_sum_path_query` builder — the path
        // query shape is identical for count-and-sum point lookups; the
        // only difference is that we decode each emitted element via
        // `count_sum_value_or_default()` rather than
        // `sum_value_or_default()`. The terminator's value tree on a
        // `summable + countable` index is a `CountSumTree` /
        // `ProvableCountSumTree`, so both fields are present on every
        // emitted element.
        let drive_version = &platform_version.drive;
        let path_query = sum_query.point_lookup_sum_path_query(platform_version)?;
        let mut drive_operations = vec![];
        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryElementResultType,
            &mut drive_operations,
            drive_version,
        )?;

        // Fold across emitted count-sum-bearing elements:
        // - Equal-only: 0 or 1 element (0 when the branch is absent).
        // - In at any position: one element per In branch that has at
        //   least one doc; missing branches contribute (0, 0).
        //
        // `checked_add` rather than `saturating_add` / wrapping add so
        // an overflowed aggregate fails deterministically with a typed
        // query error rather than silently clamping. The sum-side
        // executor uses the same pattern for its i64 fold; we
        // additionally guard u64 overflow on the count axis.
        let mut count_acc: u64 = 0;
        let mut sum_acc: i64 = 0;
        for elem in results.elements.iter() {
            if let QueryResultElement::ElementResultItem(element) = elem {
                let (c, s) = element.count_sum_value_or_default();
                count_acc = count_acc.checked_add(c).ok_or_else(|| {
                    Error::Query(QuerySyntaxError::Unsupported(
                        "point-lookup count-and-sum overflowed u64 on the count axis \
                         when summing per-In branches. Narrow the query (smaller In set) \
                         or use multiple queries and combine client-side."
                            .to_string(),
                    ))
                })?;
                sum_acc = sum_acc.checked_add(s).ok_or_else(|| {
                    Error::Query(QuerySyntaxError::Unsupported(
                        "point-lookup count-and-sum overflowed i64 on the sum axis \
                         when summing per-In branches. Narrow the query (smaller In set) \
                         or use multiple queries and combine client-side."
                            .to_string(),
                    ))
                })?;
            }
        }
        Ok(DocumentAverageResponse::Aggregate {
            count: count_acc,
            sum: sum_acc,
        })
    }

    /// Reads the document-type primary-key tree's count-sum-bearing
    /// element (`[contract_doc, contract_id, [1], doctype, 0]`) and
    /// returns `count_sum_value_or_default()`. Used by the
    /// `documents_summable + documents_countable` empty-where fast path
    /// on the joint count-and-sum flow.
    ///
    /// `insert_contract_operations_v0` unconditionally creates a
    /// count-sum-bearing tree at `[..., doctype, 0]` for every applied
    /// document type whose `documents_summable` is set AND whose
    /// `documents_countable` is true, so a missing element here
    /// indicates contract-state corruption or a mis-applied contract —
    /// fail fast rather than silently returning `(0, 0)`. Mirrors
    /// `read_primary_key_sum_tree`'s contract.
    pub(super) fn read_primary_key_count_sum_tree(
        &self,
        contract_id: &[u8; 32],
        document_type_name: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, i64), Error> {
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
                    "missing primary-key count-sum tree for an applied document type — \
                     insert_contract_operations_v0 must have created it when both \
                     documents_summable and documents_countable are set",
                ))
            })?;
        Ok(element.count_sum_value_or_default())
    }
}
