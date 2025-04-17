use crate::drive::tokens::paths::token_perpetual_distributions_identity_last_claimed_time_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{PathKeyElementInfo};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TreeType};
use std::collections::HashMap;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::AllItems;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;

impl Drive {
    /// Version 0 of `mark_perpetual_release_as_distributed_v0`
    pub(super) fn mark_perpetual_release_as_distributed_operations_v0(
        &self,
        token_id: [u8; 32],
        recipient_id: [u8; 32],
        current_moment: RewardDistributionMoment,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations = vec![];

        let perpetual_distributions_path =
            token_perpetual_distributions_identity_last_claimed_time_path_vec(token_id);

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_token_perpetual_distribution(
                Some(token_id),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;

            let estimated_layer_count = match current_moment {
                RewardDistributionMoment::BlockBasedMoment(_)
                | RewardDistributionMoment::TimeBasedMoment(_) => EstimatedLevel(0, false),
                RewardDistributionMoment::EpochBasedMoment(_) => EstimatedLevel(10, false),
            };

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(perpetual_distributions_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count,
                    estimated_layer_sizes: AllItems(1, 8, None),
                },
            );
        }

        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                perpetual_distributions_path,
                recipient_id.to_vec(),
                Element::new_item(current_moment.to_be_bytes_vec()),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
