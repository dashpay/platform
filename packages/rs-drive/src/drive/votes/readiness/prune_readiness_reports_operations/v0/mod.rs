use crate::drive::votes::paths::readiness_round_reports_tree_path;
use crate::drive::votes::readiness::estimation_costs::ESTIMATED_READINESS_REPORT_RECORD_SIZE;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn prune_readiness_reports_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hashes: &[[u8; 32]],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        } else {
            BatchDeleteApplyType::StatelessBatchDelete {
                in_tree_type: TreeType::CountTree,
                estimated_key_size: DEFAULT_HASH_SIZE_U32,
                estimated_value_size: ESTIMATED_READINESS_REPORT_RECORD_SIZE,
            }
        };
        let path = readiness_round_reports_tree_path(&contract_id, &round_id);
        for pro_tx_hash in pro_tx_hashes {
            self.batch_delete(
                (&path).into(),
                pro_tx_hash,
                apply_type,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        }
        Ok(drive_operations)
    }
}
