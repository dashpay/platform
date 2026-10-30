use dpp::block::block_info::BlockInfo;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::AssetUnlockPayload;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};

use crate::{
    error::{execution::ExecutionError, Error},
    execution::platform_events::withdrawals::withdrawal_amount_is_core_dust,
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};
use dpp::version::PlatformVersion;
use drive::drive::system::core_fork_activations::CoreFork;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Build list of Core transactions from withdrawal documents.
    ///
    /// Version 1 changes on version 0 by emitting asset unlock payload version 2 once Dash
    /// Core's V24 is active. The txid of a version 2 asset unlock leaves out the quorum signing
    /// info, so every instance Platform signs of one withdrawal shares one txid, and Core may
    /// InstantSend-lock it before it is mined. Core refuses version 2 before V24, so the
    /// version follows Drive's record of the Core forks Platform has seen active, read in the
    /// block's `transaction`: `update_core_info` records V24, earlier in the block, at init
    /// chain or in the first block that moves the chain locked height to one where V24 is
    /// active, and forks never switch off, so a recorded V24 is active at the chain locked
    /// height of that block and every later one. Until V24 is recorded, which can be a few
    /// blocks after Core activated it, withdrawals stay at version 1, which Core always
    /// accepts. Core is not asked. A
    /// withdrawal whose whole amount is below Core's dust threshold stays at version 1 (see
    /// `withdrawal_amount_is_core_dust`).
    ///
    /// The version is decided here once and kept in the untied transaction bytes, which every
    /// later signing reuses unchanged. Nothing may rewrite those bytes, nor change how a
    /// signed transaction is built from them, for a withdrawal pooled as version 2: an
    /// InstantSend lock pins its txid, so an instance with another txid could never be mined
    /// while the lock holds.
    pub(super) fn build_untied_withdrawal_transactions_from_documents_v1(
        &self,
        documents: &mut [Document],
        start_index: WithdrawalTransactionIndex,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<WithdrawalTransactionIndexAndBytes>, Credits), Error> {
        if documents.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let dust_relay_fee_per_kb = platform_version
            .system_limits
            .core_dust_relay_fee_per_kb
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "build_untied_withdrawal_transactions_from_documents v1 requires a Core dust relay fee",
            )))?;

        let v24_active = self
            .drive
            .fetch_core_fork_activation_height(CoreFork::V24, transaction, platform_version)?
            .is_some();

        documents.iter_mut().enumerate().try_fold(
            (Vec::new(), 0u64), // Start with an empty vector for transactions and 0 for total amount.
            |(mut transactions, mut total_amount), (i, document)| {
                // Calculate the transaction index.
                let transaction_index = start_index + i as WithdrawalTransactionIndex;

                // Convert the document into the withdrawal transaction information.
                let mut withdrawal_transaction = document
                    .try_into_asset_unlock_base_transaction_info(
                        transaction_index,
                        platform_version,
                    )?;

                if v24_active && !withdrawal_amount_is_core_dust(document, dust_relay_fee_per_kb)? {
                    withdrawal_transaction.base_payload.version =
                        AssetUnlockPayload::STABLE_TXID_VERSION;
                }

                // Create a buffer to hold the encoded transaction.
                let mut transaction_buffer: Vec<u8> = vec![];

                // Get the withdrawal amount from the document properties.
                let amount: u64 = document
                    .properties()
                    .get_integer(withdrawal::properties::AMOUNT)?;

                // Add the amount to the total, checking for overflow.
                total_amount = total_amount.checked_add(amount).ok_or_else(|| {
                    Error::Execution(ExecutionError::Overflow(
                        "Overflow while calculating total amount",
                    ))
                })?;

                // Consensus encode the withdrawal transaction into the buffer.
                withdrawal_transaction
                    .consensus_encode(&mut transaction_buffer)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't consensus encode a withdrawal transaction",
                        ))
                    })?;

                // Update the document properties.
                document.set_u64(withdrawal::properties::TRANSACTION_INDEX, transaction_index);
                document.set_u8(
                    withdrawal::properties::STATUS,
                    withdrawals_contract::WithdrawalStatus::POOLED as u8,
                );
                document.set_updated_at(Some(block_info.time_ms));

                // Increment the document revision, handle error if it fails.
                document.increment_revision().map_err(|_| {
                    Error::Execution(ExecutionError::Overflow(
                        "Overflow when adding to document revision for withdrawals",
                    ))
                })?;

                // Add the transaction index and encoded transaction to the result.
                transactions.push((transaction_index, transaction_buffer));

                // Return the updated accumulator.
                Ok((transactions, total_amount))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::balances::credits::CREDITS_PER_DUFF;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::dashcore::consensus::Decodable;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::AssetUnlockBaseTransactionInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
    use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::Document;
    use dpp::fee::Credits;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::prelude::Identifier;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::version::PlatformVersion;
    use dpp::withdrawal::{
        core_dust_threshold_duffs, Pooling, WithdrawalTransactionIndex,
        WithdrawalTransactionIndexAndBytes,
    };
    use drive::config::DEFAULT_QUERY_LIMIT;
    use drive::drive::identity::withdrawals::paths::get_withdrawal_transactions_broadcasted_path;
    use drive::drive::system::core_fork_activations::{CoreFork, CORE_FORK_V24_ACTIVATION_KEY};
    use drive::drive::system::misc_path;
    use drive::error::{drive::DriveError, Error as DriveErrorKind};
    use drive::grovedb::{Element, TransactionArg};
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};

    /// The chain locked height of the Platform block that pools the withdrawals.
    const CORE_HEIGHT: u32 = 2_000;

    const START_INDEX: WithdrawalTransactionIndex = 50;

    /// The Platform block that pools the withdrawals, at `CORE_HEIGHT`.
    fn block_info() -> BlockInfo {
        BlockInfo {
            time_ms: 1_700_000_000_000,
            height: 10,
            core_height: CORE_HEIGHT,
            epoch: Epoch::default(),
        }
    }

    fn p2pkh() -> CoreScript {
        CoreScript::new_p2pkh([0x11; 20])
    }

    fn p2sh() -> CoreScript {
        CoreScript::new_p2sh([0x22; 20])
    }

    /// A withdrawal document of `amount` credits to `output_script` in `status`, with a
    /// transaction index and a sign height once it has been pooled.
    fn withdrawal_document(
        amount: Credits,
        output_script: &CoreScript,
        status: WithdrawalStatus,
        transaction_index: Option<WithdrawalTransactionIndex>,
        platform_version: &PlatformVersion,
    ) -> Document {
        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        let output_script = output_script.clone();
        let properties = match transaction_index {
            None => platform_value!({
                "amount": amount,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": output_script,
                "status": status as u8,
            }),
            Some(transaction_index) => platform_value!({
                "amount": amount,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": output_script,
                "status": status as u8,
                "transactionIndex": transaction_index,
                "transactionSignHeight": 1_000u64,
            }),
        };
        get_withdrawal_document_fixture(
            &data_contract,
            Identifier::new([7u8; 32]),
            properties,
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document")
    }

    /// Withdrawal documents as the pooling step hands them over: queued, without an index.
    fn queued_withdrawals(
        amounts: &[Credits],
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        amounts
            .iter()
            .map(|amount| {
                withdrawal_document(
                    *amount,
                    &p2pkh(),
                    WithdrawalStatus::QUEUED,
                    None,
                    platform_version,
                )
            })
            .collect()
    }

    /// A platform whose Core answers nothing, so a test fails if it is asked, with the
    /// withdrawals contract's state structure.
    fn platform_without_core() -> TempPlatform<MockCoreRPCLike> {
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        platform.core_rpc = MockCoreRPCLike::new();
        platform
    }

    /// Records V24 at `core_height` in `transaction`, as `update_core_info` does earlier in a
    /// block that sees it active.
    fn record_v24(
        platform: &TempPlatform<MockCoreRPCLike>,
        core_height: u32,
        transaction: TransactionArg,
    ) {
        platform
            .drive
            .store_core_fork_activation_height(
                CoreFork::V24,
                core_height,
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to record V24");
    }

    /// Pools `documents` through the dispatcher at `platform_version`, in a block transaction
    /// holding V24's record at `v24_recorded_at`, or none. Returns the untied transactions,
    /// their total amount and the pooled documents.
    fn build_untied(
        v24_recorded_at: Option<u32>,
        mut documents: Vec<Document>,
        platform_version: &PlatformVersion,
    ) -> (
        Vec<WithdrawalTransactionIndexAndBytes>,
        Credits,
        Vec<Document>,
    ) {
        let platform = platform_without_core();
        let transaction = platform.drive.grove.start_transaction();
        if let Some(core_height) = v24_recorded_at {
            record_v24(&platform, core_height, Some(&transaction));
        }
        let (transactions, total_amount) = platform
            .build_untied_withdrawal_transactions_from_documents(
                &mut documents,
                START_INDEX,
                &block_info(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to build the untied withdrawal transactions");
        (transactions, total_amount, documents)
    }

    /// Decodes the untied transaction bytes the pooling step stores.
    fn decode(untied_transaction_bytes: &[u8]) -> AssetUnlockBaseTransactionInfo {
        AssetUnlockBaseTransactionInfo::consensus_decode(&mut &untied_transaction_bytes[..])
            .expect("expected an untied asset unlock transaction")
    }

    /// A withdrawal pooled once Platform has recorded Core's V24 as active, at the block's
    /// chain locked height or below it, gets payload version 2, and one pooled before gets
    /// version 1; Core is never asked. Apart from the payload version the transactions, their
    /// total and the pooled documents are those version 0 builds.
    #[test]
    fn should_choose_the_asset_unlock_payload_version_from_the_v24_record() {
        let platform_version = PlatformVersion::latest();
        let documents = queued_withdrawals(&[1_000_000, 2_500_000, 70_000_000], platform_version);

        let platform = platform_without_core();
        let mut expected_documents = documents.clone();
        let (expected_transactions, expected_total_amount) = platform
            .build_untied_withdrawal_transactions_from_documents_v0(
                &mut expected_documents,
                START_INDEX,
                &block_info(),
                platform_version,
            )
            .expect("expected to build the untied withdrawal transactions");

        for (v24_recorded_at, payload_version) in [
            (None, 1),
            (Some(CORE_HEIGHT), 2),
            (Some(CORE_HEIGHT - 1_000), 2),
        ] {
            let (transactions, total_amount, pooled_documents) =
                build_untied(v24_recorded_at, documents.clone(), platform_version);

            assert_eq!(total_amount, expected_total_amount);
            assert_eq!(pooled_documents, expected_documents);
            assert_eq!(transactions.len(), expected_transactions.len());
            for ((index, bytes), (expected_index, expected_bytes)) in
                transactions.iter().zip(&expected_transactions)
            {
                assert_eq!(index, expected_index);
                let mut transaction = decode(bytes);
                assert_eq!(
                    transaction.base_payload.version, payload_version,
                    "V24 recorded at {v24_recorded_at:?}"
                );
                transaction.base_payload.version = 1;
                assert_eq!(transaction, decode(expected_bytes));
            }
            if payload_version == 1 {
                assert_eq!(
                    transactions, expected_transactions,
                    "before V24 is recorded the bytes are version 0's"
                );
            }
        }
    }

    /// Protocol version 14 emits payload version 1 whatever the record holds, so the blocks
    /// committed under it replay byte for byte; protocol version 15 emits version 2 for the
    /// same withdrawals once V24 is recorded.
    #[test]
    fn should_keep_emitting_version_1_asset_unlocks_at_protocol_version_14_even_when_v24_is_recorded(
    ) {
        let platform_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        let documents = queued_withdrawals(&[1_000_000, 2_500_000], platform_version_14);

        let (transactions, _, _) =
            build_untied(Some(CORE_HEIGHT), documents.clone(), platform_version_14);
        assert!(transactions
            .iter()
            .all(|(_, bytes)| decode(bytes).base_payload.version == 1));

        let (transactions, _, _) =
            build_untied(Some(CORE_HEIGHT), documents, PlatformVersion::latest());
        assert!(transactions
            .iter()
            .all(|(_, bytes)| decode(bytes).base_payload.version == 2));
    }

    /// A record that cannot be read fails the block instead of being read as V24 not being
    /// recorded, before any document is pooled.
    #[test]
    fn should_fail_rather_than_guess_when_the_v24_record_cannot_be_read() {
        let platform_version = PlatformVersion::latest();
        let documents = queued_withdrawals(&[1_000_000], platform_version);
        let platform = platform_without_core();
        let transaction = platform.drive.grove.start_transaction();
        platform
            .drive
            .grove_insert(
                (&misc_path()).into(),
                CORE_FORK_V24_ACTIVATION_KEY,
                Element::Item(vec![0, 0, 1], None),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to write a malformed record");

        let mut pooled_documents = documents.clone();
        let result = platform.build_untied_withdrawal_transactions_from_documents(
            &mut pooled_documents,
            START_INDEX,
            &block_info(),
            Some(&transaction),
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveErrorKind::Drive(
                    DriveError::CorruptedSerialization(_)
                )))
            ),
            "{result:?}"
        );
        assert_eq!(pooled_documents, documents, "no document may be pooled");
    }

    /// The status the expiry step leaves an expired withdrawal of `amount` credits to
    /// `output_script` in, at `platform_version`: FAILED when it gives up on the withdrawal,
    /// BROADCASTED when it re-signs it.
    fn status_after_expiry(
        amount: Credits,
        output_script: &CoreScript,
        platform_version: &PlatformVersion,
    ) -> WithdrawalStatus {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));
        let document = withdrawal_document(
            amount,
            output_script,
            WithdrawalStatus::EXPIRED,
            Some(START_INDEX),
            platform_version,
        );
        setup_document(
            &platform.drive,
            &document,
            &data_contract,
            data_contract
                .document_type_for_name(withdrawal::NAME)
                .expect("expected to get document type"),
            Some(&transaction),
        );
        platform
            .drive
            .grove_insert_if_not_exists(
                get_withdrawal_transactions_broadcasted_path()
                    .as_slice()
                    .into(),
                &START_INDEX.to_be_bytes(),
                Element::Item(vec![0xAB; 64], None),
                Some(&transaction),
                None,
                &platform_version.drive,
            )
            .expect("expected to store the untied transaction bytes");

        platform
            .rebroadcast_expired_withdrawal_documents(
                &block_info(),
                &platform.state.load(),
                &transaction,
                platform_version,
            )
            .expect("expected to process expired withdrawals");

        let statuses = [WithdrawalStatus::FAILED, WithdrawalStatus::BROADCASTED]
            .into_iter()
            .filter(|status| {
                !platform
                    .drive
                    .fetch_oldest_withdrawal_documents_by_status(
                        (*status).into(),
                        DEFAULT_QUERY_LIMIT,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("expected to fetch withdrawal documents by status")
                    .is_empty()
            })
            .collect::<Vec<_>>();
        assert_eq!(statuses.len(), 1, "{statuses:?}");
        statuses[0]
    }

    /// A withdrawal whose whole amount is below Core's dust threshold for its output script
    /// stays at version 1 while V24 is active. That is the threshold at which an expired
    /// withdrawal is marked FAILED instead of re-signed, so FAILED only ever ends a version 1
    /// withdrawal, which Core never InstantSend-locks.
    #[test]
    fn should_keep_a_dust_withdrawal_at_version_1_even_when_v24_is_active() {
        let platform_version = PlatformVersion::latest();
        let dust_relay_fee_per_kb = platform_version
            .system_limits
            .core_dust_relay_fee_per_kb
            .expect("expected a Core dust relay fee");

        for (output_script, expected_threshold_duffs) in [(p2pkh(), 546), (p2sh(), 540)] {
            let threshold_duffs = core_dust_threshold_duffs(&output_script, dust_relay_fee_per_kb);
            assert_eq!(threshold_duffs, expected_threshold_duffs);
            let dust = (threshold_duffs - 1) * CREDITS_PER_DUFF;
            let viable = threshold_duffs * CREDITS_PER_DUFF;

            let (transactions, _, _) = build_untied(
                Some(CORE_HEIGHT),
                [dust, viable]
                    .iter()
                    .map(|amount| {
                        withdrawal_document(
                            *amount,
                            &output_script,
                            WithdrawalStatus::QUEUED,
                            None,
                            platform_version,
                        )
                    })
                    .collect(),
                platform_version,
            );
            let payload_versions = transactions
                .iter()
                .map(|(_, bytes)| decode(bytes).base_payload.version)
                .collect::<Vec<_>>();
            assert_eq!(
                payload_versions,
                vec![1, 2],
                "{threshold_duffs} duff threshold"
            );

            assert_eq!(
                status_after_expiry(dust, &output_script, platform_version),
                WithdrawalStatus::FAILED
            );
            assert_eq!(
                status_after_expiry(viable, &output_script, platform_version),
                WithdrawalStatus::BROADCASTED
            );
        }
    }
}
