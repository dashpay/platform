use std::collections::BTreeMap;

use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::system_data_contracts::withdrawals_contract;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use indexmap::IndexMap;

use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::{
    drive::Drive,
    error::{drive::DriveError, Error},
    query::{DriveQuery, InternalClauses, OrderClause, WhereClause},
};

impl Drive {
    /// Fetch withdrawal documents by it's status
    pub fn fetch_withdrawal_documents_by_status(
        &self,
        status: u8,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let data_contract_id = withdrawals_contract::ID;

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_fee(
                data_contract_id.to_buffer(),
                None,
                true,
                transaction,
                platform_version,
            )?
            .1
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't fetch data contract",
                ))
            })?;

        let document_type = contract_fetch_info
            .contract
            .document_type_for_name(withdrawal::NAME)?;

        let mut where_clauses = BTreeMap::new();

        //todo: make this lazy loaded or const
        where_clauses.insert(
            withdrawal::properties::STATUS.to_string(),
            WhereClause {
                field: withdrawal::properties::STATUS.to_string(),
                operator: crate::query::WhereOperator::Equal,
                value: Value::U8(status),
            },
        );

        let mut order_by = IndexMap::new();

        order_by.insert(
            withdrawal::properties::UPDATED_AT.to_string(),
            OrderClause {
                field: withdrawal::properties::UPDATED_AT.to_string(),
                ascending: true,
            },
        );

        let drive_query = DriveQuery {
            contract: &contract_fetch_info.contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: where_clauses,
            },
            offset: None,
            limit: Some(100),
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

    /// Find documents by it's transactionIndex field
    pub fn find_withdrawal_documents_by_status_and_transaction_indices(
        &self,
        status: withdrawals_contract::WithdrawalStatus,
        transaction_indices: &[WithdrawalTransactionIndex],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        // TODO: Use drive cache system_data_contracts
        let data_contract_id = withdrawals_contract::ID;

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_fee(
                data_contract_id.to_buffer(),
                None,
                true,
                transaction,
                platform_version,
            )?
            .1
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't fetch data contract",
                ))
            })?;

        let document_type = contract_fetch_info
            .contract
            .document_type_for_name(withdrawal::NAME)?;

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

        // TODO: Currently it queries only 100 documents
        //  We need to query all documents. It would be nice to update DriveQuery
        //  with internal param that will allow us to disable strict limits

        let drive_query = DriveQuery {
            contract: &contract_fetch_info.contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: where_clauses,
            },
            offset: None,
            limit: None,
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
    use dpp::prelude::Identifier;
    use dpp::system_data_contracts::withdrawals_contract;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::tests::helpers::setup::{setup_document, setup_system_data_contract};

    mod fetch_withdrawal_documents_by_status {
        use super::*;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::identity::core_script::CoreScript;
        use dpp::platform_value::platform_value;
        use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;
        use dpp::withdrawal::Pooling;

        #[test]
        fn test_return_list_of_documents() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            let data_contract =
                load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                    .expect("to load system data contract");

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 0);

            let owner_id = Identifier::new([1u8; 32]);

            let document = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                    "transactionIndex": 1u64,
                }),
                None,
                platform_version.protocol_version,
            )
            .expect("expected withdrawal document");

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

            let document = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::POOLED,
                    "transactionIndex": 2u64,
                }),
                None,
                platform_version.protocol_version,
            )
            .expect("expected withdrawal document");

            setup_document(
                &drive,
                &document,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
        }
    }

    mod find_withdrawal_documents_by_status_and_transaction_indices {
        use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::DocumentV0Getters;
        use dpp::identity::core_script::CoreScript;
        use dpp::platform_value::{platform_value, Bytes32};
        use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
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
                    Some(&transaction),
                    platform_version,
                )
                .expect("to find document by it's transaction id");

            assert_eq!(found_document.len(), 1);
        }
    }
}
