use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::borrow::Cow;

mod v0;
mod v1;

/// Drive contract application methods.
impl Drive {
    /// Applies a contract with its serialization.
    ///
    /// This method delegates the contract application to the appropriate versioned method
    /// according to the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be applied.
    /// * `contract_serialization` - The bytes Drive stores the contract as at `platform_version`
    ///   ([`DataContract::serialize_to_storage_bytes`]): from protocol version 15 its compiled
    ///   form (format version 2), not the bytes state transitions carry.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being applied.
    /// * `apply` - A boolean indicating whether the contract should be applied (`true`) or not (`false`).
    /// * `storage_flags` - An optional `Cow<StorageFlags>` containing the storage flags for the contract.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for applying the contract.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract application to use.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - If successful, returns a `FeeResult` representing the fee
    ///   for applying the contract. If an error occurs during the contract application or fee calculation,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract application or fee calculation fails, or if
    /// the provided drive version does not match any known versions.
    ///
    /// Crate-private: the bytes are trusted to be what [`serialize_contract_for_storage`]
    /// gives for `contract`, so only Drive's own writers, which make them with it, pass them.
    ///
    /// [`serialize_contract_for_storage`]: crate::drive::contract::serialize_contract_for_storage
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_contract_with_serialization(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .apply
            .apply_contract_with_serialization
        {
            0 => self.apply_contract_with_serialization_v0(
                contract,
                contract_serialization,
                block_info,
                apply,
                storage_flags,
                transaction,
                platform_version,
            ),
            1 => self.apply_contract_with_serialization_v1(
                contract,
                contract_serialization,
                block_info,
                apply,
                storage_flags,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_contract_with_serialization".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Gets the operations for applying a contract with its serialization.
    ///
    /// This method delegates the operation gathering for applying a contract to the appropriate versioned method
    /// according to the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be applied.
    /// * `contract_serialization` - The bytes Drive stores the contract as at `platform_version`
    ///   ([`DataContract::serialize_to_storage_bytes`]): from protocol version 15 its compiled
    ///   form (format version 2), not the bytes state transitions carry.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being applied.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an `Option` of a `HashMap`
    ///   containing estimated layer information. If provided (`Some`), only the estimated costs will be updated.
    /// * `storage_flags` - An optional `Cow<StorageFlags>` containing the storage flags for the contract.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for applying the contract.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract operations application to use.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - If successful, returns a vector of `LowLevelDriveOperation`
    ///   containing the operations for applying the contract. If an error occurs during the operation gathering,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the operation gathering fails, or if the provided drive version does not
    /// match any known versions.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_contract_with_serialization_operations(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .apply
            .apply_contract_with_serialization
        {
            0 => self.apply_contract_with_serialization_operations_v0(
                contract,
                contract_serialization,
                block_info,
                estimated_costs_only_with_layer_info,
                storage_flags,
                transaction,
                platform_version,
            ),
            1 => self.apply_contract_with_serialization_operations_v1(
                contract,
                contract_serialization,
                block_info,
                estimated_costs_only_with_layer_info,
                storage_flags,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_contract_with_serialization_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::DataContract;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    fn protocol_version_14() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("expected protocol version 14")
    }

    fn fixture_contract(platform_version: &PlatformVersion) -> DataContract {
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned()
    }

    /// The bytes stored for the contract: at its key 0, or the latest revision a contract
    /// keeping history refers to there.
    fn stored_bytes(drive: &Drive, contract: &DataContract) -> Vec<u8> {
        let grove_version = &PlatformVersion::latest().drive.grove_version;
        let id = contract.id_ref().as_bytes();
        let element = match drive
            .grove
            .get_raw((&contract_root_path(id)).into(), &[0], None, grove_version)
            .unwrap()
            .expect("expected the contract to be stored")
        {
            Element::Tree(..) => drive
                .grove
                .get(
                    &contract_keeping_history_root_path(id),
                    &[0],
                    None,
                    grove_version,
                )
                .unwrap()
                .expect("expected the latest revision"),
            element => element,
        };
        match element {
            Element::Item(bytes, _) => bytes,
            other => panic!("expected an item, got {other:?}"),
        }
    }

    fn apply_at(
        drive: &Drive,
        contract: &DataContract,
        time_ms: u64,
        platform_version: &PlatformVersion,
    ) {
        let block_info = BlockInfo {
            time_ms,
            ..Default::default()
        };
        drive
            .apply_contract(contract, block_info, true, None, None, platform_version)
            .expect("expected to apply the contract");
    }

    #[test]
    fn should_not_rewrite_an_unchanged_contract_stored_before_protocol_version_15() {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = fixture_contract(protocol_version_14());
        apply_at(&drive, &contract, 0, protocol_version_14());
        let stored = stored_bytes(&drive, &contract);
        assert_eq!(stored[0], 1);

        apply_at(&drive, &contract, 0, PlatformVersion::latest());
        assert_eq!(stored_bytes(&drive, &contract), stored);
    }

    #[test]
    fn should_not_rewrite_an_unchanged_contract_keeping_history_stored_before_protocol_version_15()
    {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut contract = fixture_contract(protocol_version_14());
        contract.config_mut().set_keeps_history(true);
        apply_at(&drive, &contract, 1_000, protocol_version_14());
        let stored = stored_bytes(&drive, &contract);
        assert_eq!(stored[0], 1);

        // A rewrite at a later time would add a revision
        apply_at(&drive, &contract, 2_000, PlatformVersion::latest());
        assert_eq!(stored_bytes(&drive, &contract), stored);
        let revisions = drive
            .fetch_contract_with_history(
                contract.id().to_buffer(),
                None,
                0,
                None,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the contract's history");
        assert_eq!(revisions.keys().copied().collect::<Vec<_>>(), vec![1_000]);
    }

    #[test]
    fn should_store_a_contract_updated_from_protocol_version_15_compiled() {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut contract = fixture_contract(protocol_version_14());
        apply_at(&drive, &contract, 0, protocol_version_14());
        assert_eq!(stored_bytes(&drive, &contract)[0], 1);

        contract.set_version(2);
        apply_at(&drive, &contract, 0, PlatformVersion::latest());
        assert_eq!(stored_bytes(&drive, &contract)[0], 2);
    }
}
