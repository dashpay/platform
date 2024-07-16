use crate::drive::constants::{CONTESTED_DOCUMENT_REFERENCE_SIZE, STORAGE_FLAGS_SIZE};
use crate::drive::document::make_document_contested_reference;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DocumentInfo::{
    DocumentAndSerialization, DocumentEstimatedAverageSize, DocumentOwnedInfo,
    DocumentRefAndSerialization, DocumentRefInfo,
};
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::object_size_info::KeyElementInfo::{KeyElement, KeyUnknownElementSize};
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U8_SIZE_U32, U8_SIZE_U8};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, Mix};
use grovedb::EstimatedSumTrees::AllSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the terminal reference.
    #[inline(always)]
    pub(super) fn add_contested_reference_and_vote_subtree_to_document_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
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
        //              Sam's ID                Ivan's ID
        //             /    \                  /      \
        //         0 (ref)   1 (sum tree)    0 (ref)   1 (sum tree) <---- We now need to insert at this level
        //

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have all the identities
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(2),
                    estimated_layer_sizes: Mix {
                        // The votes don't have storage flags
                        subtrees_size: Some((
                            U8_SIZE_U8,
                            AllSumTrees, // There is 1 tree that is a sum tree, so all are sum trees
                            None,
                            1,
                        )),
                        items_size: None,
                        // The references do have storage flags
                        // We keep storage flags because when the item is moved after being won,
                        //  a transformation needs to take place on the storage flags to be able to
                        //  allow the refund of credits on delete later.
                        references_size: Some((
                            U8_SIZE_U8,
                            CONTESTED_DOCUMENT_REFERENCE_SIZE,
                            storage_flags.map(|s| s.serialized_size()),
                            1,
                        )),
                    },
                },
            );
        }

        // We create the reference

        // Here we are getting the document id and the reference
        let ref_key_element_info =
            match &document_and_contract_info.owned_document_info.document_info {
                DocumentRefAndSerialization((document, _, storage_flags))
                | DocumentRefInfo((document, storage_flags)) => {
                    let document_reference = make_document_contested_reference(
                        document,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                    );
                    KeyElement((&[0], document_reference))
                }
                DocumentOwnedInfo((document, storage_flags))
                | DocumentAndSerialization((document, _, storage_flags)) => {
                    let document_reference = make_document_contested_reference(
                        document,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                    );
                    KeyElement((&[0], document_reference))
                }
                DocumentEstimatedAverageSize(max_size) => {
                    let unique_id = document_and_contract_info
                        .document_type
                        .unique_id_for_storage()
                        .to_vec();
                    KeyUnknownElementSize((
                        KeyInfo::MaxKeySize {
                            unique_id,
                            max_size: DEFAULT_HASH_SIZE_U8,
                        },
                        Element::required_item_space(
                            *max_size,
                            STORAGE_FLAGS_SIZE,
                            &drive_version.grove_version,
                        )?,
                    ))
                }
            };

        // Now let's insert the reference, the reference is a key element that already has the 0

        let reference_path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
            index_path_info.clone(),
            ref_key_element_info,
        )?;

        // here we are inserting the ref
        self.batch_insert(
            reference_path_key_element_info,
            batch_operations,
            drive_version,
        )?;

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

        Ok(())
    }
}
