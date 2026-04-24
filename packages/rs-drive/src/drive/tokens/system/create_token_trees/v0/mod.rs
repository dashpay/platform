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

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::tokens::status::v0::TokenStatusV0Accessors;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_create_token_trees_and_initialize_supply_to_zero() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [51u8; 32];
        let contract_id = Identifier::from([52u8; 32]);

        drive
            .create_token_trees_v0(
                contract_id,
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        // Supply is initialized as SumItem(0)
        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch supply");
        assert_eq!(supply, Some(0));

        // Aggregated balances tree exists and sums to 0
        let balances = drive
            .fetch_token_total_aggregated_identity_balances(token_id, None, platform_version)
            .expect("expected to fetch balances");
        assert_eq!(balances, Some(0));
    }

    #[test]
    fn should_error_on_double_creation_without_allow_already_exists() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [53u8; 32];
        let contract_id = Identifier::from([54u8; 32]);

        drive
            .create_token_trees_v0(
                contract_id,
                0,
                token_id,
                false,
                false, // allow_already_exists
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("first creation should succeed");

        let result = drive.create_token_trees_v0(
            contract_id,
            0,
            token_id,
            false,
            false, // allow_already_exists
            &block_info,
            true,
            None,
            platform_version,
        );

        assert!(
            result.is_err(),
            "expected CorruptedDriveState on duplicate creation"
        );
    }

    #[test]
    fn should_succeed_on_double_creation_with_allow_already_exists() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [55u8; 32];
        let contract_id = Identifier::from([56u8; 32]);

        drive
            .create_token_trees_v0(
                contract_id,
                0,
                token_id,
                false,
                true, // allow_already_exists (idempotent)
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("first creation should succeed");

        // Seed some supply so we can confirm it is not reset to 0 by a second call
        drive
            .add_to_token_total_supply(
                token_id,
                999,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to seed supply");

        drive
            .create_token_trees_v0(
                contract_id,
                0,
                token_id,
                false,
                true, // allow_already_exists -> idempotent no-op
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("second idempotent creation should succeed");

        // The supply should remain what we had (not re-initialized to 0)
        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch supply");
        assert_eq!(supply, Some(999));
    }

    #[test]
    fn should_create_independent_trees_for_different_token_ids() {
        // Multi-token creation under different positions / ids coexists.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let contract_id = Identifier::from([200u8; 32]);

        for (i, tid) in [[201u8; 32], [202u8; 32], [203u8; 32]].iter().enumerate() {
            drive
                .create_token_trees_v0(
                    contract_id,
                    i as u16,
                    *tid,
                    false,
                    false,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to create token trees");
        }

        // Each token has independent supply counters initialized to 0
        for tid in [[201u8; 32], [202u8; 32], [203u8; 32]] {
            let supply = drive
                .fetch_token_total_supply(tid, None, platform_version)
                .expect("expected to fetch supply");
            assert_eq!(supply, Some(0));
        }

        // Mutating one token's supply must not affect the others
        drive
            .add_to_token_total_supply(
                [201u8; 32],
                500,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to seed supply for first token");

        assert_eq!(
            drive
                .fetch_token_total_supply([201u8; 32], None, platform_version)
                .unwrap(),
            Some(500)
        );
        assert_eq!(
            drive
                .fetch_token_total_supply([202u8; 32], None, platform_version)
                .unwrap(),
            Some(0)
        );
        assert_eq!(
            drive
                .fetch_token_total_supply([203u8; 32], None, platform_version)
                .unwrap(),
            Some(0)
        );
    }

    #[test]
    fn should_respect_start_as_paused_flag() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id_active = [60u8; 32];
        let token_id_paused = [61u8; 32];
        let contract_id = Identifier::from([62u8; 32]);

        drive
            .create_token_trees_v0(
                contract_id,
                0,
                token_id_active,
                false, // start_as_paused
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("active creation should succeed");

        drive
            .create_token_trees_v0(
                contract_id,
                1,
                token_id_paused,
                true, // start_as_paused
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("paused creation should succeed");

        // Both tokens should have supply initialized to 0
        for tid in [token_id_active, token_id_paused] {
            let supply = drive
                .fetch_token_total_supply(tid, None, platform_version)
                .expect("expected to fetch supply");
            assert_eq!(supply, Some(0));
        }

        // And critically: the paused flag must actually be persisted distinctly.
        let active_status = drive
            .fetch_token_status(token_id_active, None, platform_version)
            .expect("expected to fetch active token status")
            .expect("active token status must exist");
        assert!(
            !active_status.paused(),
            "token created with start_as_paused=false should not be paused"
        );

        let paused_status = drive
            .fetch_token_status(token_id_paused, None, platform_version)
            .expect("expected to fetch paused token status")
            .expect("paused token status must exist");
        assert!(
            paused_status.paused(),
            "token created with start_as_paused=true should be paused"
        );
    }
}
