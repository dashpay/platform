use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::Document;
use dpp::platform_value::Value;
use grovedb::TransactionArg;
use indexmap::IndexMap;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    // TODO(withdrawals): Currently it queries only up to 100 documents.
    //  It works while we don't have pooling

    pub(super) fn find_withdrawal_documents_by_status_and_transaction_indices_v0(
        &self,
        status: withdrawals_contract::WithdrawalStatus,
        transaction_indices: &[WithdrawalTransactionIndex],
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let mut where_clauses = BTreeMap::new();

        where_clauses.insert(
            withdrawal::properties::STATUS.to_string(),
            WhereClause {
                field: withdrawal::properties::STATUS.to_string(),
                operator: crate::query::WhereOperator::Equal,
                value: Value::U8(status as u8),
            },
        );

        where_clauses.insert(
            withdrawal::properties::TRANSACTION_INDEX.to_string(),
            WhereClause {
                field: withdrawal::properties::TRANSACTION_INDEX.to_string(),
                operator: crate::query::WhereOperator::In,
                value: Value::Array(
                    transaction_indices
                        .iter()
                        .map(|index| Value::U64(*index))
                        .collect::<Vec<_>>(),
                ),
            },
        );

        let mut order_by = IndexMap::new();

        order_by.insert(
            withdrawal::properties::TRANSACTION_INDEX.to_string(),
            OrderClause {
                field: withdrawal::properties::TRANSACTION_INDEX.to_string(),
                ascending: true,
            },
        );

        let contract = self.cache.system_data_contracts.load_withdrawals();

        let document_type = contract.document_type_for_name(withdrawal::NAME)?;

        let drive_query = DriveDocumentQuery {
            contract: &contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: where_clauses,
            },
            offset: None,
            limit: Some(limit),
            order_by,
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

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

#[cfg(test)]
mod tests {
    use crate::config::DEFAULT_QUERY_LIMIT;
    use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
    use crate::util::test_helpers::setup::{
        setup_document, setup_drive_with_initial_state_structure, setup_system_data_contract,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::version::PlatformVersion;
    use dpp::withdrawal::Pooling;

    use super::*;

    #[test]
    fn test_find_pooled_withdrawal_documents_by_transaction_index() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&drive, &data_contract, Some(&transaction));

        let owner_id = Identifier::new([1u8; 32]);

        let transaction_index: WithdrawalTransactionIndex = 1;

        let document = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::POOLED as u8,
                "transactionIndex": transaction_index,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected to get withdrawal document");

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        setup_document(
            &drive,
            &document,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let found_document = drive
            .find_withdrawal_documents_by_status_and_transaction_indices(
                withdrawals_contract::WithdrawalStatus::POOLED,
                &[transaction_index],
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to find document by it's transaction id");

        assert_eq!(found_document.len(), 1);
    }
}
