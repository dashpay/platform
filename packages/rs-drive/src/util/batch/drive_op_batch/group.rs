use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Group operations requiring many people to agree to something for the action to occur.
#[derive(Clone, Debug)]
pub enum GroupOperationType {
    /// Adds a group action
    AddGroupAction {
        /// The contract id
        contract_id: Identifier,
        /// The group contract position
        group_contract_position: GroupContractPosition,
        /// Initialize with the action info
        initialize_with_insert_action_info: Option<GroupAction>,
        /// The action id
        action_id: Identifier,
        /// The identity that is signing
        signer_identity_id: Identifier,
        /// The signer's power in the group
        signer_power: GroupMemberPower,
        /// Should we close the group action and mark it as complete?
        closes_group_action: bool,
    },
}

impl DriveLowLevelOperationConverter for GroupOperationType {
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
            GroupOperationType::AddGroupAction {
                contract_id,
                group_contract_position,
                initialize_with_insert_action_info,
                action_id,
                signer_identity_id,
                signer_power,
                closes_group_action,
            } => drive.add_group_action_operations(
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
        }
    }
}
