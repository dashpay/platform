use crate::drive::tokens::paths::{
    token_perpetual_distributions_path_vec, token_root_perpetual_distributions_path_vec,
    TokenPerpetualDistributionPaths, TOKEN_PERPETUAL_DISTRIBUTIONS_FIRST_EVENT_KEY,
    TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY, TOKEN_PERPETUAL_DISTRIBUTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{PathKeyElementInfo, PathKeyInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_distribution_key::{
    TokenDistributionKey, TokenDistributionType,
};
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::{
    TokenPerpetualDistributionV0Accessors, TokenPerpetualDistributionV0Methods,
};
use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Version 0 of `add_perpetual_distribution`
    pub(super) fn add_perpetual_distribution_v0(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        distribution: &TokenPerpetualDistribution,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_token_perpetual_distribution(
                Some(token_id),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let serialized_distribution = distribution.serialize_to_bytes()?;

        // Storage flags for cleanup logic
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

        let root_perpetual_distributions_path = token_root_perpetual_distributions_path_vec();

        let perpetual_distributions_path = token_perpetual_distributions_path_vec(token_id);

        let tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::<0>::PathKey((root_perpetual_distributions_path, token_id.to_vec())),
            TreeType::NormalTree,
            None,
            tree_apply_type,
            transaction,
            &mut None,
            batch_operations,
            &platform_version.drive,
        )?;

        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution("we can not insert the perpetual distribution as it already existed, this should have been validated before insertion")));
        }

        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                perpetual_distributions_path.clone(),
                vec![TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY],
                Element::new_item(serialized_distribution),
            )),
            batch_operations,
            &platform_version.drive,
        )?;

        let next_interval = distribution.next_interval(block_info);

        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                perpetual_distributions_path,
                vec![TOKEN_PERPETUAL_DISTRIBUTIONS_FIRST_EVENT_KEY],
                Element::new_item(next_interval.to_be_bytes().to_vec()),
            )),
            batch_operations,
            &platform_version.drive,
        )?;

        // We will distribute for the first time on the next interval
        let distribution_path_for_next_interval =
            distribution.distribution_path_for_next_interval_from_block_info(block_info);

        let distribution_key = TokenDistributionKey {
            token_id: token_id.into(),
            recipient: distribution.distribution_recipient(),
            distribution_type: TokenDistributionType::Perpetual,
        };

        let serialized_key = distribution_key.serialize_consume_to_bytes()?;

        let remaining_reference = vec![vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY], token_id.to_vec()];

        let reference = ReferencePathType::UpstreamRootHeightReference(2, remaining_reference);

        // Now we create the reference
        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                distribution_path_for_next_interval,
                serialized_key,
                Element::new_reference_with_flags(reference, storage_flags.to_some_element_flags()),
            )),
            batch_operations,
            &platform_version.drive,
        )?;

        Ok(())
    }
}
