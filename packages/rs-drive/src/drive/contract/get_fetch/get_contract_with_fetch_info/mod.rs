mod v0;

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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info_and_fee".to_string(),
                known_versions: vec![0],
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info".to_string(),
                known_versions: vec![0],
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info_and_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::contract::tests::setup_reference_contract;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::fee::fee_result::FeeResult;
    use dpp::prelude::Identifier;
    use dpp::tests::json_document::json_document_to_contract;

    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    #[test]
    fn should_get_contract_from_global_and_block_cache() {
        let (drive, mut contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
                None,
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
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let result = drive
            .get_contract_with_fetch_info_and_fee([0; 32], None, true, None, platform_version)
            .expect("should get contract");

        assert!(result.0.is_none());
        assert!(result.1.is_none());
    }

    #[test]
    fn should_return_fees_for_non_existing_contract_if_epoch_is_passed() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

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
        // Merk trees have own cache and depends on does contract node cached or not
        // we get could get different costs. To avoid of it we fetch contracts without tree caching

        let (drive, mut ref_contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

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
}
