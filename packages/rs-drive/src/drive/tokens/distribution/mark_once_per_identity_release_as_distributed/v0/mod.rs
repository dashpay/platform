use crate::drive::tokens::paths::token_once_per_identity_distributions_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation};
use std::collections::HashMap;

impl Drive {
    /// Inserts the claim item: key is the claimant's identity id, value is the claim's block
    /// time as 8 big-endian bytes, storage flags owned by the claimant.
    pub(super) fn mark_once_per_identity_release_as_distributed_operations_v0(
        &self,
        token_id: [u8; 32],
        recipient_id: [u8; 32],
        claimed_at_ms: TimestampMillis,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_token_once_per_identity_distribution(
                Some(token_id),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(recipient_id));

        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                token_once_per_identity_distributions_path_vec(token_id),
                recipient_id.to_vec(),
                Element::new_item_with_flags(
                    claimed_at_ms.to_be_bytes().to_vec(),
                    storage_flags.to_some_element_flags(),
                ),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
