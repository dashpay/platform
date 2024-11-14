use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::object_size_info::PathInfo;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U8_SIZE_U32, U8_SIZE_U8};
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, Mix};
use grovedb::EstimatedSumTrees::AllSumTrees;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the contested vote subtree
    #[inline(always)]
    pub(super) fn add_contested_vote_subtree_for_non_identities_operations_v0(
        &self,
        index_path_info: PathInfo<0>,
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        //                Inter-wizard championship (event type)
        //                             |
        //                       Goblet of Fire (event name)
        //                  /                    \
        //              Lock                Abstain
        //              \                      \
        //          1 (sum tree)           1 (sum tree) <---- We now need to insert at this level
        //

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have all the identities
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(1),
                    estimated_layer_sizes: Mix {
                        // The votes don't have storage flags
                        subtrees_size: Some((
                            U8_SIZE_U8,
                            AllSumTrees, // There is 1 tree that is a sum tree, so all are sum trees
                            None,
                            1,
                        )),
                        items_size: None,
                        references_size: None,
                    },
                },
            );
        }

        // Let's insert the voting tree

        let votes_key_path_info = KeyRef(&[1]);

        let votes_path_key_info = votes_key_path_info.add_path_info(index_path_info.clone());

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                votes_path_key_info.clone().convert_to_key_info_path()?,
                EstimatedLayerInformation {
                    is_sum_tree: true,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, U8_SIZE_U32, None),
                },
            );
        }

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: true,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

        // here we are the tree that will contain the voting tree
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            votes_path_key_info.clone(),
            true,
            storage_flags,
            apply_type,
            transaction,
            &mut None,
            batch_operations,
            drive_version,
        )?;

        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                format!("contested votes tree already exists for a non identity (abstain or lock), trying to insert empty tree at path {}", votes_path_key_info),
            )));
        }

        Ok(())
    }
}
