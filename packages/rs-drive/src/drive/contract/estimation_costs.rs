use crate::drive::contract::contract_keeping_history_storage_path;
use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, DEFAULT_FLOAT_SIZE, DEFAULT_FLOAT_SIZE_U8,
    ESTIMATED_AVERAGE_INDEX_NAME_SIZE,
};
use crate::drive::document::contract_document_type_path;
use crate::drive::flags::StorageFlags;
use crate::drive::Drive;

use dpp::data_contract::{DataContract, DriveContractExt};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimation costs for a contract insertion
    pub(super) fn add_estimation_costs_for_contract_insertion(
        contract: &DataContract,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
            contract,
            estimated_costs_only_with_layer_info,
        );

        // we only store the owner_id storage
        let storage_flags = if contract.can_be_deleted() || !contract.readonly() {
            Some(StorageFlags::approximate_size(true, None))
        } else {
            None
        };

        for (document_type_name, _) in contract.document_types() {
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_document_type_path(
                    contract.id.as_bytes(),
                    document_type_name.as_str(),
                )),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: EstimatedLevel(0, true),
                    estimated_layer_sizes: AllSubtrees(
                        ESTIMATED_AVERAGE_INDEX_NAME_SIZE,
                        NoSumTrees,
                        storage_flags,
                    ),
                },
            );
        }

        if contract.keeps_history() {
            // we are dealing with a sibling reference
            // sibling reference serialized size is going to be the encoded time size
            // (DEFAULT_FLOAT_SIZE) plus 1 byte for reference type and 1 byte for the space of
            // the encoded time
            let reference_size = DEFAULT_FLOAT_SIZE + 2;

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_keeping_history_storage_path(
                    contract.id.as_bytes(),
                )),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(AVERAGE_NUMBER_OF_UPDATES as u32),
                    estimated_layer_sizes: Mix {
                        subtrees_size: None,
                        items_size: Some((
                            DEFAULT_FLOAT_SIZE_U8,
                            contract.to_cbor().unwrap().len() as u32, //todo: fix this
                            storage_flags,
                            AVERAGE_NUMBER_OF_UPDATES,
                        )),
                        references_size: Some((1, reference_size, storage_flags, 1)),
                    },
                },
            );
        }
    }
}
