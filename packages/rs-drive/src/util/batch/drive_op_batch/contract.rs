use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;

use crate::error::drive::DriveError;
use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::borrow::{Borrow, Cow};

/// Operations on DataContracts
#[derive(Clone, Debug)]
pub enum DataContractOperationType<'a> {
    // TODO: split into create and update
    /// Applies a contract: an update if it already exists, otherwise an insert. Drive
    /// serializes it as it stores it at the protocol version, checking the contract size caps.
    ApplyContract {
        // this is Cow because we want to allow the contract to be owned or not
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
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
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

impl DriveOperationFinalizationTasks for DataContractOperationType<'_> {
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .operations
            .contracts
            .finalization_tasks
        {
            0 => self.finalization_tasks_v0(),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DataContractOperationType.finalization_tasks".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl DataContractOperationType<'_> {
    fn finalization_tasks_v0(&self) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        let tasks = match self {
            Self::ApplyContract { contract, .. } => {
                vec![DriveOperationFinalizeTask::RefreshDataContractCache {
                    contract_id: contract.id(),
                }]
            }
        };

        Ok(Some(tasks))
    }
}
