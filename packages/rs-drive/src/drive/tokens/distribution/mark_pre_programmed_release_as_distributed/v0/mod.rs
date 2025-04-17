use crate::drive::tokens::paths::{
    token_distributions_root_path_vec, token_ms_timed_at_time_distributions_path_vec,
    token_pre_programmed_distributions_identity_last_claimed_time_path_vec,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType::{
    StatefulBatchDelete, StatelessBatchDelete,
};
use crate::util::object_size_info::PathKeyElementInfo;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U32, U8_SIZE_U8};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_distribution_key::{
    TokenDistributionKey, TokenDistributionType,
};
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::prelude::TimestampMillis;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

/// Marks the pre-programmed release as distributed.
///
/// This function “consumes” the scheduled pre‑programmed release for the given token and
/// recipient (identity). In practice, it deletes the reference from the queue (i.e. the
/// millisecond‑timed distributions tree) that was previously inserted when scheduling the
/// pre‑programmed distribution.
///
/// # Parameters
/// - `token_id`: The 32‑byte token identifier.
/// - `owner_id`: The 32‑byte owner identifier (typically the caller).
/// - `identity_id`: The identity for which the pre‑programmed release was scheduled.
/// - `release_time`: The scheduled release time (as TimestampMillis, e.g. a 4‑byte value).
/// - `block_info`: Block info for the current state transition.
/// - `estimated_costs_only_with_layer_info`: Optional estimation info.
/// - `transaction`: The GroveDB transaction argument.
/// - `platform_version`: The current platform version.
///
/// # Returns
/// A vector of low‑level drive operations that, when applied, remove the pre‑programmed release
/// from the queue.
///
/// # Errors
/// Returns an error if serialization fails or if the underlying batch deletion fails.
impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn mark_pre_programmed_release_as_distributed_operations_v0(
        &self,
        token_id: [u8; 32],
        recipient_id: [u8; 32],
        release_time: TimestampMillis, // TimestampMillis represented as a 32-bit unsigned integer
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let pre_programmed_distributions_path =
            token_pre_programmed_distributions_identity_last_claimed_time_path_vec(token_id);

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(token_distributions_root_path_vec()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(1, false), // We should be on the first level
                    estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
                },
            );

            Drive::add_estimation_costs_for_token_pre_programmed_distribution::<
                std::iter::Empty<&TimestampMillis>,
            >(
                token_id,
                None,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;

            Drive::add_estimation_costs_for_root_token_ms_interval_distribution(
                [&release_time],
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(pre_programmed_distributions_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(5, false),
                    estimated_layer_sizes: AllItems(1, 8, None),
                },
            );
        }
        // Initialize an empty batch of operations
        let mut batch_operations = vec![];

        // Create storage flags for cleanup logic; these flags are attached to inserted elements.
        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(recipient_id));

        // The pre-programmed distribution was scheduled by inserting a reference in the
        // millisecond-timed distributions tree at a key corresponding to the release time.
        let ms_time_at_time_distribution_path =
            token_ms_timed_at_time_distributions_path_vec(release_time);

        // Build the distribution key used when the pre-programmed release was scheduled.
        let distribution_key = TokenDistributionKey {
            token_id: token_id.into(),
            recipient: TokenDistributionRecipient::Identity(recipient_id.into()),
            distribution_type: TokenDistributionType::PreProgrammed,
        };

        // Serialize the distribution key to obtain the key used in the reference tree.
        let serialized_distribution_key = distribution_key.serialize_consume_to_bytes()?;

        // When scheduling, the reference was created using a “remaining reference” vector:
        let remaining_reference = vec![
            vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
            token_id.to_vec(),
            release_time.to_be_bytes().to_vec(),
            recipient_id.to_vec(),
        ];

        let reference = ReferencePathType::UpstreamRootHeightReference(2, remaining_reference);

        // Choose a delete apply type. If we are only estimating costs, use a stateless delete;
        // otherwise, a stateful delete.
        let delete_apply_type = if estimated_costs_only_with_layer_info.is_some() {
            StatelessBatchDelete {
                in_tree_type: TreeType::NormalTree,
                estimated_key_size: DEFAULT_HASH_SIZE_U32,
                estimated_value_size: reference.serialized_size() as u32
                    + storage_flags.serialized_size(),
            }
        } else {
            StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(grovedb::MaybeTree::NotTree),
            }
        };

        // Delete the reference from the millisecond-timed distributions tree.
        self.batch_delete(
            ms_time_at_time_distribution_path.as_slice().into(),
            &serialized_distribution_key,
            delete_apply_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        self.batch_insert(
            PathKeyElementInfo::<0>::PathKeyElement((
                pre_programmed_distributions_path,
                recipient_id.to_vec(),
                Element::new_item(release_time.to_be_bytes().to_vec()),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
