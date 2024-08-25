use dashcore_rpc::json::AssetUnlockStatus;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;

use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::version::PlatformVersion;
use itertools::Itertools;
use std::collections::HashSet;

use drive::config::DEFAULT_QUERY_LIMIT;
use drive::drive::identity::withdrawals::WithdrawalTransactionIndex;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

const NUMBER_OF_BLOCKS_BEFORE_EXPIRED: u32 = 48;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update statuses for broadcasted withdrawals
    pub(super) fn update_broadcasted_withdrawal_statuses_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let broadcasted_withdrawal_documents =
            self.drive.fetch_oldest_withdrawal_documents_by_status(
                WithdrawalStatus::BROADCASTED.into(),
                DEFAULT_QUERY_LIMIT,
                transaction.into(),
                platform_version,
            )?;

        if broadcasted_withdrawal_documents.is_empty() {
            return Ok(());
        }

        // Collecting unique withdrawal indices
        let broadcasted_withdrawal_indices = broadcasted_withdrawal_documents
            .iter()
            .map(|document| {
                document
                    .properties()
                    .get_optional_u64(withdrawal::properties::TRANSACTION_INDEX)?
                    .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                        "Can't get transaction index from withdrawal document".to_string(),
                    )))
            })
            .collect::<Result<HashSet<WithdrawalTransactionIndex>, Error>>()?
            .into_iter()
            .collect_vec();

        let withdrawal_transaction_statuses = self.fetch_transactions_block_inclusion_status(
            block_info.core_height,
            &broadcasted_withdrawal_indices,
            platform_version,
        )?;

        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Collecting only documents that have been updated
        let mut documents_to_update = Vec::new();

        for mut document in broadcasted_withdrawal_documents {
            let withdrawal_index = document
                .properties()
                .get_optional_u64(withdrawal::properties::TRANSACTION_INDEX)?
                .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                    "Can't get transaction index from withdrawal document".to_string(),
                )))?;

            let transaction_sign_height = document
                .properties()
                .get_optional_u64(withdrawal::properties::TRANSACTION_SIGN_HEIGHT)?
                .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                    "Can't get transaction sign height from withdrawal document".to_string(),
                )))? as u32;

            let withdrawal_transaction_status = withdrawal_transaction_statuses
                .get(&withdrawal_index)
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "Withdrawal transaction with index {} is not found in Core",
                        withdrawal_index
                    );

                    &AssetUnlockStatus::Unknown
                });

            let block_height_difference = block_info.core_height - transaction_sign_height;

            let status = if withdrawal_transaction_status == &AssetUnlockStatus::Chainlocked {
                tracing::debug!(
                    transaction_sign_height,
                    "Withdrawal with transaction index {} is marked as complete",
                    withdrawal_index
                );

                WithdrawalStatus::COMPLETE
            } else if block_height_difference > NUMBER_OF_BLOCKS_BEFORE_EXPIRED {
                tracing::debug!(
                    transaction_sign_height,
                    "Withdrawal with transaction index {} is marked as expired",
                    withdrawal_index
                );

                WithdrawalStatus::EXPIRED
            } else {
                continue;
            };

            document.set_u8(withdrawal::properties::STATUS, status.into());

            document.set_updated_at(Some(block_info.time_ms));

            document.increment_revision().map_err(Error::Protocol)?;

            documents_to_update.push(document);
        }

        if documents_to_update.is_empty() {
            return Ok(());
        }

        let withdrawals_contract = self.drive.cache.system_data_contracts.load_withdrawals();

        self.drive.add_update_multiple_documents_operations(
            &documents_to_update,
            &withdrawals_contract,
            withdrawals_contract
                .document_type_for_name(withdrawal::NAME)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            block_info,
            transaction.into(),
            platform_version,
            None,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dashcore_rpc::json::{AssetUnlockStatus, AssetUnlockStatusResult};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::DocumentV0Getters;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
    use dpp::version::PlatformVersion;
    use dpp::withdrawal::Pooling;
    use dpp::{
        data_contracts::withdrawals_contract, tests::fixtures::get_withdrawal_document_fixture,
    };
    use dpp::{
        prelude::Identifier,
        system_data_contracts::{load_system_data_contract, SystemDataContract},
    };
    use drive::util::test_helpers::setup::setup_document;
    use drive::util::test_helpers::setup::setup_system_data_contract;

    #[test]
    fn test_statuses_are_updated() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut mock_rpc_client = MockCoreRPCLike::new();

        mock_rpc_client
            .expect_get_asset_unlock_statuses()
            .returning(move |indices: &[u64], _core_chain_locked_height| {
                Ok(indices
                    .iter()
                    .map(|index| {
                        let status = if index == &1 {
                            AssetUnlockStatus::Chainlocked
                        } else {
                            AssetUnlockStatus::Unknown
                        };

                        AssetUnlockStatusResult {
                            index: *index,
                            status,
                        }
                    })
                    .collect())
            });

        platform.core_rpc = mock_rpc_client;

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 0,
            height: 1,
            core_height: 96,
            epoch: Default::default(),
        };

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

        let owner_id = Identifier::new([1u8; 32]);

        let document_1 = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::BROADCASTED as u8,
                "transactionIndex": 1u64,
                "transactionSignHeight": 1,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        setup_document(
            &platform.drive,
            &document_1,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let document_2 = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::BROADCASTED as u8,
                "transactionIndex": 2u64,
                "transactionSignHeight": 1,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        setup_document(
            &platform.drive,
            &document_2,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        platform
            .update_broadcasted_withdrawal_statuses_v0(&block_info, &transaction, platform_version)
            .expect("to update withdrawal statuses");

        let documents = platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                WithdrawalStatus::EXPIRED.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        assert_eq!(documents.len(), 1);
        assert_eq!(
            documents.first().unwrap().id().to_vec(),
            document_2.id().to_vec()
        );

        let documents = platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                WithdrawalStatus::COMPLETE.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        assert_eq!(documents.len(), 1);
        assert_eq!(
            documents.first().unwrap().id().to_vec(),
            document_1.id().to_vec()
        );
    }
}
