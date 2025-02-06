use crate::drive::tokens::paths::{token_perpetual_distributions_path_vec, TokenPerpetualDistributionMomentPaths, TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY, TOKEN_PERPETUAL_DISTRIBUTIONS_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_distribution_key::{
    TokenDistributionType, TokenDistributionKey,
};
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::{TokenPerpetualDistributionV0Accessors};
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::collections::HashMap;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::util::grove_operations::BatchDeleteApplyType::{StatefulBatchDelete, StatelessBatchDelete};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;

impl Drive {
    /// Version 0 of `mark_perpetual_release_as_distributed_v0`
    pub(super) fn mark_perpetual_release_as_distributed_operations_v0(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        previous_moment: RewardDistributionMoment,
        next_moment: RewardDistributionMoment,
        distribution_recipient: TokenDistributionRecipient,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,

        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations = vec![];
        if estimated_costs_only_with_layer_info.is_some() {
            // Drive::add_estimation_costs_for_perpetual_distribution(
            //     estimated_costs_only_with_layer_info,
            //     &platform_version.drive,
            // )?;
        }

        // Storage flags for cleanup logic
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

        let perpetual_distributions_path = token_perpetual_distributions_path_vec(token_id);

        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                perpetual_distributions_path,
                vec![TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY],
                Element::new_item(next_moment.to_be_bytes_vec()),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        // We will distribute for the first time on the next interval

        let distribution_path_for_previous_moment = previous_moment.distribution_path();

        let distribution_path_for_next_moment = next_moment.distribution_path();

        let distribution_key = TokenDistributionKey {
            token_id: token_id.into(),
            recipient: distribution_recipient,
            distribution_type: TokenDistributionType::Perpetual,
        };

        let serialized_distribution_key = distribution_key.serialize_consume_to_bytes()?;

        let remaining_reference = vec![vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY], token_id.to_vec()];

        let reference = ReferencePathType::UpstreamRootHeightReference(2, remaining_reference);

        let delete_apply_type = if estimated_costs_only_with_layer_info.is_some() {
            StatelessBatchDelete {
                in_tree_type: TreeType::NormalTree,
                estimated_key_size: DEFAULT_HASH_SIZE_U32,
                estimated_value_size: reference.serialized_size() as u32
                    + storage_flags.serialized_size(),
            }
        } else {
            // we know we are not deleting a subtree
            StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        };

        let new_element =
            Element::new_reference_with_flags(reference, storage_flags.to_some_element_flags());

        // We delete the old one
        self.batch_delete(
            distribution_path_for_previous_moment.as_slice().into(),
            &serialized_distribution_key,
            delete_apply_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        // Now we add the new one
        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                distribution_path_for_next_moment,
                serialized_distribution_key,
                new_element,
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
