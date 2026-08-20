use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::withdrawal::WithdrawalTransactionIndex;
use grovedb::TransactionArg;
use indexmap::IndexMap;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    // v1 (protocol version 14): identical to v0 except the transaction-index
    // `In` clause is carried in `InternalClauses.in_clauses` instead of
    // riding in `equal_clauses`. Both shapes lower to the identical grovedb
    // path query (pinned by `withdrawal_in_clause_placement_equivalence`
    // in the query tests); this is a structural cleanup version.

    // TODO(withdrawals): Currently it queries only up to 100 documents.
    //  It works while we don't have pooling
    // This should be a pathquery directly instead of a drive query for efficiency

    pub(super) fn find_withdrawal_documents_by_status_and_transaction_indices_v1(
        &self,
        status: withdrawals_contract::WithdrawalStatus,
        transaction_indices: &[WithdrawalTransactionIndex],
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let mut equal_clauses = BTreeMap::new();

        equal_clauses.insert(
            withdrawal::properties::STATUS.to_string(),
            WhereClause {
                field: withdrawal::properties::STATUS.to_string(),
                operator: crate::query::WhereOperator::Equal,
                value: Value::U8(status as u8),
            },
        );

        let transaction_index_in_clause = WhereClause {
            field: withdrawal::properties::TRANSACTION_INDEX.to_string(),
            operator: crate::query::WhereOperator::In,
            value: Value::Array(
                transaction_indices
                    .iter()
                    .map(|index| Value::U64(*index))
                    .collect::<Vec<_>>(),
            ),
        };

        let mut order_by = IndexMap::new();

        order_by.insert(
            withdrawal::properties::TRANSACTION_INDEX.to_string(),
            OrderClause {
                field: withdrawal::properties::TRANSACTION_INDEX.to_string(),
                ascending: true,
            },
        );

        let contract = self
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)?;

        let document_type = contract.document_type_for_name(withdrawal::NAME)?;

        let drive_query = DriveDocumentQuery {
            contract: &contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: vec![transaction_index_in_clause],
                range_clause: None,
                equal_clauses,
            },
            offset: None,
            limit: Some(limit),
            order_by,
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        // todo: deal with cost of this operation
        let outcome = self.query_documents(
            drive_query,
            None,
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;

        Ok(outcome.documents_owned())
    }
}
