use dpp::block::block_info::BlockInfo;
use dpp::dashcore::ScriptBuf;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::identity::convert_credits_to_duffs;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::version::PlatformVersion;
use dpp::withdrawal::{core_dust_threshold_duffs, WithdrawalTransactionIndex};
use std::collections::BTreeSet;

use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Version 2 changes on version 1 by giving up on withdrawals Core can never mine.
    ///
    /// An expired withdrawal is re-signed with a fresh quorum and sign height exactly as in
    /// v1, unless its whole amount is below the dust threshold Core's mempool applies to its
    /// output script (`core_dust_relay_fee_per_kb`, 546 duffs for P2PKH at Core's default).
    /// Every node's mempool rejects such an asset unlock as `dust`, so it never reaches a
    /// block, expires 48 Core blocks later, and v1 re-signed it forever. The whole amount is
    /// the right quantity to test: the payout can never exceed the amount, whichever
    /// converter version built the transaction, so an amount below the threshold is
    /// unmineable under every one of them.
    ///
    /// Such a withdrawal is marked FAILED, a terminal status, and its untied transaction is
    /// dropped from the broadcasted tree so nothing signs it again. The withdrawn credits
    /// are NOT returned: a quorum signature for the payout was released, dust is a mempool
    /// policy rather than a consensus rule, and Platform cannot prove a signed transaction
    /// will never be mined, so a refund could double-spend Core's credit pool. They stay
    /// locked in that pool.
    pub(super) fn rebroadcast_expired_withdrawal_documents_v2(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let dust_relay_fee_per_kb = platform_version
            .system_limits
            .core_dust_relay_fee_per_kb
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "rebroadcast_expired_withdrawal_documents v2 requires a Core dust relay fee",
            )))?;

        let expired_withdrawal_documents = self.drive.fetch_oldest_withdrawal_documents_by_status(
            WithdrawalStatus::EXPIRED.into(),
            platform_version
                .system_limits
                .retry_signing_expired_withdrawal_documents_per_block_limit,
            transaction.into(),
            platform_version,
        )?;

        if expired_withdrawal_documents.is_empty() {
            return Ok(());
        }

        let mut indices_to_retry: BTreeSet<WithdrawalTransactionIndex> = BTreeSet::new();
        let mut indices_to_fail: BTreeSet<WithdrawalTransactionIndex> = BTreeSet::new();
        let mut documents_to_update = Vec::with_capacity(expired_withdrawal_documents.len());

        for mut document in expired_withdrawal_documents {
            let withdrawal_index = document
                .properties()
                .get_optional_u64(withdrawal::properties::TRANSACTION_INDEX)?
                .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                    "Can't get transaction index from withdrawal document".to_string(),
                )))?;

            let status = if withdrawal_amount_is_core_dust(&document, dust_relay_fee_per_kb)? {
                tracing::warn!(
                    withdrawal_index,
                    "Withdrawal with transaction index {withdrawal_index} pays out below Core's dust threshold and can never be mined, marking it as failed",
                );

                indices_to_fail.insert(withdrawal_index);

                WithdrawalStatus::FAILED
            } else {
                indices_to_retry.insert(withdrawal_index);

                document.set_u64(
                    withdrawal::properties::TRANSACTION_SIGN_HEIGHT,
                    block_info.core_height as u64,
                );

                WithdrawalStatus::BROADCASTED
            };

            document.set_u8(withdrawal::properties::STATUS, status.into());

            document.set_updated_at(Some(block_info.time_ms));

            document.increment_revision().map_err(Error::Protocol)?;

            documents_to_update.push(document);
        }

        // One untied transaction is built per withdrawal document, so an index is never
        // shared. Should that ever change, a transaction another document still retries
        // must not be dropped.
        let indices_to_fail = indices_to_fail
            .difference(&indices_to_retry)
            .copied()
            .collect::<Vec<_>>();

        let mut drive_operations: Vec<DriveOperation> = vec![];

        self.drive
            .move_broadcasted_withdrawal_transactions_back_to_queue_operations(
                indices_to_retry.into_iter().collect(),
                &mut drive_operations,
                platform_version,
            )?;

        self.drive
            .remove_broadcasted_withdrawal_transactions_after_completion_operations(
                indices_to_fail,
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

/// Whether the whole withdrawal amount is below the dust threshold Core's mempool applies
/// to the withdrawal's output script, so no asset unlock built from this document can
/// ever be relayed.
fn withdrawal_amount_is_core_dust(
    document: &Document,
    dust_relay_fee_per_kb: u64,
) -> Result<bool, Error> {
    let amount: u64 = document
        .properties()
        .get_integer(withdrawal::properties::AMOUNT)?;

    let output_script_bytes = document
        .properties()
        .get_bytes(withdrawal::properties::OUTPUT_SCRIPT)?;

    let amount_duffs = convert_credits_to_duffs(amount).map_err(Error::Protocol)?;

    let output_script = ScriptBuf::from_bytes(output_script_bytes);

    Ok(amount_duffs < core_dust_threshold_duffs(&output_script, dust_relay_fee_per_kb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::prelude::Identifier;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::withdrawal::Pooling;
    use drive::config::DEFAULT_QUERY_LIMIT;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_transactions_broadcasted_path,
        get_withdrawal_transactions_broadcasted_path_vec,
        get_withdrawal_transactions_queue_path_vec,
    };
    use drive::grovedb::query_result_type::QueryResultType;
    use drive::grovedb::{Element, PathQuery, Query, SizedQuery};
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};

    /// Mainnet withdrawal 9815: 191 duffs to a P2PKH address, admitted under the 190-duff
    /// floor of the first system limits and rejected by Core as dust ever since.
    const DUST_AMOUNT_CREDITS: u64 = 191_000;
    const VIABLE_AMOUNT_CREDITS: u64 = 1_000_000;

    fn p2pkh_output_script() -> CoreScript {
        let mut bytes = vec![0x76, 0xa9, 0x14];
        bytes.extend_from_slice(&[0x11; 20]);
        bytes.extend_from_slice(&[0x88, 0xac]);
        CoreScript::from_bytes(bytes)
    }

    fn block_info() -> BlockInfo {
        BlockInfo {
            time_ms: 1_700_000_000_000,
            height: 10,
            core_height: 2_000,
            epoch: Epoch::default(),
        }
    }

    /// Stores an EXPIRED withdrawal document and the untied transaction bytes the dequeue
    /// step left in the broadcasted tree for it, the state a withdrawal is in when the
    /// rebroadcast step sees it.
    fn setup_expired_withdrawal(
        platform: &crate::platform_types::platform::Platform<crate::rpc::core::MockCoreRPCLike>,
        transaction: &Transaction,
        amount: u64,
        transaction_index: u64,
        platform_version: &PlatformVersion,
    ) -> Document {
        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&platform.drive, &data_contract, Some(transaction));

        let document = get_withdrawal_document_fixture(
            &data_contract,
            Identifier::new([7u8; 32]),
            platform_value!({
                "amount": amount,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": p2pkh_output_script(),
                "status": WithdrawalStatus::EXPIRED as u8,
                "transactionIndex": transaction_index,
                "transactionSignHeight": 1_000u64,
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
            &document,
            &data_contract,
            document_type,
            Some(transaction),
        );

        platform
            .drive
            .grove_insert_if_not_exists(
                get_withdrawal_transactions_broadcasted_path()
                    .as_slice()
                    .into(),
                &transaction_index.to_be_bytes(),
                Element::Item(vec![0xAB; 64], None),
                Some(transaction),
                None,
                &platform_version.drive,
            )
            .expect("expected to store the untied transaction bytes");

        document
    }

    fn keys_in(
        platform: &crate::platform_types::platform::Platform<crate::rpc::core::MockCoreRPCLike>,
        path: Vec<Vec<u8>>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Vec<Vec<u8>> {
        let mut query = Query::new();
        query.insert_all();
        let (results, _) = platform
            .drive
            .grove_get_raw_path_query(
                &PathQuery::new(path, SizedQuery::new(query, None, None)),
                Some(transaction),
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to query the withdrawal transaction tree");
        results
            .to_key_elements()
            .into_iter()
            .map(|(key, _)| key)
            .collect()
    }

    fn documents_with_status(
        platform: &crate::platform_types::platform::Platform<crate::rpc::core::MockCoreRPCLike>,
        status: WithdrawalStatus,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                status.into(),
                DEFAULT_QUERY_LIMIT,
                Some(transaction),
                platform_version,
            )
            .expect("expected to fetch withdrawal documents by status")
    }

    #[test]
    fn should_fail_an_expired_withdrawal_whose_amount_is_core_dust_without_resigning_it() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let document = setup_expired_withdrawal(
            &platform,
            &transaction,
            DUST_AMOUNT_CREDITS,
            9_815,
            platform_version,
        );

        let block_info = block_info();

        platform
            .rebroadcast_expired_withdrawal_documents_v2(
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to process expired withdrawals");

        let failed = documents_with_status(
            &platform,
            WithdrawalStatus::FAILED,
            &transaction,
            platform_version,
        );
        assert_eq!(failed.len(), 1);
        let failed = &failed[0];
        assert_eq!(failed.id(), document.id());
        assert_eq!(failed.revision(), document.revision().map(|r| r + 1));
        assert_eq!(failed.updated_at(), Some(block_info.time_ms));
        assert_eq!(
            failed
                .properties()
                .get_integer::<u64>(withdrawal::properties::TRANSACTION_SIGN_HEIGHT)
                .expect("sign height"),
            1_000,
            "a failed withdrawal keeps the sign height of its last signing"
        );

        for status in [WithdrawalStatus::EXPIRED, WithdrawalStatus::BROADCASTED] {
            assert!(
                documents_with_status(&platform, status, &transaction, platform_version).is_empty(),
                "no document may remain in {status}"
            );
        }

        assert!(
            keys_in(
                &platform,
                get_withdrawal_transactions_broadcasted_path_vec(),
                &transaction,
                platform_version
            )
            .is_empty(),
            "the untied transaction must be dropped so it is never signed again"
        );
        assert!(
            keys_in(
                &platform,
                get_withdrawal_transactions_queue_path_vec(),
                &transaction,
                platform_version
            )
            .is_empty(),
            "the untied transaction must not be queued for re-signing"
        );
    }

    #[test]
    fn should_still_resign_an_expired_withdrawal_above_core_dust() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let document = setup_expired_withdrawal(
            &platform,
            &transaction,
            VIABLE_AMOUNT_CREDITS,
            42,
            platform_version,
        );

        let block_info = block_info();

        platform
            .rebroadcast_expired_withdrawal_documents_v2(
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to process expired withdrawals");

        let broadcasted = documents_with_status(
            &platform,
            WithdrawalStatus::BROADCASTED,
            &transaction,
            platform_version,
        );
        assert_eq!(broadcasted.len(), 1);
        let broadcasted = &broadcasted[0];
        assert_eq!(broadcasted.id(), document.id());
        assert_eq!(
            broadcasted
                .properties()
                .get_integer::<u64>(withdrawal::properties::TRANSACTION_SIGN_HEIGHT)
                .expect("sign height"),
            block_info.core_height as u64
        );

        assert!(documents_with_status(
            &platform,
            WithdrawalStatus::FAILED,
            &transaction,
            platform_version
        )
        .is_empty());

        assert!(keys_in(
            &platform,
            get_withdrawal_transactions_broadcasted_path_vec(),
            &transaction,
            platform_version
        )
        .is_empty());
        assert_eq!(
            keys_in(
                &platform,
                get_withdrawal_transactions_queue_path_vec(),
                &transaction,
                platform_version
            ),
            vec![42u64.to_be_bytes().to_vec()],
            "the untied transaction must be queued for re-signing"
        );
    }

    #[test]
    fn should_refuse_to_run_without_a_core_dust_relay_fee() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let platform_version_13 = PlatformVersion::get(13).expect("expected platform version 13");
        assert!(platform_version_13
            .system_limits
            .core_dust_relay_fee_per_kb
            .is_none());

        let result = platform.rebroadcast_expired_withdrawal_documents_v2(
            &block_info(),
            &transaction,
            platform_version_13,
        );

        assert!(matches!(
            result,
            Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_only_treat_amounts_below_the_script_threshold_as_dust() {
        let platform_version = PlatformVersion::latest();
        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        let dust_relay_fee_per_kb = platform_version
            .system_limits
            .core_dust_relay_fee_per_kb
            .expect("v14 sets the dust relay fee");

        for (amount, expected) in [
            (DUST_AMOUNT_CREDITS, true),
            (545_000u64, true),
            (546_000u64, false),
            (VIABLE_AMOUNT_CREDITS, false),
        ] {
            let document = get_withdrawal_document_fixture(
                &data_contract,
                Identifier::new([7u8; 32]),
                platform_value!({
                    "amount": amount,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": p2pkh_output_script(),
                    "status": WithdrawalStatus::EXPIRED as u8,
                    "transactionIndex": 1u64,
                    "transactionSignHeight": 1u64,
                }),
                None,
                platform_version.protocol_version,
            )
            .expect("expected withdrawal document");

            assert_eq!(
                withdrawal_amount_is_core_dust(&document, dust_relay_fee_per_kb)
                    .expect("expected a dust verdict"),
                expected,
                "{amount} credits"
            );
        }
    }
}
