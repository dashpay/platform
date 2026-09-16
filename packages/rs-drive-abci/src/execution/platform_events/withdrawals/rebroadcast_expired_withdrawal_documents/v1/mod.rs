use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;

use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::version::PlatformVersion;
use std::collections::BTreeSet;

use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};
use dpp::withdrawal::WithdrawalTransactionIndex;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Version 1 changes on Version 0, by not having the Core 2 Quorum limit.
    /// We should switch to Version 1 once Core has fixed the issue
    pub(super) fn rebroadcast_expired_withdrawal_documents_v1(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let expired_withdrawal_documents_to_retry_signing =
            self.drive.fetch_oldest_withdrawal_documents_by_status(
                WithdrawalStatus::EXPIRED.into(),
                platform_version
                    .system_limits
                    .retry_signing_expired_withdrawal_documents_per_block_limit,
                transaction.into(),
                platform_version,
            )?;

        if expired_withdrawal_documents_to_retry_signing.is_empty() {
            return Ok(());
        }

        // Collecting unique withdrawal indices of expired documents
        let expired_withdrawal_indices: Vec<WithdrawalTransactionIndex> =
            expired_withdrawal_documents_to_retry_signing
                .iter()
                .map(|document| {
                    document
                        .properties()
                        .get_optional_u64(withdrawal::properties::TRANSACTION_INDEX)?
                        .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                            "Can't get transaction index from withdrawal document".to_string(),
                        )))
                })
                .collect::<Result<BTreeSet<WithdrawalTransactionIndex>, Error>>()?
                .into_iter()
                .collect();

        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Collecting only documents that have been updated
        let mut documents_to_update = Vec::new();

        for mut document in expired_withdrawal_documents_to_retry_signing {
            document.set_u8(
                withdrawal::properties::STATUS,
                WithdrawalStatus::BROADCASTED as u8,
            );

            document.set_u64(
                withdrawal::properties::TRANSACTION_SIGN_HEIGHT,
                block_info.core_height as u64,
            );

            document.set_updated_at(Some(block_info.time_ms));

            document.increment_revision().map_err(Error::Protocol)?;

            documents_to_update.push(document);
        }

        if documents_to_update.is_empty() {
            return Ok(());
        }

        self.drive
            .move_broadcasted_withdrawal_transactions_back_to_queue_operations(
                expired_withdrawal_indices,
                &mut drive_operations,
                platform_version,
            )?;

        let withdrawals_contract = self
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)?;

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
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;
    use dpp::data_contracts::SystemDataContract;
    use dpp::system_data_contracts::load_system_data_contract;
    use drive::util::test_helpers::setup::setup_system_data_contract;

    /// With the withdrawals system data contract installed but no EXPIRED
    /// documents, the function must return `Ok(())` on the `is_empty()`
    /// guard without attempting any drive operations.
    #[test]
    fn v1_returns_ok_when_no_expired_documents() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load withdrawals system contract");
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

        let block_info = BlockInfo {
            time_ms: 42,
            height: 10,
            core_height: 100,
            epoch: Epoch::default(),
        };

        platform
            .rebroadcast_expired_withdrawal_documents_v1(
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("v1 must return Ok when there are no expired documents");
    }
}
