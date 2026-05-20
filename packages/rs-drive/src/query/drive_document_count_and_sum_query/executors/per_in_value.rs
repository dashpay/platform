//! Joint count-and-sum `PerInValue` executor for
//! [`DocumentSumMode::PerInValue`] dispatch on the AVG no-prove path.
//!
//! Mirrors [`crate::query::drive_document_sum_query::executors::per_in_value`]
//! with the same two substitutions documented in the sibling
//! [`super::total`] module — element decoding via
//! `count_sum_value_or_default()` and picker-filtering on
//! `idx.countable.is_countable()`.
//!
//! ## Atomicity
//!
//! The per-In fan-out issues one grovedb read per In branch. When the
//! caller didn't provide a transaction, we open a short-lived shared
//! read transaction internally and reuse it across every per-branch
//! read so the joint executor sees a single grovedb snapshot. This is
//! the same atomicity contract the pre-#3687 AVG dispatcher
//! implemented at the dispatcher layer, moved here because the
//! dispatcher itself now does only a single executor call per AVG
//! request — the multi-read concern is purely an internal property of
//! this executor (and its `range_no_proof` sibling, which has the same
//! per-In fan-out shape for the compound-flat-summed branch).

use super::super::super::drive_document_average_query::{AverageEntry, DocumentAverageResponse};
use super::super::super::drive_document_sum_query::index_picker::find_summable_index_for_where_clauses;
use super::super::super::drive_document_sum_query::{DriveDocumentSumQuery, RangeSumOptions};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::TransactionArg;

impl Drive {
    /// Cartesian-forks the single `In` clause into one Equal-per-value
    /// sub-query against a `summable + countable` index; reads
    /// `(count, sum)` from each per-branch point-lookup walk in one
    /// grovedb read each, then emits per-In-value
    /// [`AverageEntry { in_key: None, key, count, sum }`] entries.
    ///
    /// Returns [`DocumentAverageResponse::Entries`].
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_and_sum_per_in_value_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        options: RangeSumOptions,
        limit: Option<u32>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        // Enforce exactly one `In` clause. The sum-side analog
        // documents the silent-drop bug this guards against; same
        // failure mode would apply here.
        let in_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|wc| wc.operator == WhereOperator::In)
            .collect();
        if in_clauses.len() != 1 {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "execute_document_count_and_sum_per_in_value_no_proof requires \
                     exactly one `in` clause",
                ),
            ));
        }
        let in_clause = in_clauses[0].clone();
        let in_values = in_clause.in_values().into_data_with_error()??;

        let other_clauses: Vec<WhereClause> = where_clauses
            .iter()
            .filter(|wc| wc.operator != WhereOperator::In)
            .cloned()
            .collect();

        // Open a short-lived shared read transaction if the caller
        // didn't provide one. Multiple per-In branches read grovedb
        // separately; without a shared snapshot a concurrent block
        // commit could slip between branches and produce a
        // `(count, sum)` pair from inconsistent state. Read-only; the
        // local transaction is dropped without commit at the end.
        let local_tx;
        let effective_transaction: TransactionArg = if transaction.is_some() {
            transaction
        } else {
            local_tx = self.grove.start_transaction();
            Some(&local_tx)
        };

        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
        // BTreeMap deduplicates by canonical key bytes and orders
        // ascending — matches sum-side analog. Storing the full
        // (count, sum) pair per key rather than separate maps means
        // we never need to zip post-walk; each grovedb read writes
        // exactly one entry into the map.
        let mut merged: std::collections::BTreeMap<Vec<u8>, (u64, i64)> =
            std::collections::BTreeMap::new();
        for value in in_values.iter() {
            let key_bytes = document_type.serialize_value_for_key(
                in_clause.field.as_str(),
                value,
                platform_version,
            )?;
            if merged.contains_key(&key_bytes) {
                continue;
            }

            let mut clauses_for_value = other_clauses.clone();
            clauses_for_value.push(WhereClause {
                field: in_clause.field.clone(),
                operator: WhereOperator::Equal,
                value: value.clone(),
            });

            let index = find_summable_index_for_where_clauses(
                document_type.indexes(),
                &clauses_for_value,
                &sum_property,
            )
            .filter(|idx| idx.countable.is_countable())
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "average query requires an index that declares BOTH \
                     `summable: \"<prop>\"` AND a countable terminator \
                     (`countable: \"countable\"` or `\"countableAllowingOffset\"`) \
                     matching the where clause fields"
                        .to_string(),
                ))
            })?;

            let sum_query = DriveDocumentSumQuery {
                document_type,
                contract_id,
                document_type_name: document_type_name.clone(),
                index,
                where_clauses: clauses_for_value,
                sum_property: sum_property.clone(),
            };

            let drive_version = &platform_version.drive;
            let path_query = sum_query.point_lookup_sum_path_query(platform_version)?;
            let mut drive_operations = vec![];
            let (results, _) = self.grove_get_path_query(
                &path_query,
                effective_transaction,
                QueryResultType::QueryElementResultType,
                &mut drive_operations,
                drive_version,
            )?;

            // For the per-In-value branch the inner walk visits at
            // most one count-sum-bearing element (the Equal arm fully
            // covered the index). Sum across the emitted elements to
            // handle the boundary case where the picker accepts an
            // index whose terminator is `In`-shaped in the same prop
            // (rare for AVG since the dispatched In was lifted out,
            // but cheap to do correctly).
            let mut count_acc: u64 = 0;
            let mut sum_acc: i64 = 0;
            for elem in results.elements.iter() {
                if let QueryResultElement::ElementResultItem(element) = elem {
                    let (c, s) = element.count_sum_value_or_default();
                    count_acc = count_acc.checked_add(c).ok_or_else(|| {
                        Error::Query(QuerySyntaxError::Unsupported(
                            "per-In-value count-and-sum overflowed u64 on the count axis \
                             when summing branch elements. Narrow the query or use \
                             multiple queries and combine client-side."
                                .to_string(),
                        ))
                    })?;
                    sum_acc = sum_acc.checked_add(s).ok_or_else(|| {
                        Error::Query(QuerySyntaxError::Unsupported(
                            "per-In-value count-and-sum overflowed i64 on the sum axis \
                             when summing branch elements. Narrow the query or use \
                             multiple queries and combine client-side."
                                .to_string(),
                        ))
                    })?;
                }
            }
            merged.insert(key_bytes, (count_acc, sum_acc));
        }

        let mut entries: Vec<AverageEntry> = merged
            .into_iter()
            .map(|(key, (count, sum))| AverageEntry {
                in_key: None,
                key,
                count: Some(count),
                sum: Some(sum),
            })
            .collect();
        if !options.left_to_right {
            entries.reverse();
        }
        // Apply caller's `limit` AFTER ordering — same shape as
        // count's `execute_document_count_per_in_value_no_proof`. The
        // BTreeMap iteration was already ascending; an explicit
        // descending order reverses the vec, then the truncate caps
        // it at `limit` from the front.
        if let Some(l) = limit {
            entries.truncate(l as usize);
        }
        Ok(DocumentAverageResponse::Entries(entries))
    }
}
