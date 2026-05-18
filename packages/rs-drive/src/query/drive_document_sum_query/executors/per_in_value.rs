//! Per-`In`-value sum executor for
//! [`super::super::DocumentSumMode::PerInValue`] dispatch — Equal/In
//! sum queries without a range clause, emitting one sum entry per
//! `In` value.
//!
//! Mirror of count's `executors/per_in_value.rs`. Same cartesian-fork
//! pattern; substitutions per `executors/mod.rs`.

use super::super::index_picker::find_summable_index_for_where_clauses;
use super::super::{DriveDocumentSumQuery, RangeSumOptions, SumEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Cartesian-forks the single `In` clause into one Equal-per-value
    /// sub-query against a `summable: "<sum_property>"` index; aggregates
    /// the per-branch sums into a `BTreeMap<serialized_value, i64>`
    /// (deduplicated by key bytes) and emits per-In-value `SumEntry`
    /// where `in_key: None`, `key: serialized_value`, `sum: Some(i64)`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_sum_per_in_value_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        options: RangeSumOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SumEntry>, Error> {
        // Enforce exactly one `In` clause. The previous `find` would
        // silently use the first In and drop any others, producing
        // incorrect per-value sums.
        let in_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|wc| wc.operator == WhereOperator::In)
            .collect();
        if in_clauses.len() != 1 {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "execute_document_sum_per_in_value_no_proof requires exactly one `in` clause",
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

        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
        let mut merged: std::collections::BTreeMap<Vec<u8>, i64> =
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
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "sum query requires a summable index on the document type that \
                     matches the where clause properties and sum_property"
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
            let results = sum_query.execute_no_proof(self, transaction, platform_version)?;
            let sum = results.first().and_then(|entry| entry.sum).unwrap_or(0);
            merged.insert(key_bytes, sum);
        }

        let mut entries: Vec<SumEntry> = merged
            .into_iter()
            .map(|(key, sum)| SumEntry {
                in_key: None,
                key,
                sum: Some(sum),
            })
            .collect();
        if !options.left_to_right {
            entries.reverse();
        }
        Ok(entries)
    }
}
