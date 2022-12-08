use crate::drive::contract::{
    all_contracts_global_root_path, contract_root_path,
};
use crate::drive::defaults::{
    DEFAULT_HASH_SIZE_U8,
    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE,
};


use crate::drive::flags::StorageFlags;
use crate::drive::{contract_documents_path, Drive};

use dpp::data_contract::extra::DriveContractExt;
use dpp::data_contract::DataContract;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerInformation::{
    ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements,
};
use grovedb::EstimatedLayerSizes::{AllSubtrees};

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

    pub(crate) fn add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
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
}
