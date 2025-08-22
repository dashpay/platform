use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::prelude::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::{BTreeMap, HashMap};

mod v0;

impl Drive {
    /// Adds groups to the state.
    pub fn add_new_groups(
        &self,
        contract_id: Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.group.insert.add_new_groups {
            0 => self.add_new_groups_v0(
                contract_id,
                groups,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_groups".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds groups creation operations to drive operations
    pub fn add_new_groups_add_to_operations(
        &self,
        contract_id: Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.group.insert.add_new_groups {
            0 => self.add_new_groups_add_to_operations_v0(
                contract_id,
                groups,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_groups_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations needed to create groups
    pub fn add_new_groups_operations(
        &self,
        contract_id: Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.group.insert.add_new_groups {
            0 => self.add_new_groups_operations_v0(
                contract_id,
                groups,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_groups_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
