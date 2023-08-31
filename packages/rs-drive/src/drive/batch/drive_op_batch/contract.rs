use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::drive::flags::StorageFlags;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;

/// Operations on DataContracts
#[derive(Clone, Debug)]
pub enum DataContractOperationType<'a> {
    /// Applies a contract and returns the fee for applying.
    /// If the contract already exists, an update is applied, otherwise an insert.
    ApplyContractWithSerialization {
        /// The contract
        contract: Cow<'a, DataContract>,
        /// The serialized contract
        serialized_contract: Vec<u8>,
        /// Storage flags for the contract
        storage_flags: Option<Cow<'a, StorageFlags>>,
    },
    /// Applies a contract without serialization.
    ApplyContract {
        // this is Cow because we want allow the contract to be owned or not
        // ownership is interesting because you can easily create the contract
        // in sub functions
        // borrowing is interesting because you can create the contract and then
        // use it as borrowed for document insertions
        /// The contract
        contract: Cow<'a, DataContract>,
        /// Storage flags for the contract
        storage_flags: Option<Cow<'a, StorageFlags>>,
    },
}

impl DriveLowLevelOperationConverter for DataContractOperationType<'_> {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            DataContractOperationType::ApplyContractWithSerialization {
                contract,
                serialized_contract: contract_serialization,
                storage_flags,
            } => drive.apply_contract_with_serialization_operations(
                contract.borrow(),
                contract_serialization,
                block_info,
                estimated_costs_only_with_layer_info,
                storage_flags,
                transaction,
                platform_version,
            ),
            DataContractOperationType::ApplyContract {
                contract,
                storage_flags,
            } => drive.apply_contract_operations(
                contract.borrow(),
                block_info,
                estimated_costs_only_with_layer_info,
                storage_flags,
                transaction,
                platform_version,
            ),
        }
    }
}
