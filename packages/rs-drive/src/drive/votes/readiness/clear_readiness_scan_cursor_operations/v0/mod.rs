use crate::drive::votes::paths::{readiness_round_tree_path, READINESS_ROUND_SCAN_CURSOR_KEY};
use crate::drive::votes::readiness::estimation_costs::ESTIMATED_READINESS_SCAN_CURSOR_SIZE;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn clear_readiness_scan_cursor_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        let path = readiness_round_tree_path(&contract_id, &round_id);
        let apply_type = match estimated_costs_only_with_layer_info {
            None => {
                let present = self.grove_has_raw(
                    (&path).into(),
                    &[READINESS_ROUND_SCAN_CURSOR_KEY],
                    DirectQueryType::StatefulDirectQuery,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
                if !present {
                    return Ok(drive_operations);
                }
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                }
            }
            Some(estimated_costs_only_with_layer_info) => {
                Self::add_estimation_costs_for_readiness(
                    contract_id,
                    round_id,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
                BatchDeleteApplyType::StatelessBatchDelete {
                    in_tree_type: TreeType::NormalTree,
                    estimated_key_size: 1,
                    estimated_value_size: ESTIMATED_READINESS_SCAN_CURSOR_SIZE,
                }
            }
        };
        self.batch_delete(
            (&path).into(),
            &[READINESS_ROUND_SCAN_CURSOR_KEY],
            apply_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(drive_operations)
    }
}
