use crate::drive::contract::paths::{contract_moderation_list_key, contract_root_path};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::config::moderation::ContractModerationConfig;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_moderation_trees_operations_v0(
        &self,
        contract_id: [u8; 32],
        moderation: &ContractModerationConfig,
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_moderation_trees(
                contract_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: storage_flags
                    .map(|flags| flags.serialized_size())
                    .unwrap_or_default(),
            }
        };

        let contract_root_path = contract_root_path(&contract_id);

        for list in moderation.lists() {
            // A contract update that turns a list on adds the tree; a list that is already on
            // has one. Both go through the same `if not exists` insert, and the check is
            // what makes the update path safe to call on an insert path's fresh contract too.
            self.batch_insert_empty_tree_if_not_exists(
                PathFixedSizeKeyRef((contract_root_path, contract_moderation_list_key(list))),
                TreeType::NormalTree,
                storage_flags,
                apply_type,
                transaction,
                &mut None,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
