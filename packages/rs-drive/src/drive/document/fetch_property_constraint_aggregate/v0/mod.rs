use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::drive_document_count_query::DriveDocumentCountQuery;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::property_constraints::{AggregateKind, AggregateRead};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 of [`Drive::fetch_property_constraint_aggregate`]. It reads the elements
    /// the count and sum queries read for the same total (the primary-key tree of a whole
    /// type, or the value tree of the answering index a point lookup reaches), through the
    /// same path-query builders, so the total is the one those queries report and prove. A
    /// branch no document reached yet is absent and adds 0.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn fetch_property_constraint_aggregate_v0(
        &self,
        contract_id: [u8; 32],
        counted: DocumentTypeRef,
        read: &AggregateRead,
        filter_values: &[(String, Value)],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<i128, Error> {
        let document_type_name = counted.name().clone();
        let path_query = if read.filter.is_empty() {
            match &read.kind {
                AggregateKind::Count => DriveDocumentCountQuery::primary_key_count_tree_path_query(
                    contract_id,
                    &document_type_name,
                ),
                AggregateKind::Sum { .. } => DriveDocumentSumQuery::primary_key_sum_path_query(
                    contract_id,
                    &document_type_name,
                ),
            }
        } else {
            let Some(index) = read.answering_index(&counted) else {
                return Err(Error::Drive(DriveError::CorruptedContractIndexes(format!(
                    "no index of document type {document_type_name} keeps the {} a registered \
                     propertyConstraints rule reads",
                    read.wire_name()
                ))));
            };
            let where_clauses = filter_values
                .iter()
                .map(|(field, value)| WhereClause {
                    field: field.clone(),
                    operator: WhereOperator::Equal,
                    value: value.clone(),
                })
                .collect();
            match &read.kind {
                AggregateKind::Count => DriveDocumentCountQuery {
                    document_type: counted,
                    contract_id,
                    document_type_name,
                    index,
                    where_clauses,
                }
                .point_lookup_count_path_query(platform_version)?,
                AggregateKind::Sum { property } => DriveDocumentSumQuery {
                    document_type: counted,
                    contract_id,
                    document_type_name,
                    index,
                    where_clauses,
                    sum_property: property.clone(),
                }
                .point_lookup_sum_path_query(platform_version)?,
            }
        };
        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryElementResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        let mut total = 0i128;
        for result in results.elements {
            let QueryResultElement::ElementResultItem(element) = result else {
                continue;
            };
            let value = match &read.kind {
                AggregateKind::Count => i128::from(element.count_value_or_default()),
                AggregateKind::Sum { .. } => i128::from(element.sum_value_or_default()),
            };
            // Each element holds at most a `u64` or an `i64`, and a point lookup reaches one
            total = total.checked_add(value).ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "a propertyConstraints total overflowed an i128".to_string(),
                ))
            })?;
        }
        Ok(total)
    }
}
