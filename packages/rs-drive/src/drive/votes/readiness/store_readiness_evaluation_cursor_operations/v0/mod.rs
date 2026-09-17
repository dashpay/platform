use crate::drive::votes::paths::{
    readiness_tree_path, readiness_tree_path_vec, READINESS_EVALUATION_CURSOR_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn store_readiness_evaluation_cursor_operations_v0(
        &self,
        last_contract_id: Option<[u8; 32]>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                [0u8; 32],
                [0u8; 32],
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        match last_contract_id {
            Some(contract_id) => {
                drive_operations.push(GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
                    readiness_tree_path_vec(),
                    vec![READINESS_EVALUATION_CURSOR_KEY],
                    Element::new_item(contract_id.to_vec()),
                )));
            }
            None => {
                let path = readiness_tree_path();
                let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    let present = self.grove_has_raw(
                        (&path).into(),
                        &[READINESS_EVALUATION_CURSOR_KEY],
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
                } else {
                    BatchDeleteApplyType::StatelessBatchDelete {
                        in_tree_type: TreeType::NormalTree,
                        estimated_key_size: 1,
                        estimated_value_size: DEFAULT_HASH_SIZE_U32,
                    }
                };
                self.batch_delete(
                    (&path).into(),
                    &[READINESS_EVALUATION_CURSOR_KEY],
                    apply_type,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
            }
        }
        Ok(drive_operations)
    }
}
