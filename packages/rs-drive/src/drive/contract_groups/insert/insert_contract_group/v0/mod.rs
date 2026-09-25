use crate::drive::contract_groups::paths::{
    contract_group_path, contract_groups_groups_path, CONTRACT_GROUP_CONTRACTS_KEY,
    CONTRACT_GROUP_DOCUMENT_TYPES_KEY, CONTRACT_GROUP_INFO_KEY, CONTRACT_GROUP_TOKENS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKey;
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::ContractGroupInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn insert_contract_group_v0(
        &self,
        contract_group_id: Identifier,
        info: &ContractGroupInfo,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.insert_contract_group_operations_v0(
            contract_group_id,
            info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    pub(super) fn insert_contract_group_operations_v0(
        &self,
        contract_group_id: Identifier,
        info: &ContractGroupInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_insert_contract_group(
                contract_group_id.to_buffer(),
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
                flags_len: 0,
            }
        };

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        // The group's own tree under [ContractGroups, Groups]. Registration is validated
        // against the state first, so an existing tree here means the state and the
        // validation disagree.
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKey((contract_groups_groups_path(), contract_group_id.to_vec())),
            TreeType::NormalTree,
            None,
            apply_type,
            transaction,
            &mut None,
            &mut batch_operations,
            &platform_version.drive,
        )?;
        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "contract group {} already exists",
                contract_group_id
            ))));
        }

        let group_path = contract_group_path(contract_group_id.as_slice());

        self.batch_insert(
            PathFixedSizeKeyRefElement((
                group_path,
                CONTRACT_GROUP_INFO_KEY,
                Element::new_item(info.serialize_to_bytes()?),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        for members_key in [
            CONTRACT_GROUP_CONTRACTS_KEY,
            CONTRACT_GROUP_DOCUMENT_TYPES_KEY,
            CONTRACT_GROUP_TOKENS_KEY,
        ] {
            self.batch_insert_empty_tree(
                group_path,
                DriveKeyInfo::KeyRef(members_key),
                None,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(batch_operations)
    }
}
