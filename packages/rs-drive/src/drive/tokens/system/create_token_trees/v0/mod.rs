use crate::drive::balances::total_tokens_root_supply_path;
use crate::drive::tokens::paths::{
    token_balances_root_path, token_contract_infos_root_path, token_identity_infos_root_path,
    token_statuses_root_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType, QueryTarget};
use crate::util::object_size_info::PathKeyElementInfo;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::TokenContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::status::TokenStatus;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Creates a new token root subtree at `TokenBalances` keyed by `token_id`.
    /// This function applies the operations directly, calculates fees, and returns the fee result.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_token_trees_v0(
        &self,
        contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // Add operations to create the token root tree
        self.create_token_trees_add_to_operations_v0(
            contract_id,
            token_contract_position,
            token_id,
            start_as_paused,
            allow_already_exists,
            apply,
            &mut None,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        // If applying, calculate fees
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

    /// Adds the token root creation operations to the provided `drive_operations` vector without
    #[allow(clippy::too_many_arguments)]
    /// calculating or returning fees. If `apply` is false, it will only estimate costs.
    pub(super) fn create_token_trees_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        // Get the operations required to create the token tree
        let batch_operations = self.create_token_trees_operations_v0(
            contract_id,
            token_contract_position,
            token_id,
            start_as_paused,
            allow_already_exists,
            previous_batch_operations,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        // Apply or estimate the operations
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Gathers the operations needed to create the token root subtree. If `apply` is false, it
    /// populates `estimated_costs_only_with_layer_info` instead of applying.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_token_trees_operations_v0(
        &self,
        contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let non_sum_tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        let item_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: TreeType::NormalTree,
                target: QueryTarget::QueryTargetValue(8),
            }
        };

        let token_balance_tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::BigSumTree,
                tree_type: TreeType::SumTree,
                flags_len: 0,
            }
        };

        // Insert an empty tree for this token if it doesn't exist
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKeyRef::<2>((token_balances_root_path(), token_id.as_slice())),
            TreeType::SumTree,
            None,
            token_balance_tree_apply_type,
            transaction,
            previous_batch_operations,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted && !allow_already_exists {
            // The token root already exists. Depending on your logic, this might be allowed or should be treated as an error.
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token balance root tree already exists".to_string(),
            )));
        }

        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKeyRef::<2>((token_identity_infos_root_path(), token_id.as_slice())),
            TreeType::NormalTree,
            None,
            non_sum_tree_apply_type,
            transaction,
            &mut None,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted && !allow_already_exists {
            // The token root already exists. Depending on your logic, this might be allowed or should be treated as an error.
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token balance tree already exists".to_string(),
            )));
        }

        let starting_status = TokenStatus::new(start_as_paused, platform_version)?;
        let token_status_bytes = starting_status.serialize_consume_to_bytes()?;

        let inserted = self.batch_insert_if_not_exists(
            PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                token_statuses_root_path(),
                token_id.as_slice(),
                Element::Item(token_status_bytes, None),
            )),
            item_apply_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted && !allow_already_exists {
            // The token root already exists. Depending on your logic, this might be allowed or should be treated as an error.
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token info tree already exists".to_string(),
            )));
        }

        let token_contract_info =
            TokenContractInfo::new(contract_id, token_contract_position, platform_version)?;
        let token_contract_info_bytes = token_contract_info.serialize_consume_to_bytes()?;

        self.batch_insert(
            PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                token_contract_infos_root_path(),
                token_id.as_slice(),
                Element::Item(token_contract_info_bytes, None),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        self.batch_insert_sum_item_if_not_exists(
            PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                total_tokens_root_supply_path(),
                token_id.as_slice(),
                Element::SumItem(0, None),
            )),
            !allow_already_exists,
            item_apply_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
