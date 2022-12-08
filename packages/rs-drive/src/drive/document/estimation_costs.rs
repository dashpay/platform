use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
    CONTRACT_DOCUMENTS_PATH_HEIGHT, DEFAULT_FLOAT_SIZE, DEFAULT_FLOAT_SIZE_U8,
    DEFAULT_HASH_SIZE_U8,
};
use crate::drive::document::contract_documents_keeping_history_primary_key_path_for_document_id;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::{EstimatedIntermediateFlagSizes, EstimatedValueSize};

use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use dpp::data_contract::extra::DriveContractExt;
use grovedb::batch::KeyInfoPath;
use grovedb::Error::MerkError;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerInformation::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
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
                PotentiallyAtMaxElements(AllSubtrees(DEFAULT_HASH_SIZE_U8, average_flags_size)),
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
                ApproximateElements(
                    AVERAGE_NUMBER_OF_UPDATES as u32,
                    Mix {
                        subtrees_size: None,
                        items_size: Some((
                            DEFAULT_FLOAT_SIZE_U8,
                            document_type.estimated_size() as u32,
                            average_flags_size,
                            AVERAGE_NUMBER_OF_UPDATES,
                        )),
                        references_size: Some((1, reference_size, average_flags_size, 1)),
                    },
                ),
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
                PotentiallyAtMaxElements(AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    document_type.estimated_size() as u32,
                    Some(flags_size),
                )),
            );
        }
    }
    //
    // /// Adds estimation costs for the index reference
    // fn add_estimation_costs_for_index_reference(
    //     mut index_path_info: PathInfo<0>,
    //     unique: bool,
    //     any_fields_can_be_null: bool,
    //     average_flags_size: &Option<u32>,
    //     estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    // ) -> Result<(), Error> {
    //     // unique indexes will be stored under key "0"
    //     // non unique indices should have a tree at key "0" that has all elements based off of primary key
    //     if !unique || any_fields_can_be_null {
    //         index_path_info.push(Key(vec![0]))?;
    //
    //         // On this level we will have a 0 and all the top index paths
    //         estimated_costs_only_with_layer_info.insert(
    //             index_path_info.convert_to_key_info_path(),
    //             PotentiallyAtMaxElements(AllSubtrees(
    //                 DEFAULT_HASH_SIZE_U8,
    //                 average_flags_size.clone(),
    //             )),
    //         );
    //     }
    //     Ok(())
    // }
    //
    // /// Adds indices for an index level and recurses.
    // fn add_estimation_costs_for_index_level(
    //     document_info: &DocumentInfo,
    //     document_type: &DocumentType,
    //     index_path_info: PathInfo<0>,
    //     index_level: &IndexLevel,
    //     mut any_fields_can_be_null: bool,
    //     average_flags_size: &Option<u32>,
    //     estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    // ) -> Result<(), Error> {
    //     if let Some(unique) = index_level.has_index_with_uniqueness {
    //         Self::add_estimation_costs_for_index_reference(
    //             index_path_info.clone(),
    //             unique,
    //             any_fields_can_be_null,
    //             average_flags_size,
    //             estimated_costs_only_with_layer_info,
    //         )?;
    //     }
    //
    //     let sub_level_index_count = index_level.sub_index_levels.len() as u32;
    //
    //     // On this level we will have a 0 and all the top index paths
    //     estimated_costs_only_with_layer_info.insert(
    //         index_path_info.clone().convert_to_key_info_path(),
    //         ApproximateElements(
    //             sub_level_index_count + 1,
    //             AllSubtrees(DEFAULT_HASH_SIZE_U8, average_flags_size.clone()),
    //         ),
    //     );
    //
    //     // fourth we need to store a reference to the document for each index
    //     for (name, sub_level) in &index_level.sub_index_levels {
    //         let mut sub_level_index_path_info = index_path_info.clone();
    //         let index_property_key = KeyRef(name.as_bytes());
    //
    //         let document_index_field = document_info
    //             .get_raw_for_document_type(name, document_type, None)?
    //             .unwrap_or_default();
    //
    //         sub_level_index_path_info.push(index_property_key)?;
    //
    //         let document_top_field_estimated_size =
    //             document_info.get_estimated_size_for_document_type(name, document_type)?;
    //
    //         if document_top_field_estimated_size > u8::MAX as u16 {
    //             return Err(Error::Fee(FeeError::Overflow(
    //                 "document top field is too big for being an index",
    //             )));
    //         }
    //
    //         estimated_costs_only_with_layer_info.insert(
    //             sub_level_index_path_info
    //                 .clone()
    //                 .convert_to_key_info_path(),
    //             PotentiallyAtMaxElements(AllSubtrees(
    //                 document_top_field_estimated_size as u8,
    //                 average_flags_size.clone(),
    //             )),
    //         );
    //
    //         // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
    //         // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference
    //
    //         any_fields_can_be_null |= document_type.field_can_be_null(name);
    //
    //         // we push the actual value of the index path
    //         sub_level_index_path_info.push(document_index_field)?;
    //         // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
    //         // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
    //         Self::add_estimation_costs_for_index_level(
    //             document_info,
    //             document_type,
    //             sub_level_index_path_info,
    //             sub_level,
    //             any_fields_can_be_null,
    //             average_flags_size,
    //             estimated_costs_only_with_layer_info,
    //         )?;
    //     }
    //     Ok(())
    // }
    //
    // /// Adds indices for the top index level and calls for lower levels.
    // pub(crate) fn add_estimation_costs_for_top_index_level(
    //     contract: &Contract,
    //     document_type: &DocumentType,
    //     estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    // ) -> Result<(), Error> {
    //     let average_flags_size = if document_type.documents_mutable || contract.can_be_deleted() {
    //         // the trees flags will never change
    //         let flags_size = StorageFlags::approximate_size(true, None);
    //         Some(flags_size)
    //     } else {
    //         None
    //     };
    //
    //     // we need to construct the path for documents on the contract
    //     // the path is
    //     //  * Document and Contract root tree
    //     //  * Contract ID recovered from document
    //     //  * 0 to signify Documents and not Contract
    //     let contract_document_type_path =
    //         contract_document_type_path_vec(contract.id.as_bytes(), document_type.name.as_str());
    //
    //     let sub_level_index_count = document_type.index_structure.sub_index_levels.len() as u32;
    //
    //     // On this level we will have a 0 and all the top index paths
    //     estimated_costs_only_with_layer_info.insert(
    //         KeyInfoPath::from_known_owned_path(contract_document_type_path.clone()),
    //         ApproximateElements(
    //             sub_level_index_count + 1,
    //             AllSubtrees(DEFAULT_HASH_SIZE_U8, average_flags_size),
    //         ),
    //     );
    //
    //     let document_info = DocumentEstimatedAverageSize(document_type.estimated_size() as u32);
    //
    //     // next we need to store a reference to the document for each index
    //     for (name, sub_level) in &document_type.index_structure.sub_index_levels {
    //         // at this point the contract path is to the contract documents
    //         // for each index the top index component will already have been added
    //         // when the contract itself was created
    //         let mut index_path: Vec<Vec<u8>> = contract_document_type_path.clone();
    //         index_path.push(Vec::from(name.as_bytes()));
    //
    //         // with the example of the dashpay contract's first index
    //         // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId
    //         let document_top_field = document_info
    //             .get_raw_for_document_type(name, document_type, None)?
    //             .unwrap_or_default();
    //
    //         let document_top_field_estimated_size =
    //             document_info.get_estimated_size_for_document_type(name, document_type)?;
    //
    //         if document_top_field_estimated_size > u8::MAX as u16 {
    //             return Err(Error::Fee(FeeError::Overflow(
    //                 "document top field is too big for being an index",
    //             )));
    //         }
    //
    //         // On this level we will have all the user defined values for the paths
    //         estimated_costs_only_with_layer_info.insert(
    //             KeyInfoPath::from_known_owned_path(index_path.clone()),
    //             PotentiallyAtMaxElements(AllSubtrees(
    //                 document_top_field_estimated_size as u8,
    //                 average_flags_size,
    //             )),
    //         );
    //
    //         let any_fields_can_be_null = document_type.field_can_be_null(name);
    //
    //         let mut index_path_info =
    //             PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(index_path));
    //
    //         // we push the actual value of the index path
    //         index_path_info.push(document_top_field)?;
    //         // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>
    //
    //         Self::add_estimation_costs_for_index_level(
    //             &document_info,
    //             document_type,
    //             index_path_info,
    //             sub_level,
    //             any_fields_can_be_null,
    //             &average_flags_size,
    //             estimated_costs_only_with_layer_info,
    //         )?;
    //     }
    //     Ok(())
    // }

    pub(super) fn stateless_delete_for_costs(
        element_estimated_size: u32,
        key_info_path: &KeyInfoPath,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
    ) -> Result<Option<(EstimatedValueSize, EstimatedIntermediateFlagSizes)>, Error> {
        // if estimated_costs_only_with_layer_info.is_some() {
        //     for k in estimated_costs_only_with_layer_info.as_ref().unwrap().keys() {
        //         let path = k.to_path().iter()
        //             .map(|k| hex::encode(k.as_slice()))
        //             .join("/");
        //         dbg!(path);
        //     }
        // }
        estimated_costs_only_with_layer_info.as_ref().map_or(
            Ok::<Option<(u32, IntMap<u32>)>, Error>(None),
            |layer_info| {
                let flags_size_map = (CONTRACT_DOCUMENTS_PATH_HEIGHT..(key_info_path.len() as u16))
                    .into_iter()
                    .map(|s| {
                        let subpath =
                            KeyInfoPath::from_vec(key_info_path.0[..(s as usize)].to_vec());
                        let size = layer_info
                            .get(&subpath)
                            .map(|estimated_layer_information| estimated_layer_information.sizes())
                            .ok_or(Error::Fee(FeeError::CorruptedEstimatedLayerInfoMissing(
                                format!(
                                    "layer info missing at path {}",
                                    subpath
                                        .0
                                        .iter()
                                        .map(|k| hex::encode(k.as_slice()))
                                        .join("/")
                                ),
                            )))?;

                        let flag_size = size
                            .layered_flags_size()
                            .map_err(MerkError)?
                            .unwrap_or_default();
                        Ok((s as u64, flag_size))
                    })
                    .collect::<Result<IntMap<u32>, Error>>()?;
                Ok(Some((element_estimated_size, flags_size_map)))
            },
        )
    }
}
