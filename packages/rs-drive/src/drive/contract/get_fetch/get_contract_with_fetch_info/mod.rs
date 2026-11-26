mod v0;
mod v1;

use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::sync::Arc;

impl Drive {
    /// Retrieves the specified contract.
    ///
    /// The method version is selected based on the `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract.
    /// * `add_to_cache_if_pulled` - A boolean indicating whether to add the fetched contract to the
    ///   cache if it was pulled from storage.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, the function calculates
    ///   the fee for the contract operations.
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<FeeResult>, Option<Arc<DataContractFetchInfo>>), Error>` - If successful,
    ///   returns a tuple containing an `Option` with the `FeeResult` (if an epoch was provided) and
    ///   an `Option` containing an `Arc` to the fetched `ContractFetchInfo`. If an error occurs
    ///   during the contract fetching or fee calculation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching or fee calculation fails or if the
    /// drive version does not match any of the implemented method versions.
    pub fn get_contract_with_fetch_info_and_fee(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<FeeResult>, Option<Arc<DataContractFetchInfo>>), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .get_contract_with_fetch_info
        {
            0 => self.get_contract_with_fetch_info_and_fee_v0(
                contract_id,
                epoch,
                add_to_cache_if_pulled,
                transaction,
                platform_version,
            ),
            1 => self.get_contract_with_fetch_info_and_fee_v1(
                contract_id,
                epoch,
                add_to_cache_if_pulled,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info_and_fee".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Retrieves the specified contract.
    ///
    /// The method version is selected based on the `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract.
    /// * `add_to_cache_if_pulled` - A boolean indicating whether to add the fetched contract to the
    ///   cache if it was pulled from storage.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Arc<DataContractFetchInfo>>, Error>` - If successful, returns an `Option` containing a
    ///   reference to the fetched `Contract`. If an error occurs during the contract fetching,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching fails or if the
    /// drive version does not match any of the implemented method versions.
    pub fn get_contract_with_fetch_info(
        &self,
        contract_id: [u8; 32],
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .get_contract_with_fetch_info
        {
            0 => self.get_contract_with_fetch_info_v0(
                contract_id,
                add_to_cache_if_pulled,
                transaction,
                platform_version,
            ),
            1 => self.get_contract_with_fetch_info_v1(
                contract_id,
                add_to_cache_if_pulled,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Returns the contract with fetch info and operations with the given ID.
    ///
    /// The method version is selected based on the `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract and its fetch info.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, the function calculates
    ///   the fee for the contract operations.
    /// * `add_to_cache_if_pulled` - A boolean indicating whether to add the fetched contract to the
    ///   cache if it was pulled from storage.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Arc<DataContractFetchInfo>>, Error>` - If successful, returns an `Option` containing a
    ///   reference to the fetched `Contract` and related operations. If an error occurs during the contract fetching,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching fails or if the
    /// drive version does not match any of the implemented method versions.
    pub(crate) fn get_contract_with_fetch_info_and_add_to_operations(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .get_contract_with_fetch_info
        {
            0 => self.get_contract_with_fetch_info_and_add_to_operations_v0(
                contract_id,
                epoch,
                add_to_cache_if_pulled,
                transaction,
                drive_operations,
                platform_version,
            ),
            1 => self.get_contract_with_fetch_info_and_add_to_operations_v1(
                contract_id,
                epoch,
                add_to_cache_if_pulled,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info_and_add_to_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::contract::tests::setup_reference_contract_at;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::fee::fee_result::FeeResult;
    use dpp::prelude::Identifier;
    use dpp::tests::json_document::json_document_to_contract;

    use crate::util::batch::{DataContractOperationType, DriveOperation};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::version::PlatformVersion;
    use std::borrow::Cow;
    use std::sync::Arc;

    /// A protocol version selecting each generation: 14 selects generation 0, which chains at
    /// protocol version 14 execute and replay, and the latest selects generation 1.
    fn each_generation() -> [&'static PlatformVersion; 2] {
        [
            PlatformVersion::get(14).expect("expected protocol version 14"),
            PlatformVersion::latest(),
        ]
    }

    #[test]
    fn should_get_contract_from_global_and_block_cache() {
        for platform_version in each_generation() {
            should_get_contract_from_global_and_block_cache_at(platform_version);
        }
    }

    fn should_get_contract_from_global_and_block_cache_at(platform_version: &PlatformVersion) {
        let (drive, mut contract) = setup_reference_contract_at(platform_version);

        let transaction = drive.grove.start_transaction();

        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("should update contract");

        let fetch_info_from_database = drive
            .get_contract_with_fetch_info_and_fee(
                contract.id().to_buffer(),
                None,
                true,
                None,
                platform_version,
            )
            .expect("should get contract")
            .1
            .expect("should be present");

        assert_eq!(fetch_info_from_database.contract.version(), 1);

        let fetch_info_from_cache = drive
            .get_contract_with_fetch_info_and_fee(
                contract.id().to_buffer(),
                None,
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("should get contract")
            .1
            .expect("should be present");

        assert_eq!(fetch_info_from_cache.contract.version(), 2);
    }

    #[test]
    fn should_return_none_if_contract_not_exist() {
        for platform_version in each_generation() {
            should_return_none_if_contract_not_exist_at(platform_version);
        }
    }

    fn should_return_none_if_contract_not_exist_at(platform_version: &PlatformVersion) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let result = drive
            .get_contract_with_fetch_info_and_fee([0; 32], None, true, None, platform_version)
            .expect("should get contract");

        assert!(result.0.is_none());
        assert!(result.1.is_none());
    }

    #[test]
    fn should_return_fees_for_non_existing_contract_if_epoch_is_passed() {
        for platform_version in each_generation() {
            should_return_fees_for_non_existing_contract_if_epoch_is_passed_at(platform_version);
        }
    }

    fn should_return_fees_for_non_existing_contract_if_epoch_is_passed_at(
        platform_version: &PlatformVersion,
    ) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let result = drive
            .get_contract_with_fetch_info_and_fee(
                [0; 32],
                Some(&Epoch::new(0).unwrap()),
                true,
                None,
                platform_version,
            )
            .expect("should get contract");

        assert_eq!(
            result.0,
            Some(FeeResult {
                processing_fee: 2800,
                ..Default::default()
            })
        );

        assert!(result.1.is_none());
    }

    #[test]
    fn should_always_have_then_same_cost() {
        for platform_version in each_generation() {
            should_always_have_then_same_cost_at(platform_version);
        }
    }

    fn should_always_have_then_same_cost_at(platform_version: &PlatformVersion) {
        // Merk trees have own cache and depends on does contract node cached or not
        // we get could get different costs. To avoid of it we fetch contracts without tree caching

        let (drive, mut ref_contract) = setup_reference_contract_at(platform_version);

        /*
         * Firstly, we create multiple contracts during block processing (in transaction)
         */

        let ref_contract_id_buffer = Identifier::from([0; 32]).to_buffer();

        let transaction = drive.grove.start_transaction();

        // Create more contracts to trigger re-balancing
        for i in 0..150u8 {
            ref_contract.set_id(Identifier::from([i; 32]));

            drive
                .apply_contract(
                    &ref_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to apply contract successfully");
        }

        // Create a deep placed contract
        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested10.json";
        let deep_contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get cbor document");
        drive
            .apply_contract(
                &deep_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let mut ref_contract_fetch_info_transactional = drive
            .get_contract_with_fetch_info_and_fee(
                ref_contract_id_buffer,
                Some(&Epoch::new(0).unwrap()),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        let mut deep_contract_fetch_info_transactional = drive
            .get_contract_with_fetch_info_and_fee(
                deep_contract.id().to_buffer(),
                Some(&Epoch::new(0).unwrap()),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        /*
         * Then we commit the block
         */

        // Commit transaction and merge block (transactional) cache to global cache
        transaction.commit().expect("expected to commit");

        drive.cache.data_contracts.merge_and_clear_block_cache();

        /*
         *DataContracts fetched with user query and during block execution must have equal costs
         */

        let deep_contract_fetch_info = drive
            .get_contract_with_fetch_info_and_fee(
                deep_contract.id().to_buffer(),
                None,
                true,
                None,
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        let ref_contract_fetch_info = drive
            .get_contract_with_fetch_info_and_fee(
                ref_contract_id_buffer,
                None,
                true,
                None,
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        assert_eq!(
            deep_contract_fetch_info_transactional,
            deep_contract_fetch_info
        );

        assert_eq!(
            ref_contract_fetch_info_transactional,
            ref_contract_fetch_info
        );

        /*
         * User restarts the node
         */

        // Drop cache so contract will be fetched once again
        drive
            .drop_cache(&platform_version.drive)
            .expect("expected to drop cache");

        /*
         * Other nodes weren't restarted so contracts queried by user after restart
         * must have the same costs as transactional contracts and contracts before
         * restart
         */

        let deep_contract_fetch_info_without_cache = drive
            .get_contract_with_fetch_info_and_fee(
                deep_contract.id().to_buffer(),
                None,
                true,
                None,
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        let ref_contract_fetch_info_without_cache = drive
            .get_contract_with_fetch_info_and_fee(
                ref_contract_id_buffer,
                None,
                true,
                None,
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        // Remove fees to match with fetch with epoch provided
        let deep_contract_fetch_info_transactional_without_arc =
            Arc::make_mut(&mut deep_contract_fetch_info_transactional);

        deep_contract_fetch_info_transactional_without_arc.fee = None;

        let ref_contract_fetch_info_transactional_without_arc =
            Arc::make_mut(&mut ref_contract_fetch_info_transactional);

        ref_contract_fetch_info_transactional_without_arc.fee = None;

        assert_eq!(
            deep_contract_fetch_info_transactional,
            deep_contract_fetch_info_without_cache
        );
        assert_eq!(
            ref_contract_fetch_info_transactional,
            ref_contract_fetch_info_without_cache
        );

        /*
         * Let's imagine that many blocks were executed and the node is restarted again
         */
        drive
            .drop_cache(&platform_version.drive)
            .expect("expected to drop cache");

        /*
         * Drive executes a new block
         */

        let transaction = drive.grove.start_transaction();

        // Create more contracts to trigger re-balancing
        for i in 150..200u8 {
            ref_contract.set_id(Identifier::from([i; 32]));

            drive
                .apply_contract(
                    &ref_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to apply contract successfully");
        }

        /*
         * Other nodes weren't restarted so contracts fetched during block execution
         * should have the same cost as previously fetched contracts
         */

        let mut deep_contract_fetch_info_transactional2 = drive
            .get_contract_with_fetch_info_and_fee(
                deep_contract.id().to_buffer(),
                Some(&Epoch::new(0).unwrap()),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        let mut ref_contract_fetch_info_transactional2 = drive
            .get_contract_with_fetch_info_and_fee(
                ref_contract_id_buffer,
                Some(&Epoch::new(0).unwrap()),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("got contract")
            .1
            .expect("got contract fetch info");

        // Remove fees to match with fetch with epoch provided
        let deep_contract_fetch_info_transactional_without_arc =
            Arc::make_mut(&mut deep_contract_fetch_info_transactional2);

        deep_contract_fetch_info_transactional_without_arc.fee = None;

        let ref_contract_fetch_info_transactional_without_arc =
            Arc::make_mut(&mut ref_contract_fetch_info_transactional2);

        ref_contract_fetch_info_transactional_without_arc.fee = None;

        assert_eq!(
            ref_contract_fetch_info_transactional,
            ref_contract_fetch_info_transactional2,
        );

        assert_eq!(
            deep_contract_fetch_info_transactional,
            deep_contract_fetch_info_transactional2
        );
    }

    /// The ordinary contract update path, end to end at the Drive level. The block applies an
    /// update through the drive operation batch, whose finalization task re-seeds the cache
    /// from the transaction. A concurrent committed-state query (what a public `getDocuments`
    /// does on a validator) then puts the committed, pre-update definition into the global
    /// cache. The block's next transactional read must still resolve to the update, whether
    /// or not the seeded copy is still resident, and the query's copy must never outlive the
    /// commit.
    #[test]
    fn should_resolve_a_contract_updated_in_the_block_after_a_committed_query_refills_the_global_cache(
    ) {
        for platform_version in each_generation() {
            resolve_an_updated_contract_after_a_query_refill_at(platform_version);
        }
    }

    fn resolve_an_updated_contract_after_a_query_refill_at(platform_version: &PlatformVersion) {
        let (drive, contract) = setup_reference_contract_at(platform_version);
        let contract_id = contract.id().to_buffer();
        let epoch = Epoch::new(0).expect("epoch 0");

        let transaction = drive.grove.start_transaction();

        // The block's first read of the contract, as the update's validation performs it.
        let before = drive
            .get_contract_with_fetch_info_and_fee(
                contract_id,
                Some(&epoch),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the read to succeed")
            .1
            .expect("expected the contract");
        assert_eq!(before.contract.version(), 1);

        let mut updated = contract.clone();
        updated.increment_version();
        drive
            .apply_drive_operations(
                vec![DriveOperation::DataContractOperation(
                    DataContractOperationType::ApplyContract {
                        contract: Cow::Borrowed(&updated),
                        storage_flags: None,
                    },
                )],
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to apply the update");

        // The finalization task seeded the block cache with what the transaction now holds
        // and recorded the rewrite.
        assert_eq!(
            drive
                .cache
                .data_contracts
                .get(contract_id, true)
                .expect("the block cache must hold the updated contract")
                .contract
                .version(),
            2
        );
        assert!(drive.cache.data_contracts.is_modified_in_block(contract_id));

        // The concurrent committed-state query.
        let committed = drive
            .get_contract_with_fetch_info(contract_id, true, None, platform_version)
            .expect("expected the query to succeed")
            .expect("expected the committed contract");
        assert_eq!(
            committed.contract.version(),
            1,
            "precondition: the query reads committed state, which still holds the pre-update contract"
        );

        // The block's next read, as a second update or a document transition performs it.
        let after = drive
            .get_contract_with_fetch_info_and_fee(
                contract_id,
                Some(&epoch),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the read to succeed")
            .1
            .expect("expected the contract");
        assert_eq!(after.contract.version(), 2);

        // And once more with the seeded copy gone.
        drive.cache.data_contracts.drop_block_modified_entries();
        let after = drive
            .get_contract_with_fetch_info_and_fee(
                contract_id,
                Some(&epoch),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the read to succeed")
            .1
            .expect("expected the contract");
        assert_eq!(
            after.contract.version(),
            2,
            "a transactional read must never fall back to the committed copy of a contract the block rewrote"
        );

        // Commit and promote. A query that read the pre-update contract before the commit
        // and only now performs its insert must not publish it.
        let observed_before_commit = drive.cache.data_contracts.committed_generation();
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
        drive.cache.data_contracts.merge_and_clear_block_cache();
        drive
            .cache
            .data_contracts
            .insert_committed(committed, observed_before_commit);

        let published = drive
            .get_contract_with_fetch_info(contract_id, true, None, platform_version)
            .expect("expected the query to succeed")
            .expect("expected the contract");
        assert_eq!(
            published.contract.version(),
            2,
            "a committed-state reader must resolve to the committed update after the promotion"
        );
    }

    /// Estimating a batch writes nothing, so it must not record a rewrite either.
    #[test]
    fn should_not_record_a_rewrite_when_only_estimating_a_batch() {
        for platform_version in each_generation() {
            should_not_record_a_rewrite_when_only_estimating_a_batch_at(platform_version);
        }
    }

    fn should_not_record_a_rewrite_when_only_estimating_a_batch_at(
        platform_version: &PlatformVersion,
    ) {
        let (drive, contract) = setup_reference_contract_at(platform_version);
        let contract_id = contract.id().to_buffer();

        let transaction = drive.grove.start_transaction();

        let mut updated = contract.clone();
        updated.increment_version();
        drive
            .apply_drive_operations(
                vec![DriveOperation::DataContractOperation(
                    DataContractOperationType::ApplyContract {
                        contract: Cow::Borrowed(&updated),
                        storage_flags: None,
                    },
                )],
                false,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to estimate the update");

        assert!(!drive.cache.data_contracts.is_modified_in_block(contract_id));
        assert_eq!(
            drive
                .get_contract_with_fetch_info(
                    contract_id,
                    false,
                    Some(&transaction),
                    platform_version
                )
                .expect("expected the read to succeed")
                .expect("expected the contract")
                .contract
                .version(),
            1
        );
    }

    /// A contract updated in the block is afterwards served from the block cache, seeded by
    /// the update's refresh. Billing that read from the cost recorded at the refresh must
    /// charge what a cold read of the contract through the same transaction charges (which is
    /// what a node that evicted the contract on update used to bill), even when documents were
    /// written under the contract and other contracts rebalanced the contracts tree between
    /// the refresh and the read. Run at the shipped protocol version 13 and at the latest.
    #[test]
    fn should_bill_a_block_cache_hit_after_an_update_like_a_cold_read() {
        for platform_version in [
            PlatformVersion::get(13).expect("expected protocol version 13"),
            PlatformVersion::latest(),
        ] {
            let drive = setup_drive_with_initial_state_structure(None);
            let contract = json_document_to_contract(
                "tests/supporting_files/contract/references/references.json",
                false,
                platform_version,
            )
            .expect("expected to get a contract");
            drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");
            let contract_id = contract.id().to_buffer();
            let epoch = Epoch::new(0).expect("epoch 0");

            let transaction = drive.grove.start_transaction();

            let mut updated = contract.clone();
            updated.increment_version();
            drive
                .apply_drive_operations(
                    vec![DriveOperation::DataContractOperation(
                        DataContractOperationType::ApplyContract {
                            contract: Cow::Borrowed(&updated),
                            storage_flags: None,
                        },
                    )],
                    true,
                    &BlockInfo::default(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to apply the update");
            assert!(drive.cache.data_contracts.is_modified_in_block(contract_id));
            // The entry the refresh seeded, with the cost recorded before the writes below.
            let seeded = drive
                .cache
                .data_contracts
                .get(contract_id, true)
                .expect("the refresh must seed the block cache");

            // Documents written under the contract after the refresh.
            let note_type = updated
                .document_type_for_name("note")
                .expect("expected the note type");
            for seed in 0..10u64 {
                let note = note_type
                    .random_document(Some(seed), platform_version)
                    .expect("expected a random note");
                drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentRefInfo((&note, None)),
                                owner_id: None,
                            },
                            contract: &updated,
                            document_type: note_type,
                        },
                        false,
                        BlockInfo::default(),
                        true,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("expected to insert a note");
            }

            // Other contracts that rebalance the contracts tree after the refresh.
            let mut other = contract.clone();
            for i in 100..140u8 {
                other.set_id(Identifier::from([i; 32]));
                drive
                    .apply_contract(
                        &other,
                        BlockInfo::default(),
                        true,
                        StorageFlags::optional_default_as_cow(),
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("expected to apply contract successfully");
            }

            // The billed read of the updated contract, served from the block cache.
            let (hit_fee, hit) = drive
                .get_contract_with_fetch_info_and_fee(
                    contract_id,
                    Some(&epoch),
                    true,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the read to succeed");
            let hit = hit.expect("expected the contract");
            assert_eq!(hit.contract.version(), 2);
            // The read must be served by the entry the refresh seeded, not by a cold fetch that
            // would make this test compare two cold reads.
            assert!(
                Arc::ptr_eq(&seeded, &hit),
                "protocol version {}: the billed read must be the refresh-seeded cache hit",
                platform_version.protocol_version
            );

            // A cold read of the same contract through the same transaction.
            let cold = drive
                .fetch_contract_and_add_operations(
                    contract_id,
                    Some(&epoch),
                    Some(&transaction),
                    &mut vec![],
                    platform_version,
                )
                .expect("expected the read to succeed")
                .expect("expected the contract");

            assert_eq!(
                hit.cost, cold.cost,
                "protocol version {}: the cached cost must equal a cold read's cost",
                platform_version.protocol_version
            );
            assert_eq!(
                hit_fee, cold.fee,
                "protocol version {}: the cached read must bill like a cold read",
                platform_version.protocol_version
            );
        }
    }

    /// Readers at different protocol versions sharing the cache. A query thread reads
    /// committed state at the protocol version of the platform state it loaded, which on the
    /// first block of a new version (and for a query that loaded it just before) is still the
    /// previous one, and a block reads at its own version. Both use the cache.
    mod parse_version {
        use crate::drive::contract::DataContractFetchInfo;
        use crate::drive::Drive;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use crate::util::test_helpers::setup_contract;
        use dpp::block::epoch::Epoch;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
        use dpp::data_contract::DataContract;
        use dpp::fee::fee_result::FeeResult;
        use dpp::version::PlatformVersion;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::{Duration, Instant};

        /// Its `tip` type sums a property bounded past 2^27, so it keeps its sums in 128 bits
        /// from protocol version 15 and in 64 bits before: the two versions read the stored
        /// contract differently.
        const TIP_JAR: &str = "tests/supporting_files/contract/tip-jar/tip-jar-contract.json";

        fn protocol_version_14() -> &'static PlatformVersion {
            PlatformVersion::get(14).expect("expected protocol version 14")
        }

        /// A drive holding the tip jar, stored at protocol version 14.
        fn drive_with_tip_jar_stored_at_14() -> (Drive, [u8; 32]) {
            let drive = setup_drive_with_initial_state_structure(Some(protocol_version_14()));
            let contract = setup_contract(
                &drive,
                TIP_JAR,
                None,
                None,
                None::<fn(&mut DataContract)>,
                None,
                Some(protocol_version_14()),
            );
            let contract_id = contract.id().to_buffer();
            (drive, contract_id)
        }

        fn keeps_big_sums(fetch_info: &DataContractFetchInfo) -> bool {
            fetch_info
                .contract
                .document_type_for_name("tip")
                .expect("expected the tip type")
                .uses_big_sums()
        }

        /// A block at protocol version 15 reads a contract it has not rewritten, after a query
        /// still at protocol version 14 cached its reading of it: on the first block of the
        /// version, or later after the promoted entry was evicted. The block must get the
        /// reading of 15, as a node with an empty cache does; served the cached one, it would
        /// write the tip type's sums in 64 bits where every other node writes 128.
        #[test]
        fn should_not_serve_a_block_at_protocol_version_15_the_reading_a_protocol_version_14_query_cached(
        ) {
            let (drive, contract_id) = drive_with_tip_jar_stored_at_14();
            let platform_version = PlatformVersion::latest();
            let epoch = Epoch::new(0).expect("epoch 0");

            let cold = drive
                .fetch_contract(contract_id, None, None, None, platform_version)
                .value
                .expect("expected the fetch to succeed")
                .expect("expected the contract");
            assert!(
                keeps_big_sums(&cold),
                "precondition: protocol version 15 reads the tip type with 128-bit sums"
            );

            drive
                .get_contract_with_fetch_info(contract_id, true, None, protocol_version_14())
                .expect("expected the query to succeed")
                .expect("expected the contract");
            let cached = drive
                .cache
                .data_contracts
                .get(contract_id, false)
                .expect("precondition: the query cached its reading");
            assert!(
                !keeps_big_sums(&cached),
                "precondition: protocol version 14 reads the tip type with 64-bit sums"
            );
            // The query's copy survives the promotion of a block that did not read the
            // contract.
            drive.cache.data_contracts.merge_and_clear_block_cache();
            drive.cache.data_contracts.clear_block_cache();
            assert!(
                Arc::ptr_eq(
                    &drive
                        .cache
                        .data_contracts
                        .get(contract_id, true)
                        .expect("precondition: the query's copy is still cached"),
                    &cached
                ),
                "precondition: the block's read finds the query's copy in the cache"
            );

            let transaction = drive.grove.start_transaction();
            let (_, read) = drive
                .get_contract_with_fetch_info_and_fee(
                    contract_id,
                    Some(&epoch),
                    true,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the read to succeed");
            let read = read.expect("expected the contract");
            assert!(
                keeps_big_sums(&read),
                "a block at protocol version 15 must read the contract as protocol version 15 does"
            );
            assert_eq!(read.contract, cold.contract);
        }

        /// Sets the flag when dropped, also while unwinding, so the query thread of the test
        /// below stops whatever fails and the thread scope can join it.
        struct StopOnDrop<'a>(&'a AtomicBool);

        impl Drop for StopOnDrop<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        /// The same race with real threads: a query thread at protocol version 14 keeps
        /// reading the contract, and caches its reading whenever the cache has none, while
        /// blocks at protocol version 15 run on this thread. Before each block the contract's
        /// entry is evicted, as the first block of a version empties the cache or capacity
        /// pressure drops an entry, and the block waits until the query has cached its reading
        /// again, the interleaving the race produces, so the test fails on every run when a
        /// block is served that reading. The block then reads the contract through its
        /// transaction, commits and promotes its block cache while the query thread runs. Every
        /// block read must be the reading of 15.
        #[test]
        fn should_never_serve_a_block_at_protocol_version_15_a_reading_a_concurrent_protocol_version_14_query_caches(
        ) {
            const BLOCKS: usize = 20;
            const WAIT_FOR_THE_QUERY: Duration = Duration::from_secs(10);

            let (drive, contract_id) = drive_with_tip_jar_stored_at_14();
            let platform_version = PlatformVersion::latest();
            let stop = AtomicBool::new(false);

            let block_reads = thread::scope(|scope| {
                let _stop_the_query = StopOnDrop(&stop);
                scope.spawn(|| {
                    let _stop_the_blocks = StopOnDrop(&stop);
                    while !stop.load(Ordering::SeqCst) {
                        drive
                            .get_contract_with_fetch_info(
                                contract_id,
                                true,
                                None,
                                protocol_version_14(),
                            )
                            .expect("expected the query to succeed")
                            .expect("expected the contract");
                    }
                });

                let mut block_reads = Vec::with_capacity(BLOCKS);
                for _ in 0..BLOCKS {
                    drive.cache.data_contracts.clear_block_cache();
                    drive.cache.data_contracts.remove(contract_id);

                    let waiting_since = Instant::now();
                    while drive.cache.data_contracts.get(contract_id, false).is_none() {
                        assert!(
                            !stop.load(Ordering::SeqCst),
                            "the query thread stopped: it failed to read the contract"
                        );
                        assert!(
                            waiting_since.elapsed() < WAIT_FOR_THE_QUERY,
                            "the query thread did not cache its reading in time"
                        );
                        thread::yield_now();
                    }

                    let transaction = drive.grove.start_transaction();
                    let read = drive
                        .get_contract_with_fetch_info(
                            contract_id,
                            true,
                            Some(&transaction),
                            platform_version,
                        )
                        .expect("expected the read to succeed")
                        .expect("expected the contract");
                    block_reads.push(keeps_big_sums(&read));
                    drive
                        .grove
                        .commit_transaction(transaction)
                        .unwrap()
                        .expect("expected to commit");
                    drive.cache.data_contracts.merge_and_clear_block_cache();
                }
                block_reads
            });

            assert_eq!(
                block_reads,
                vec![true; BLOCKS],
                "every block at protocol version 15 must read the contract as protocol version 15 does"
            );
        }

        /// Generation 0, which protocol version 14 selects, still serves a cached copy whatever
        /// version parsed it, as the blocks of 14 and earlier were executed; generation 1,
        /// which protocol version 15 selects, does not.
        #[test]
        fn should_check_the_protocol_version_of_a_cached_copy_only_from_generation_1() {
            let (drive, contract_id) = drive_with_tip_jar_stored_at_14();
            let platform_version = PlatformVersion::latest();

            // A copy parsed under protocol version 15 in the cache.
            let parsed_at_15 = drive
                .get_contract_with_fetch_info(contract_id, true, None, platform_version)
                .expect("expected the read to succeed")
                .expect("expected the contract");
            assert!(keeps_big_sums(&parsed_at_15));

            let transaction = drive.grove.start_transaction();
            let read_at_14 = drive
                .get_contract_with_fetch_info(
                    contract_id,
                    false,
                    Some(&transaction),
                    protocol_version_14(),
                )
                .expect("expected the read to succeed")
                .expect("expected the contract");
            assert!(
                Arc::ptr_eq(&read_at_14, &parsed_at_15),
                "generation 0 serves the cached copy as it always did"
            );

            // A copy parsed under protocol version 14 in the cache.
            drive.cache.data_contracts.clear();
            let parsed_at_14 = drive
                .get_contract_with_fetch_info(contract_id, true, None, protocol_version_14())
                .expect("expected the read to succeed")
                .expect("expected the contract");
            assert!(
                Arc::ptr_eq(
                    &drive
                        .cache
                        .data_contracts
                        .get(contract_id, true)
                        .expect("precondition: the copy parsed under 14 is cached"),
                    &parsed_at_14
                ),
                "precondition: the read at 15 finds the copy parsed under 14 in the cache"
            );
            let read_at_15 = drive
                .get_contract_with_fetch_info(
                    contract_id,
                    false,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the read to succeed")
                .expect("expected the contract");
            assert!(!Arc::ptr_eq(&read_at_15, &parsed_at_14));
            assert!(keeps_big_sums(&read_at_15));
        }

        /// A cache entry can carry a fee that a reader of another protocol version priced
        /// under its own schedule: generation 0 stores the fee it bills a cached copy, whatever
        /// version parsed that copy. A block at protocol version 15 that hits the entry must
        /// bill what a cold read bills, never the stored fee.
        #[test]
        fn should_bill_a_cache_hit_at_protocol_version_15_what_a_cold_read_bills_whatever_fee_the_entry_carries(
        ) {
            let (drive, contract_id) = drive_with_tip_jar_stored_at_14();
            let platform_version = PlatformVersion::latest();
            let epoch = Epoch::new(0).expect("epoch 0");

            let transaction = drive.grove.start_transaction();
            let cold = drive
                .fetch_contract_and_add_operations(
                    contract_id,
                    Some(&epoch),
                    Some(&transaction),
                    &mut vec![],
                    platform_version,
                )
                .expect("expected the read to succeed")
                .expect("expected the contract");
            let cold_fee = cold.fee.clone().expect("a read with an epoch has a fee");

            let mut priced_elsewhere = (*cold).clone();
            priced_elsewhere.fee = Some(FeeResult::new_from_processing_fee(1));
            assert_ne!(priced_elsewhere.fee, Some(cold_fee.clone()));
            drive.cache.data_contracts.insert_committed(
                Arc::new(priced_elsewhere),
                drive.cache.data_contracts.committed_generation(),
            );

            let mut operations = vec![];
            let hit = drive
                .get_contract_with_fetch_info_and_add_to_operations(
                    contract_id,
                    Some(&epoch),
                    true,
                    Some(&transaction),
                    &mut operations,
                    platform_version,
                )
                .expect("expected the read to succeed")
                .expect("expected the contract");
            assert!(
                hit.fee == Some(FeeResult::new_from_processing_fee(1)),
                "precondition: the read is the cache hit"
            );
            let billed = drive
                .calculate_fee(None, Some(operations), &epoch, platform_version)
                .expect("expected the fee");
            assert_eq!(
                billed, cold_fee,
                "a cache hit must bill what a cold read bills"
            );
        }
    }
}
