use crate::drive::group::paths::{
    group_contract_path, group_path, group_root_path, GROUP_ACTIVE_ACTIONS_KEY,
    GROUP_CLOSED_ACTIONS_KEY, GROUP_INFO_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo::{PathFixedSizeKey, PathFixedSizeKeyRef};
use crate::util::object_size_info::{DriveKeyInfo, PathKeyElementInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Adds a group by inserting a new group subtree structure to the `Identities` subtree.
    pub(super) fn add_new_groups_v0(
        &self,
        contract_id: Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.add_new_groups_add_to_operations_v0(
            contract_id,
            groups,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok(fees)
    }

    /// Adds group creation operations to drive operations
    pub(super) fn add_new_groups_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.add_new_groups_operations(
            contract_id,
            groups,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// The operations needed to create a group
    pub(super) fn add_new_groups_operations_v0(
        &self,
        contract_id: Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_add_groups(
                contract_id.to_buffer(),
                groups,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let group_tree_path = group_root_path();

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        // We insert the contract root into the group tree
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKey((group_tree_path, contract_id.to_vec())),
            TreeType::NormalTree,
            None,
            apply_type,
            transaction,
            &mut None,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if inserted {
            for (group_pos, group) in groups {
                let group_pos_bytes = group_pos.to_be_bytes().to_vec();
                let path = group_contract_path(contract_id.as_slice());
                self.batch_insert_empty_tree(
                    path,
                    DriveKeyInfo::Key(group_pos_bytes.clone()),
                    None,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
                let group_path = group_path(contract_id.as_slice(), group_pos_bytes.as_slice());

                let serialized_group_info = group.serialize_to_bytes()?;
                let info_item = Element::Item(serialized_group_info, None);

                self.batch_insert_empty_tree(
                    group_path,
                    DriveKeyInfo::KeyRef(GROUP_ACTIVE_ACTIONS_KEY),
                    None,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                self.batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement::<3>((
                        group_path,
                        GROUP_INFO_KEY,
                        info_item,
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                self.batch_insert_empty_tree(
                    group_path,
                    DriveKeyInfo::KeyRef(GROUP_CLOSED_ACTIONS_KEY),
                    None,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
        } else {
            for (group_pos, group) in groups {
                let group_pos_bytes = group_pos.to_be_bytes().to_vec();
                let path = group_contract_path(contract_id.as_slice());
                let inserted = self.batch_insert_empty_tree_if_not_exists(
                    PathFixedSizeKeyRef((path, group_pos_bytes.as_slice())),
                    TreeType::NormalTree,
                    None,
                    apply_type,
                    transaction,
                    &mut None,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                if inserted {
                    let group_path = group_path(contract_id.as_slice(), group_pos_bytes.as_slice());

                    let serialized_group_info = group.serialize_to_bytes()?;
                    let info_item = Element::Item(serialized_group_info, None);

                    self.batch_insert_empty_tree(
                        group_path,
                        DriveKeyInfo::KeyRef(GROUP_ACTIVE_ACTIONS_KEY),
                        None,
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;

                    self.batch_insert(
                        PathKeyElementInfo::PathFixedSizeKeyRefElement::<3>((
                            group_path,
                            GROUP_INFO_KEY,
                            info_item,
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;

                    self.batch_insert_empty_tree(
                        group_path,
                        DriveKeyInfo::KeyRef(GROUP_CLOSED_ACTIONS_KEY),
                        None,
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;
                }
            }
        }

        Ok(batch_operations)
    }
}
