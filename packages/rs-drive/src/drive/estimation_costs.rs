use crate::drive::contract::{all_contracts_global_root_path, contract_root_path};
use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE};

use crate::drive::flags::StorageFlags;
use crate::drive::{contract_documents_path, Drive};
use dpp::data_contract::extra::DriveContractExt;
use dpp::data_contract::DataContract;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerInformation::{
    ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements,
};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use std::collections::HashMap;

impl Drive {
    pub(crate) fn add_estimation_costs_for_levels_up_to_contract(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLevel(0, false, AllSubtrees(1, None)),
        );

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(all_contracts_global_root_path()),
            PotentiallyAtMaxElements(AllSubtrees(
                DEFAULT_HASH_SIZE_U8,
                Some(StorageFlags::approximate_size(true, None)),
            )),
        );
    }

    pub(crate) fn add_estimation_costs_for_levels_up_to_contract_document_type(
        contract: &DataContract,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        Self::add_estimation_costs_for_levels_up_to_contract(estimated_costs_only_with_layer_info);

        let document_type_count = contract.documents.len() as u32;

        // we only store the owner_id storage
        let storage_flags = if contract.can_be_deleted() {
            Some(StorageFlags::approximate_size(true, None))
        } else {
            None
        };

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_root_path(contract.id.as_bytes())),
            EstimatedLevel(1, false, AllSubtrees(1, storage_flags)),
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_documents_path(contract.id.as_bytes())),
            ApproximateElements(
                document_type_count,
                AllSubtrees(ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE, storage_flags),
            ),
        );
    }

    // fn add_estimated_costs_for_indices_at_index_level(
    //     index_level: &IndexLevel,
    //     current_path: &[&[u8]],
    //     contract: &DataContract,
    //     document_type: &DocumentType,
    //     estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    // ) {
    //     let top_level_indices = index_level.sub_index_levels;
    //
    //     // we only store the owner_id storage
    //     let storage_flags = if !contract.readonly() {
    //         // the contract can maybe mutate the index names
    //         // however for now we will expect this to be so seldom that it can be discounted for estimates on costs
    //         Some(StorageFlags::approximate_size(true, None))
    //     } else if contract.can_be_deleted() {
    //         Some(StorageFlags::approximate_size(true, None))
    //     } else {
    //         None
    //     };
    //
    //     // we then need to insert the contract layer
    //     estimated_costs_only_with_layer_info.insert(
    //         KeyInfoPath::from_known_path(contract_document_type_path(
    //             contract.id.as_bytes(),
    //             document_type.name.as_str(),
    //         )),
    //         ApproximateElements(
    //             top_level_indices.len() as u32,
    //             AllSubtrees(DEFAULT_HASH_SIZE_U8, storage_flags),
    //         ),
    //     );
    // }
    //
    // pub(crate) fn add_estimated_costs_for_indices_for_document_type(
    //     contract: &DataContract,
    //     document_type: &DocumentType,
    //     estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    // ) {
    //     let top_index_level = &document_type.index_structure.sub_index_levels;
    //     let current_path =
    //         contract_document_type_path(contract.id.as_bytes(), document_type.name.as_str());
    //
    //     // the top level is different because the storage flags are different
    //
    //     // we only store the owner_id storage
    //     let storage_flags = if !contract.readonly() {
    //         // the contract can maybe mutate the index names
    //         // however for now we will expect this to be so seldom that it can be discounted for estimates on costs
    //         Some(StorageFlags::approximate_size(true, None))
    //     } else if contract.can_be_deleted() {
    //         Some(StorageFlags::approximate_size(true, None))
    //     } else {
    //         None
    //     };
    //
    //     // we then need to insert the contract layer
    //     estimated_costs_only_with_layer_info.insert(
    //         KeyInfoPath::from_known_path(current_path),
    //         ApproximateElements(
    //             top_level_indices.len() as u32,
    //             AllSubtrees(DEFAULT_HASH_SIZE_U8, storage_flags),
    //         ),
    //     );
    //
    //     let mut current_path_vec = current_path.to_vec();
    //
    //     for (path, lower_index_level) in top_index_level {
    //         current_path_vec.push(path.as_bytes());
    //         Self::add_estimated_costs_for_indices_at_index_level(
    //             lower_index_level,
    //             current_path_vec.as_slice(),
    //             contract,
    //             document_type,
    //             estimated_costs_only_with_layer_info,
    //         );
    //     }
    // }
    //
    // pub(crate) fn add_estimated_costs_for_index_at_path_for_document_type<'p, P>(
    //     path: P,
    //     contract: &DataContract,
    //     document_type: &DocumentType,
    //     estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    // ) where
    //     P: IntoIterator<Item = Vec<u8>>,
    //     <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    // {
    //     let top_level_indices = document_type.top_level_indices();
    //
    //     // we only store the owner_id storage
    //     let storage_flags = if !contract.readonly() {
    //         // the contract can maybe mutate the index names
    //         // however for now we will expect this to be so seldom that it can be discounted for estimates on costs
    //         Some(StorageFlags::approximate_size(true, None))
    //     } else if contract.can_be_deleted() {
    //         Some(StorageFlags::approximate_size(true, None))
    //     } else {
    //         None
    //     };
    //
    //     // we then need to insert the contract layer
    //     estimated_costs_only_with_layer_info.insert(
    //         KeyInfoPath::from_known_owned_path(path),
    //         ApproximateElements(
    //             top_level_indices.len() as u32,
    //             AllSubtrees(DEFAULT_HASH_SIZE_U8, storage_flags),
    //         ),
    //     );
    // }
}
