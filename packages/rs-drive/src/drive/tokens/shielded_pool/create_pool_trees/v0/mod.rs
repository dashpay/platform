use crate::drive::shielded::paths::{
    token_shielded_pool_path_vec, SHIELDED_ANCHORS_BY_HEIGHT_KEY, SHIELDED_ANCHORS_IN_POOL_KEY,
    SHIELDED_NOTES_CHUNK_POWER, SHIELDED_NOTES_KEY, SHIELDED_NULLIFIERS_KEY,
    SHIELDED_TOTAL_BALANCE_KEY,
};
use crate::drive::tokens::paths::token_shielded_pools_root_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Version 0: the pool SumTree is inserted if it does not exist, and the five children are
    /// inserted only when the pool was created here (an existing pool already has them).
    ///
    /// The children mirror `Drive::insert_shielded_pool_structure` for the credit pool. They
    /// are batch operations, so the parent Merk shape is the batch's sorted-key shape rather
    /// than the credit pool's hand-ordered one; every node builds it the same way, which is all
    /// consensus needs.
    pub(super) fn create_token_shielded_pool_trees_operations_v0(
        &self,
        token_id: [u8; 32],
        allow_already_exists: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let pool_apply_type = match estimated_costs_only_with_layer_info {
            None => BatchInsertTreeApplyType::StatefulBatchInsertTree,
            Some(estimated_costs_only_with_layer_info) => {
                Self::add_estimation_costs_for_token_shielded_pool_operations(
                    token_id,
                    estimated_costs_only_with_layer_info,
                );
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: TreeType::BigSumTree,
                    tree_type: TreeType::SumTree,
                    flags_len: 0,
                }
            }
        };

        // The pool itself: [Tokens, 224, token_id], a SumTree so its balance item propagates
        // into the pools BigSumTree.
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKeyRef::<2>((token_shielded_pools_root_path(), token_id.as_slice())),
            TreeType::SumTree,
            None,
            pool_apply_type,
            transaction,
            &mut None,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted {
            if allow_already_exists {
                return Ok(batch_operations);
            }
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token shielded pool already exists".to_string(),
            )));
        }

        let pool_path = token_shielded_pool_path_vec(token_id);

        // Notes: the Orchard note commitment tree (Sinsemilla frontier + bulk-append items).
        batch_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            pool_path.clone(),
            vec![SHIELDED_NOTES_KEY],
            Element::empty_commitment_tree(SHIELDED_NOTES_CHUNK_POWER)?,
        ));

        // Nullifiers: spent-note set, provable count for the notes floor.
        batch_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            pool_path.clone(),
            vec![SHIELDED_NULLIFIERS_KEY],
            Element::empty_provable_count_tree(),
        ));

        // Anchors: anchor_bytes -> block_height_be.
        batch_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            pool_path.clone(),
            vec![SHIELDED_ANCHORS_IN_POOL_KEY],
            Element::empty_tree(),
        ));

        // Anchors by height: block_height_be -> anchor_bytes.
        batch_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            pool_path.clone(),
            vec![SHIELDED_ANCHORS_BY_HEIGHT_KEY],
            Element::empty_tree(),
        ));

        // Total balance: the amount of the token currently in the pool.
        batch_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            pool_path,
            vec![SHIELDED_TOTAL_BALANCE_KEY],
            Element::new_sum_item(0),
        ));

        Ok(batch_operations)
    }
}
