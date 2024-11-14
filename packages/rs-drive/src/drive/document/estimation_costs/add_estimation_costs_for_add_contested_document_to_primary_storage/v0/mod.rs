use crate::drive::constants::{AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE};
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods};

use crate::error::Error;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllItems;

use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
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
    pub(super) fn add_estimation_costs_for_add_contested_document_to_primary_storage_v0<
        const N: usize,
    >(
        document_and_contract_info: &DocumentAndContractInfo,
        primary_key_path: [&[u8]; N],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if document_and_contract_info
            .owned_document_info
            .document_info
            .get_borrowed_document()
            .is_none()
        {
            return Ok(());
        };
        let document_type = document_and_contract_info.document_type;
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
        Ok(())
    }
}
