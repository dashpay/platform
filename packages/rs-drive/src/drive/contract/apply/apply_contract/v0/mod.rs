use crate::drive::contract::serialize_contract_for_storage;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

impl Drive {
    /// Applies a contract and returns the fee for applying.
    ///
    /// This function applies a given contract to the storage. If the contract already exists,
    /// an update is performed; otherwise, a new contract is inserted. The fee for applying
    /// the contract is also calculated and returned.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `Contract` to be applied.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being applied.
    /// * `apply` - A boolean indicating whether the contract should be applied (`true`) or not (`false`).
    /// * `storage_flags` - An optional `Cow<StorageFlags>` containing the storage flags for the contract.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for applying the contract.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - If successful, returns a `FeeResult` representing the fee
    ///   for applying the contract. If an error occurs during the contract application or fee calculation,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract application or fee calculation fails.
    #[inline(always)]
    pub(super) fn apply_contract_v0(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        self.apply_contract_with_serialization(
            contract,
            // Edited in place: every protocol version selects this generation; through 14 it
            // stores the bytes it serialized before, and from 15 it stores contracts compiled
            // (format version 2, v15 note 8). The size caps checked with them refuse nothing it
            // is called with in a block: only system contracts reach it there, and each fits the
            // caps at every version that has it (dpp's system contract size test)
            serialize_contract_for_storage(contract, platform_version)?,
            block_info,
            apply,
            storage_flags,
            transaction,
            platform_version,
        )
    }

    /// Gets the operations for applying a contract
    /// If the contract already exists, we get operations for an update
    /// Otherwise we get operations for an insert
    #[inline(always)]
    pub(super) fn apply_contract_operations_v0(
        &self,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        // Edited in place: every protocol version selects this generation; through 14 it
        // stores and caps the bytes it serialized before, and from 15 it stores contracts
        // compiled (format version 2, v15 note 8)
        let serialized_contract = serialize_contract_for_storage(contract, platform_version)?;

        self.apply_contract_with_serialization_operations(
            contract,
            serialized_contract,
            block_info,
            estimated_costs_only_with_layer_info,
            storage_flags,
            transaction,
            platform_version,
        )
    }
}
