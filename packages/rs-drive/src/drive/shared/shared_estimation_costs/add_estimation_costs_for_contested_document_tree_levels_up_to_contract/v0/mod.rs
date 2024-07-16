use crate::drive::constants::{
    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE, ESTIMATED_AVERAGE_INDEX_NAME_SIZE,
};

use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllSubtrees;

use crate::drive::votes::paths::{
    vote_contested_resource_active_polls_contract_document_tree_path,
    vote_contested_resource_active_polls_contract_tree_path,
    vote_contested_resource_active_polls_tree_path,
    vote_contested_resource_contract_documents_indexes_path, vote_contested_resource_tree_path,
    vote_root_path,
};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use std::collections::HashMap;

impl Drive {
    /// Adds estimated costs for layers up to the contract level.
    ///
    /// This function populates the `estimated_costs_only_with_layer_info` hashmap with estimated layer information for the top level and the contract layer.
    /// These estimates are useful for optimizing GroveDB operations.
    ///
    /// # Parameters
    ///
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a hashmap that will be populated with estimated layer information for the top level and the contract layer.
    ///
    /// # Estimated Layer Information
    ///
    /// The function estimates two layers:
    ///
    /// 1. The top layer, which is an empty layer.
    /// 2. The contract layer, which contains all global contracts.
    ///
    /// These estimates are useful for optimizing batch insertions, deletions, and other operations in GroveDB.
    ///
    /// # Usage
    ///
    /// This function is intended to be used internally within the Drive implementation.
    ///
    pub(in crate::drive) fn add_estimation_costs_for_contested_document_tree_levels_up_to_contract_v0<
        'a,
    >(
        contract: &'a DataContract,
        document_type: Option<DocumentTypeRef<'a>>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(2, false), //voting is on level 2
                // We have balances in the middle which is a sum tree
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        non_sum_trees_weight: 2,
                    },
                    None,
                ),
            },
        );

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(vote_root_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                // contested resource tree is a key of "c" so it should be on the top layer of the merk
                estimated_layer_count: EstimatedLevel(0, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(vote_contested_resource_tree_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                // active poll "p", with "e" and "i" first so it should be on the second layer of the merk
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(vote_contested_resource_active_polls_tree_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        let document_type_count = contract.document_types().len() as u32;

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(vote_contested_resource_active_polls_contract_tree_path(
                contract.id_ref().as_bytes(),
            )),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(document_type_count),
                estimated_layer_sizes: AllSubtrees(
                    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE,
                    NoSumTrees,
                    None,
                ),
            },
        );

        if let Some(document_type) = document_type {
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(
                    vote_contested_resource_active_polls_contract_document_tree_path(
                        contract.id_ref().as_bytes(),
                        document_type.name().as_str(),
                    ),
                ),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(2),
                    estimated_layer_sizes: AllSubtrees(
                        ESTIMATED_AVERAGE_INDEX_NAME_SIZE,
                        NoSumTrees,
                        None,
                    ),
                },
            );

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(
                    vote_contested_resource_contract_documents_indexes_path(
                        contract.id_ref().as_bytes(),
                        document_type.name().as_str(),
                    ),
                ),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(1024), //Just a guess
                    estimated_layer_sizes: AllSubtrees(
                        ESTIMATED_AVERAGE_INDEX_NAME_SIZE,
                        NoSumTrees,
                        None,
                    ),
                },
            );
        }
    }
}
