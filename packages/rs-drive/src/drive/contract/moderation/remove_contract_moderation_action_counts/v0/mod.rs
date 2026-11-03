use crate::drive::contract::moderation::types::CONTRACT_MODERATION_ACTION_COUNT_SIZE;
use crate::drive::contract::paths::contract_moderation_action_counts_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use grovedb::{MaybeTree, TreeType};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn remove_contract_moderation_action_counts_operations_v0(
        &self,
        contract_id: Identifier,
        identity_ids: &[Identifier],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        if identity_ids.is_empty() {
            return Ok(batch_operations);
        }
        let apply_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Drive::add_estimation_costs_for_contract_moderation_action_counts(
                contract_id.to_buffer(),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            BatchDeleteApplyType::StatelessBatchDelete {
                in_tree_type: TreeType::NormalTree,
                estimated_key_size: DEFAULT_HASH_SIZE_U32,
                estimated_value_size: CONTRACT_MODERATION_ACTION_COUNT_SIZE as u32,
            }
        } else {
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        };
        let path = contract_moderation_action_counts_path(contract_id.as_slice());
        for identity_id in identity_ids {
            // A count carries no storage flags, so its removal refunds nobody.
            self.batch_delete(
                (&path).into(),
                identity_id.as_slice(),
                apply_type,
                transaction,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }
        Ok(batch_operations)
    }
}
