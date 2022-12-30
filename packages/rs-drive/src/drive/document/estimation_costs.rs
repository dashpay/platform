use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
    CONTRACT_DOCUMENTS_PATH_HEIGHT, DEFAULT_FLOAT_SIZE, DEFAULT_FLOAT_SIZE_U8,
    DEFAULT_HASH_SIZE_U8,
};
use crate::drive::document::contract_documents_keeping_history_primary_key_path_for_document_id;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::{BatchDeleteUpTreeApplyType, IsSubTree, IsSumSubTree};

use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use dpp::data_contract::extra::DriveContractExt;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, EstimatedLayerSizes};
use intmap::IntMap;
use itertools::Itertools;
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_add_document_to_primary_storage(
        document_and_contract_info: &DocumentAndContractInfo,
        primary_key_path: [&[u8]; 5],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        let document = if let Some(document) = document_and_contract_info
            .owned_document_info
            .document_info
            .get_borrowed_document()
        {
            document
        } else {
            return;
        };
        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        // at this level we have all the documents for the contract
        if document_type.documents_keep_history {
            // if we keep history this level has trees
            // we only keep flags if the contract can be deleted
            let average_flags_size = if contract.can_be_deleted() {
                // the trees flags will never change
                let flags_size = StorageFlags::approximate_size(true, None);
                Some(flags_size)
            } else {
                None
            };
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(primary_key_path),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        average_flags_size,
                    ),
                },
            );
            let document_id_in_primary_path =
                contract_documents_keeping_history_primary_key_path_for_document_id(
                    contract.id().as_bytes(),
                    document_type.name.as_str(),
                    document.id.as_slice(),
                );
            // we are dealing with a sibling reference
            // sibling reference serialized size is going to be the encoded time size
            // (DEFAULT_FLOAT_SIZE) plus 1 byte for reference type and 1 byte for the space of
            // the encoded time
            let reference_size = DEFAULT_FLOAT_SIZE + 2;
            // on the lower level we have many items by date, and 1 ref to the current item
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(document_id_in_primary_path),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(AVERAGE_NUMBER_OF_UPDATES as u32),
                    estimated_layer_sizes: Mix {
                        subtrees_size: None,
                        items_size: Some((
                            DEFAULT_FLOAT_SIZE_U8,
                            document_type.estimated_size() as u32,
                            average_flags_size,
                            AVERAGE_NUMBER_OF_UPDATES,
                        )),
                        references_size: Some((1, reference_size, average_flags_size, 1)),
                    },
                },
            );
        } else {
            // we just have the elements
            let approximate_size = if document_type.documents_mutable {
                //todo: have the contract say how often we expect documents to mutate
                Some((
                    AVERAGE_NUMBER_OF_UPDATES as u16,
                    AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
                ))
            } else {
                None
            };
            let flags_size = StorageFlags::approximate_size(true, approximate_size);
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(primary_key_path),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: AllItems(
                        DEFAULT_HASH_SIZE_U8,
                        document_type.estimated_size() as u32,
                        Some(flags_size),
                    ),
                },
            );
        }
    }

    pub(super) fn stateless_delete_of_non_tree_for_costs(
        element_estimated_sizes: EstimatedLayerSizes,
        key_info_path: &KeyInfoPath,
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
    ) -> Result<BatchDeleteUpTreeApplyType, Error> {
        // Keep for debugging
        // if estimated_costs_only_with_layer_info.is_some() {
        //     for (k, l) in estimated_costs_only_with_layer_info.as_ref().unwrap() {
        //         let path = k
        //             .to_path()
        //             .iter()
        //             .map(|k| hex::encode(k.as_slice()))
        //             .join("/");
        //         dbg!(path, l);
        //     }
        // }
        estimated_costs_only_with_layer_info.as_ref().map_or(
            Ok(BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum,
            }),
            |layer_info| {
                let mut layer_map = (CONTRACT_DOCUMENTS_PATH_HEIGHT..(key_info_path.len() as u16))
                    .into_iter()
                    .map(|s| {
                        let subpath =
                            KeyInfoPath::from_vec(key_info_path.0[..(s as usize)].to_vec());
                        let layer_info = layer_info.get(&subpath).ok_or(Error::Fee(
                            FeeError::CorruptedEstimatedLayerInfoMissing(format!(
                                "layer info missing at path {}",
                                subpath
                                    .0
                                    .iter()
                                    .map(|k| hex::encode(k.as_slice()))
                                    .join("/")
                            )),
                        ))?;

                        Ok((s as u64, layer_info.clone()))
                    })
                    .collect::<Result<IntMap<EstimatedLayerInformation>, Error>>()?;
                // We need to update the current layer to only have 1 element that we want to delete
                let mut last_layer_information = layer_map
                    .remove((key_info_path.len() - 1) as u64)
                    .ok_or(Error::Fee(FeeError::CorruptedEstimatedLayerInfoMissing(
                        "last layer info missing".to_owned(),
                    )))?;
                last_layer_information.estimated_layer_sizes = element_estimated_sizes;
                layer_map.insert((key_info_path.len() - 1) as u64, last_layer_information);
                Ok(BatchDeleteUpTreeApplyType::StatelessBatchDelete {
                    estimated_layer_info: layer_map,
                })
            },
        )
    }
}
