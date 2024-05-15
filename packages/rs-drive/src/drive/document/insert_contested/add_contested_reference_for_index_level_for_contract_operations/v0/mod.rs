use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, STORAGE_FLAGS_SIZE};
use crate::drive::document::{
    document_reference_size, make_document_contested_reference, make_document_reference,
};
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::object_size_info::DocumentInfo::{
    DocumentAndSerialization, DocumentEstimatedAverageSize, DocumentOwnedInfo,
    DocumentRefAndSerialization, DocumentRefInfo,
};
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::drive::object_size_info::KeyElementInfo::{KeyElement, KeyUnknownElementSize};
use crate::drive::object_size_info::{DocumentAndContractInfo, PathInfo, PathKeyElementInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0Getters;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::{AllReference, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the terminal reference.
    #[inline(always)]
    pub(super) fn add_contested_reference_to_document_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        mut index_path_info: PathInfo<0>,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // Contested Resource Index
        // Under each tree we have all identifiers of identities that want the contested resource
        // Contrary to normal secondary indexes there are no property names and there is no termination key "0"
        // We get something like
        //                Inter-wizard championship (event type)
        //                             |
        //                       Goblet of Fire (event name)
        //                  /                    \
        //       Sam's Document ID         Ivan's Document ID
        //             /    \                  /      \
        //         0 (ref)   1 (sum tree)    0 (ref)   1 (sum tree)
        //

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

        // Here we are getting the document id and the reference
        let (document_id, ref_key_element_info) =
            match &document_and_contract_info.owned_document_info.document_info {
                DocumentRefAndSerialization((document, _, storage_flags))
                | DocumentRefInfo((document, storage_flags)) => {
                    let document_reference = make_document_contested_reference(
                        document,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                    );
                    (document.id(), KeyElement((&[0], document_reference)))
                }
                DocumentOwnedInfo((document, storage_flags))
                | DocumentAndSerialization((document, _, storage_flags)) => {
                    let document_reference = make_document_contested_reference(
                        document,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                    );
                    (document.id(), KeyElement((&[0], document_reference)))
                }
                DocumentEstimatedAverageSize(max_size) => {
                    let unique_id = document_and_contract_info
                        .document_type
                        .unique_id_for_storage();
                    let unique_id_vec = unique_id.to_vec();
                    (
                        unique_id.into(),
                        KeyUnknownElementSize((
                            KeyInfo::MaxKeySize {
                                unique_id: unique_id_vec,
                                max_size: DEFAULT_HASH_SIZE_U8,
                            },
                            Element::required_item_space(*max_size, STORAGE_FLAGS_SIZE),
                        )),
                    )
                }
            };

        // Let's start by inserting the document id tree

        // here we are the tree that will contain the ref
        // We are inserting this at item name contested / Goblet of Fire / 0 with the key of
        //    document_key_path_info

        let document_id_key_path_info = KeyRef(document_id.as_slice());

        let path_key_info = document_id_key_path_info.add_path_info(index_path_info.clone());

        index_path_info.push(Key(document_id.to_vec()))?;

        // We check to make sure we are not overriding the tree
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            path_key_info,
            false,
            storage_flags,
            apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;

        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                "contested votes sub tree document already exists",
            )));
        }

        let mut document_path_info = index_path_info.clone();

        document_path_info.push(KeyRef(document_id.as_slice()))?;

        let votes_key_path_info = KeyRef(&[1]);

        let votes_path_key_info = votes_key_path_info.add_path_info(document_path_info.clone());

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );

            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                votes_path_key_info.clone().convert_to_key_info_path()?,
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        let reference_path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
            document_path_info.clone(),
            ref_key_element_info,
        )?;

        // here we are inserting the ref
        self.batch_insert(
            reference_path_key_element_info,
            batch_operations,
            drive_version,
        )?;

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
            votes_path_key_info,
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
                "contested votes tree already exists",
            )));
        }

        // Now we need to add a reference to this votes, so we can keep track of it more easily

        // self.add_new_masternode_vote_type()

        Ok(())
    }
}
