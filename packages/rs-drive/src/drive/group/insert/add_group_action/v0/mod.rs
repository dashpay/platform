use crate::drive::group::{
    group_action_path, group_action_root_path, group_action_signers_path_vec, ACTION_INFO_KEY,
    ACTION_SIGNERS_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType, QueryTarget};
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use crate::util::object_size_info::{DriveKeyInfo, PathKeyElementInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::element::SumValue;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use grovedb_epoch_based_storage_flags::StorageFlags;
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_group_action_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
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

    /// Adds group creation operations to drive operations
    pub(super) fn add_group_action_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
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
    pub(super) fn add_group_action_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
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

        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        let group_action_root_path = group_action_root_path(
            contract_id.as_slice(),
            group_contract_position_bytes.as_slice(),
        );
        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
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
                false,
                None,
                apply_type,
                transaction,
                &mut None,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            if inserted_root_action {
                let group_action_path = group_action_path(
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

                let serialized = initialize_with_insert_action_info.serialize_consume_to_bytes()?;

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
                in_tree_using_sums: true,
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

        Ok(batch_operations)
    }
}
