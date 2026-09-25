use crate::drive::document::expiration::paths::documents_expirations_at_time_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_document_expiration_operations_v0(
        &self,
        document_id: [u8; 32],
        expires_at_ms: TimestampMillis,
        entry_value_size: u32,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let apply_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Self::add_estimation_costs_for_document_expiration(
                expires_at_ms,
                entry_value_size,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            BatchDeleteApplyType::StatelessBatchDelete {
                in_tree_type: TreeType::NormalTree,
                estimated_key_size: DEFAULT_HASH_SIZE_U32,
                estimated_value_size: entry_value_size,
            }
        } else {
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        };
        let time_path = documents_expirations_at_time_path_vec(expires_at_ms);
        self.batch_delete(
            time_path.as_slice().into(),
            document_id.as_slice(),
            apply_type,
            transaction,
            batch_operations,
            &platform_version.drive,
        )
    }
}
