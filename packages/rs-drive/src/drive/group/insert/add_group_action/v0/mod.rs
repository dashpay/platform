use crate::drive::group::paths::{
    group_action_signers_path_vec, group_active_action_path, group_active_action_root_path,
    group_closed_action_path, group_closed_action_path_vec, group_closed_action_root_path,
    group_closed_action_signers_path_vec, ACTION_INFO_KEY, ACTION_SIGNERS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{
    BatchDeleteApplyType, BatchInsertApplyType, BatchInsertTreeApplyType, BatchMoveApplyType,
    QueryTarget,
};
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use crate::util::object_size_info::{DriveKeyInfo, PathKeyElementInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::element::SumValue;
use grovedb::MaybeTree::NotTree;
use grovedb::{Element, EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use grovedb_epoch_based_storage_flags::StorageFlags;
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_group_action_v0(
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
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.add_group_action_add_to_operations_v0(
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

    #[allow(clippy::too_many_arguments)]
    /// Adds group creation operations to drive operations
    pub(super) fn add_group_action_add_to_operations_v0(
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
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.add_group_action_operations(
            contract_id,
            group_contract_position,
            initialize_with_insert_action_info,
            closes_group_action,
            action_id,
            signer_identity_id,
            signer_power,
            block_info,
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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_group_action_operations_v0(
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
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_add_group_action(
                contract_id.to_buffer(),
                group_contract_position,
                Some(action_id.to_buffer()),
                closes_group_action,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();

        if !closes_group_action {
            // We are not closing the group action, this means for example that the required power
            // is 5, and we are getting to 2.
            let group_action_root_path = group_active_action_root_path(
                contract_id.as_slice(),
                group_contract_position_bytes.as_slice(),
            );
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: TreeType::NormalTree,
                    tree_type: TreeType::NormalTree,
                    flags_len: 0,
                }
            };

            let storage_flags = Some(StorageFlags::new_single_epoch(
                block_info.epoch.index,
                Some(signer_identity_id.to_buffer()),
            ));

            let element_flags = StorageFlags::map_to_some_element_flags(storage_flags.as_ref());

            let mut inserted_root_action = false;

            if let Some(initialize_with_insert_action_info) = initialize_with_insert_action_info {
                // We insert the contract root into the group tree
                inserted_root_action = self.batch_insert_empty_tree_if_not_exists(
                    PathFixedSizeKeyRef((group_action_root_path, action_id.as_slice())),
                    TreeType::NormalTree,
                    None,
                    apply_type,
                    transaction,
                    &mut None,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                if inserted_root_action {
                    let group_action_path = group_active_action_path(
                        contract_id.as_slice(),
                        group_contract_position_bytes.as_slice(),
                        action_id.as_slice(),
                    );

                    self.batch_insert_empty_sum_tree(
                        group_action_path,
                        DriveKeyInfo::KeyRef(ACTION_SIGNERS_KEY),
                        None,
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;

                    let serialized =
                        initialize_with_insert_action_info.serialize_consume_to_bytes()?;

                    self.batch_insert(
                        PathKeyElementInfo::PathFixedSizeKeyRefElement::<5>((
                            group_action_path,
                            ACTION_INFO_KEY,
                            Element::Item(serialized, element_flags.clone()),
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;
                }
            }

            let signers_path = group_action_signers_path_vec(
                contract_id.as_slice(),
                group_contract_position,
                action_id.as_slice(),
            );

            let signer_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertApplyType::StatefulBatchInsert
            } else {
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: TreeType::SumTree,
                    target: QueryTarget::QueryTargetValue(8),
                }
            };

            if inserted_root_action {
                self.batch_insert(
                    PathKeyElementInfo::PathKeyElement::<0>((
                        signers_path,
                        signer_identity_id.to_vec(),
                        Element::SumItem(signer_power as SumValue, element_flags),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            } else {
                // we should verify it doesn't yet exist
                self.batch_insert_sum_item_if_not_exists(
                    PathKeyElementInfo::PathKeyElement::<0>((
                        signers_path,
                        signer_identity_id.to_vec(),
                        Element::SumItem(signer_power as SumValue, element_flags),
                    )),
                    true,
                    signer_apply_type,
                    transaction,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
        } else {
            // We are closing the group action
            // This means for example that the required power is 5, and we are getting to 5 or above.
            let group_action_root_path = group_closed_action_root_path(
                contract_id.as_slice(),
                group_contract_position_bytes.as_slice(),
            );
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: TreeType::NormalTree,
                    tree_type: TreeType::NormalTree,
                    flags_len: 0,
                }
            };

            let inserted = self.batch_insert_empty_tree_if_not_exists(
                PathFixedSizeKeyRef((group_action_root_path, action_id.as_slice())),
                TreeType::NormalTree,
                None,
                apply_type,
                transaction,
                &mut None,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            if !inserted {
                // This is an error because we should always be closing things
                return Err(Error::Drive(DriveError::InvalidAction("inserting a group action as closed should always create a new closed action tree for that action id")));
            }

            let group_closed_action_path = group_closed_action_path(
                contract_id.as_slice(),
                group_contract_position_bytes.as_slice(),
                action_id.as_slice(),
            );

            self.batch_insert_empty_sum_tree(
                group_closed_action_path,
                DriveKeyInfo::KeyRef(ACTION_SIGNERS_KEY),
                None,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            let closed_signers_path = group_closed_action_signers_path_vec(
                contract_id.as_slice(),
                group_contract_position,
                action_id.as_slice(),
            );

            self.batch_insert(
                PathKeyElementInfo::PathKeyElement::<0>((
                    closed_signers_path.clone(),
                    signer_identity_id.to_vec(),
                    Element::SumItem(signer_power as SumValue, None),
                )),
                &mut batch_operations,
                &platform_version.drive,
            )?;
            // now we need to also move all signers from the active tree to the closed tree

            let mut active_path_query = Drive::group_action_signers_query(
                contract_id.to_buffer(),
                group_contract_position,
                GroupActionStatus::ActionActive,
                action_id.to_buffer(),
            );

            active_path_query.query.limit =
                Some(platform_version.system_limits.max_contract_group_size);

            let signer_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchMoveApplyType::StatefulBatchMove {
                    is_known_to_be_subtree_with_sum: Some(NotTree),
                }
            } else {
                BatchMoveApplyType::StatelessBatchMove {
                    in_tree_type: TreeType::SumTree,
                    tree_type: None,
                    estimated_key_size: 32,
                    estimated_value_size: 8,
                    flags_len: 0,
                }
            };

            self.batch_move_items_in_path_query(
                &active_path_query,
                closed_signers_path.clone(),
                false, // maybe there were no active signers
                signer_apply_type,
                Some(None), // we should have no flags on the closed items
                transaction,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            if let Some(initialize_with_insert_action_info) = initialize_with_insert_action_info {
                let serialized = initialize_with_insert_action_info.serialize_consume_to_bytes()?;

                self.batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement::<5>((
                        group_closed_action_path,
                        ACTION_INFO_KEY,
                        Element::Item(serialized, None),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            } else {
                // we should move the info from the active state to the closed state
                let info_move_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    BatchMoveApplyType::StatefulBatchMove {
                        is_known_to_be_subtree_with_sum: Some(NotTree),
                    }
                } else {
                    BatchMoveApplyType::StatelessBatchMove {
                        in_tree_type: TreeType::NormalTree,
                        tree_type: None,
                        estimated_key_size: 1,
                        estimated_value_size: 3,
                        flags_len: 0,
                    }
                };

                let group_active_action_root_path = group_active_action_root_path(
                    contract_id.as_slice(),
                    group_contract_position_bytes.as_slice(),
                );

                let group_active_action_path = group_active_action_path(
                    contract_id.as_slice(),
                    group_contract_position_bytes.as_slice(),
                    action_id.as_slice(),
                );

                let group_closed_action_path_vec = group_closed_action_path_vec(
                    contract_id.as_slice(),
                    group_contract_position,
                    action_id.as_slice(),
                );

                self.batch_move(
                    group_active_action_path.as_slice().into(), // from active path
                    ACTION_INFO_KEY,                            // key to move
                    group_closed_action_path_vec,               // to closed path
                    info_move_apply_type,
                    Some(None), // no flags on the closed item
                    transaction,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                // We then need to then do a delete operation of the action in the active part

                let delete_apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(
                            TreeType::NormalTree,
                        )),
                    }
                } else {
                    BatchDeleteApplyType::StatelessBatchDelete {
                        in_tree_type: TreeType::NormalTree,
                        estimated_key_size: 32,
                        estimated_value_size: 32,
                    }
                };

                self.batch_delete(
                    group_active_action_path.as_slice().into(),
                    ACTION_SIGNERS_KEY,
                    delete_apply_type,
                    transaction,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                self.batch_delete(
                    group_active_action_root_path.as_slice().into(),
                    action_id.as_slice(),
                    delete_apply_type,
                    transaction,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(batch_operations)
    }
}
