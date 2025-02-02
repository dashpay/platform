use crate::drive::tokens::paths::{
    token_ms_timed_at_time_distributions_path_vec, token_ms_timed_distributions_path_vec,
    token_pre_programmed_at_time_distribution_path_vec, token_pre_programmed_distributions_path,
    token_root_pre_programmed_distributions_path, TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{DriveKeyInfo, PathInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_distribution_key::{
    DistributionType, TokenDistributionKey,
};
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::methods::v0::TokenPreProgrammedDistributionV0Methods;

impl Drive {
    /// Version 0 of `add_perpetual_distribution`
    pub(super) fn add_pre_programmed_distributions_v0(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        distribution: TokenPreProgrammedDistribution,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

        let pre_programmed_distributions_path = token_root_pre_programmed_distributions_path();

        // Insert the tree for this token's perpetual distribution
        let apply_tree_type_no_storage_flags = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        let apply_tree_type_with_storage_flags = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: storage_flags.serialized_size(),
            }
        };

        let token_tree_key_info = DriveKeyInfo::Key(token_id.to_vec());
        let pre_programmed_distributions_path_key_info = token_tree_key_info.add_path_info::<3>(
            PathInfo::PathFixedSizeArray(pre_programmed_distributions_path),
        );

        let inserted = self.batch_insert_empty_tree_if_not_exists(
            pre_programmed_distributions_path_key_info,
            TreeType::NormalTree,
            None, // we will never clean this part up
            apply_tree_type_no_storage_flags,
            transaction,
            &mut None,
            batch_operations,
            &platform_version.drive,
        )?;

        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution("we can not insert the pre programmed distribution as it already existed, this should have been validated before insertion")));
        }
        let pre_programmed_distributions_path = token_pre_programmed_distributions_path(&token_id);

        for (time, distribution) in distribution.distributions() {
            self.batch_insert_empty_sum_tree(
                pre_programmed_distributions_path,
                DriveKeyInfo::Key(time.to_be_bytes().to_vec()),
                None, // we will never clean this part up
                batch_operations,
                &platform_version.drive,
            )?;

            let ms_time_distribution_path = token_ms_timed_distributions_path_vec();

            let time_tree_key_info = DriveKeyInfo::Key(time.to_be_bytes().to_vec());
            let time_tree_reference_path_key_info = time_tree_key_info
                .add_path_info::<0>(PathInfo::PathAsVec(ms_time_distribution_path));

            self.batch_insert_empty_tree_if_not_exists(
                time_tree_reference_path_key_info,
                TreeType::NormalTree,
                Some(&storage_flags),
                apply_tree_type_with_storage_flags,
                transaction,
                &mut None,
                batch_operations,
                &platform_version.drive,
            )?;

            let pre_programmed_at_time_distribution_path =
                token_pre_programmed_at_time_distribution_path_vec(token_id, *time);
            let ms_time_at_time_distribution_path =
                token_ms_timed_at_time_distributions_path_vec(*time);

            for (recipient, amount) in distribution {
                if *amount > i64::MAX as u64 {
                    return Err(Error::Protocol(ProtocolError::Overflow(
                        "distribution amount over i64::Max",
                    )));
                }
                // We use a sum tree to be able to ask "at this time how much was distributed"
                self.batch_insert(
                    PathKeyElementInfo::<0>::PathKeyElement((
                        pre_programmed_at_time_distribution_path.clone(),
                        recipient.to_vec(),
                        Element::new_sum_item(*amount as i64),
                    )),
                    batch_operations,
                    &platform_version.drive,
                )?;

                let distribution_key = TokenDistributionKey {
                    token_id: token_id.into(),
                    recipient: TokenDistributionRecipient::Identity(*recipient),
                    distribution_type: DistributionType::PreProgrammed,
                };

                let serialized_key = distribution_key.serialize_consume_to_bytes()?;

                let remaining_reference = vec![
                    vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
                    token_id.to_vec(),
                    time.to_be_bytes().to_vec(),
                    recipient.to_vec(),
                ];

                let reference =
                    ReferencePathType::UpstreamRootHeightReference(2, remaining_reference);

                // Now we create the reference
                self.batch_insert(
                    PathKeyElementInfo::<0>::PathKeyElement((
                        ms_time_at_time_distribution_path.clone(),
                        serialized_key,
                        Element::new_reference_with_flags(
                            reference,
                            storage_flags.to_some_element_flags(),
                        ),
                    )),
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(())
    }
}
