use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_value::Identifier;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds an action to the state
    #[allow(clippy::too_many_arguments)]
    pub fn add_group_action(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
        closes_group_action: bool,
        action_id: Identifier,
        signer_identity_id: Identifier,
        signer_power: GroupMemberPower,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.group.insert.add_group_action {
            0 => self.add_group_action_v0(
                contract_id,
                group_contract_position,
                initialize_with_insert_action_info,
                closes_group_action,
                action_id,
                signer_identity_id,
                signer_power,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_group_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Adds action creation operations to drive operations
    pub fn add_group_action_add_to_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
        closes_group_action: bool,
        action_id: Identifier,
        signer_identity_id: Identifier,
        signer_power: GroupMemberPower,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.group.insert.add_group_action {
            0 => self.add_group_action_add_to_operations_v0(
                contract_id,
                group_contract_position,
                initialize_with_insert_action_info,
                closes_group_action,
                action_id,
                signer_identity_id,
                signer_power,
                block_info,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_group_action_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
    /// The operations needed to create a new group action
    #[allow(clippy::too_many_arguments)]
    pub fn add_group_action_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
        closes_group_action: bool,
        action_id: Identifier,
        signer_identity_id: Identifier,
        signer_power: GroupMemberPower,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.group.insert.add_new_groups {
            0 => self.add_group_action_operations_v0(
                contract_id,
                group_contract_position,
                initialize_with_insert_action_info,
                closes_group_action,
                action_id,
                signer_identity_id,
                signer_power,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_group_action_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
