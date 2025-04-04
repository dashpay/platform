use crate::drive::tokens::paths::{
    token_perpetual_distributions_path_vec, token_root_perpetual_distributions_path_vec,
    TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
    TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{PathKeyElementInfo, PathKeyInfo};
use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Version 0 of `add_perpetual_distribution`
    pub(super) fn add_perpetual_distribution_v0(
        &self,
        token_id: [u8; 32],
        distribution: &TokenPerpetualDistribution,
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

        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                perpetual_distributions_path.clone(),
                vec![TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY],
                Element::empty_tree(),
            )),
            batch_operations,
            &platform_version.drive,
        )?;

        Ok(())
    }
}
