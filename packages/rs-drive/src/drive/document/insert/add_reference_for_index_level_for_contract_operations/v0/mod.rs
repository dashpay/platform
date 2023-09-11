use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, STORAGE_FLAGS_SIZE};
use crate::drive::document::{document_reference_size, make_document_reference};
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::object_size_info::DocumentInfo::{
    DocumentAndSerialization, DocumentEstimatedAverageSize, DocumentOwnedInfo,
    DocumentRefAndSerialization, DocumentRefInfo,
};
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::drive::object_size_info::KeyElementInfo::{KeyElement, KeyUnknownElementSize};
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::drive::object_size_info::{DocumentAndContractInfo, PathInfo, PathKeyElementInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::IndexType;
use dpp::data_contract::document_type::IndexType::{ContestedResourceIndex, NonUniqueIndex};
use dpp::document::DocumentV0Getters;
use dpp::platform_value::Value::Identifier;
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
    pub(super) fn add_reference_for_index_level_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        mut index_path_info: PathInfo<0>,
        index_type: IndexType,
        any_fields_null: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // unique indexes will be stored under key "0"
        // non unique indices should have a tree at key "0" that has all elements based off of primary key
        if index_type == NonUniqueIndex || index_type == ContestedResourceIndex || any_fields_null {
            // Tree generation, this happens for both non unique indexes, unique indexes with a null inside
            // a member of the path and for contested resource indexes
            let key_path_info = KeyRef(&[0]);

            let path_key_info = key_path_info.add_path_info(index_path_info.clone());

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

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info,
                *storage_flags,
                apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;

            index_path_info.push(Key(vec![0]))?;

            if index_type != ContestedResourceIndex {
                // This is the simpler situation
                // Under each tree we have all the references

                if let Some(estimated_costs_only_with_layer_info) =
                    estimated_costs_only_with_layer_info
                {
                    // On this level we will have a 0 and all the top index paths
                    estimated_costs_only_with_layer_info.insert(
                        index_path_info.clone().convert_to_key_info_path(),
                        EstimatedLayerInformation {
                            is_sum_tree: false,
                            estimated_layer_count: PotentiallyAtMaxElements,
                            estimated_layer_sizes: AllReference(
                                DEFAULT_HASH_SIZE_U8,
                                document_reference_size(document_and_contract_info.document_type),
                                storage_flags.map(|s| s.serialized_size()),
                            ),
                        },
                    );
                }

                let key_element_info =
                    match &document_and_contract_info.owned_document_info.document_info {
                        DocumentRefAndSerialization((document, _, storage_flags))
                        | DocumentRefInfo((document, storage_flags)) => {
                            let document_reference = make_document_reference(
                                document,
                                document_and_contract_info.document_type,
                                storage_flags.as_ref().map(|flags| flags.as_ref()),
                            );
                            KeyElement((document.id_ref().as_slice(), document_reference))
                        }
                        DocumentOwnedInfo((document, storage_flags))
                        | DocumentAndSerialization((document, _, storage_flags)) => {
                            let document_reference = make_document_reference(
                                document,
                                document_and_contract_info.document_type,
                                storage_flags.as_ref().map(|flags| flags.as_ref()),
                            );
                            KeyElement((document.id_ref().as_slice(), document_reference))
                        }
                        DocumentEstimatedAverageSize(max_size) => KeyUnknownElementSize((
                            KeyInfo::MaxKeySize {
                                unique_id: document_and_contract_info
                                    .document_type
                                    .unique_id_for_storage()
                                    .to_vec(),
                                max_size: DEFAULT_HASH_SIZE_U8,
                            },
                            Element::required_item_space(*max_size, STORAGE_FLAGS_SIZE),
                        )),
                    };

                let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                    index_path_info,
                    key_element_info,
                )?;

                // here we should return an error if the element already exists
                self.batch_insert(path_key_element_info, batch_operations, drive_version)?;
            } else {
                // Contested Resource index
                // Under each tree we have all identifiers of identities that want the contested resource
                // We get something like
                //                      item name contested (there will be another path with item_name)
                //                             |
                //                       Goblet of Fire
                //                             |
                //              0 (for the termination of the index)
                //                  /                    \
                //            Sam's Id                Ivan's Id
                //             /    \                  /      \
                //         0 (ref)   1 (sum tree)    0 (ref)   1 (sum tree)
                //

                let (document_id, ref_key_element_info) =
                    match &document_and_contract_info.owned_document_info.document_info {
                        DocumentRefAndSerialization((document, _, storage_flags))
                        | DocumentRefInfo((document, storage_flags)) => {
                            let document_reference = make_document_reference(
                                document,
                                document_and_contract_info.document_type,
                                storage_flags.as_ref().map(|flags| flags.as_ref()),
                            );
                            (document.id(), KeyElement((&[0], document_reference)))
                        }
                        DocumentOwnedInfo((document, storage_flags))
                        | DocumentAndSerialization((document, _, storage_flags)) => {
                            let document_reference = make_document_reference(
                                document,
                                document_and_contract_info.document_type,
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

                let document_key_path_info = document.id_ref().as_slice();

                let ref_key_path_info = KeyRef(&[0]);

                let votes_key_path_info = KeyRef(&[1]);

                let ref_path_key_info = ref_key_path_info.add_path_info(index_path_info.clone());

                let votes_path_key_info =
                    votes_key_path_info.add_path_info(index_path_info.clone());

                if let Some(estimated_costs_only_with_layer_info) =
                    estimated_costs_only_with_layer_info
                {
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

                let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                    index_path_info,
                    key_element_info,
                )?;

                // here we should return an error if the element already exists
                self.batch_insert_empty_tree_if_not_exists(
                    path_key_element_info,
                    batch_operations,
                    drive_version,
                )?;
            }
        } else {
            let key_element_info =
                match &document_and_contract_info.owned_document_info.document_info {
                    DocumentRefAndSerialization((document, _, storage_flags))
                    | DocumentRefInfo((document, storage_flags)) => {
                        let document_reference = make_document_reference(
                            document,
                            document_and_contract_info.document_type,
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                        );
                        KeyElement((&[0], document_reference))
                    }
                    DocumentOwnedInfo((document, storage_flags))
                    | DocumentAndSerialization((document, _, storage_flags)) => {
                        let document_reference = make_document_reference(
                            document,
                            document_and_contract_info.document_type,
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                        );
                        KeyElement((&[0], document_reference))
                    }
                    DocumentEstimatedAverageSize(estimated_size) => KeyUnknownElementSize((
                        KeyInfo::MaxKeySize {
                            unique_id: document_and_contract_info
                                .document_type
                                .unique_id_for_storage()
                                .to_vec(),
                            max_size: 1,
                        },
                        Element::required_item_space(*estimated_size, STORAGE_FLAGS_SIZE),
                    )),
                };

            let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                index_path_info,
                key_element_info,
            )?;

            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertApplyType::StatefulBatchInsert
            } else {
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_using_sums: false,
                    target: QueryTargetValue(
                        document_reference_size(document_and_contract_info.document_type)
                            + storage_flags
                                .map(|s| s.serialized_size())
                                .unwrap_or_default(),
                    ),
                }
            };

            // here we should return an error if the element already exists
            let inserted = self.batch_insert_if_not_exists(
                path_key_element_info,
                apply_type,
                transaction,
                batch_operations,
                drive_version,
            )?;
            if !inserted {
                return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                    "reference already exists",
                )));
            }
        }
        Ok(())
    }
}
