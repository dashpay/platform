use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE, DEFAULT_FLOAT_SIZE,
    DEFAULT_FLOAT_SIZE_U8, DEFAULT_HASH_SIZE_U8,
};
use crate::drive::document::contract_documents_keeping_history_primary_key_path_for_document_id;
use crate::drive::flags::StorageFlags;

use crate::drive::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods};
use crate::drive::Drive;

use crate::error::Error;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;

use std::collections::HashMap;

impl Drive {
    /// Adds estimated storage costs for adding a document to primary storage.
    ///
    /// This function computes and updates the expected costs associated with storing
    /// a document in primary storage. Depending on the type and history preservation
    /// properties of the document, the costs are determined differently.
    ///
    /// - If the document type retains history, the function will account for costs
    ///   associated with trees and potential flags for deletion.
    /// - Otherwise, the function will only account for the cost of storing the elements.
    ///
    /// # Arguments
    /// * `document_and_contract_info`: Information about the document and its associated contract.
    /// * `primary_key_path`: Key path where the document should be stored in primary storage.
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a hashmap where the estimated layer
    ///    information will be stored for the given key path.
    /// * `platform_version`: Version of the platform being used, potentially affecting some estimates.
    ///
    /// # Returns
    /// * `Result<(), Error>`: Returns `Ok(())` if the operation succeeds, otherwise it returns an `Error`.
    ///
    /// # Errors
    /// This function might return an `Error` if there's a problem estimating the document's size for the
    /// given platform version.
    ///
    /// # Panics
    /// This function will not panic under normal circumstances. However, unexpected behavior may result
    /// from incorrect arguments or unforeseen edge cases.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_add_document_to_primary_storage_v0(
        document_and_contract_info: &DocumentAndContractInfo,
        primary_key_path: [&[u8]; 5],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let document = if let Some(document) = document_and_contract_info
            .owned_document_info
            .document_info
            .get_borrowed_document()
        {
            document
        } else {
            return Ok(());
        };
        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        // at this level we have all the documents for the contract
        if document_type.documents_keep_history() {
            // if we keep history this level has trees
            // we only keep flags if the contract can be deleted
            let average_flags_size = if contract.config().can_be_deleted() {
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
                    contract.id_ref().as_bytes(),
                    document_type.name().as_str(),
                    document.id_ref().as_slice(),
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
                            document_type.estimated_size(platform_version)? as u32,
                            average_flags_size,
                            AVERAGE_NUMBER_OF_UPDATES,
                        )),
                        references_size: Some((1, reference_size, average_flags_size, 1)),
                    },
                },
            );
        } else {
            // we just have the elements
            let approximate_size = if document_type.documents_mutable() {
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
                        document_type.estimated_size(platform_version)? as u32,
                        Some(flags_size),
                    ),
                },
            );
        }
        Ok(())
    }
}
