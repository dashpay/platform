use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllItems;

use dpp::data_contract::document_type::DocumentTypeRef;

use std::collections::HashMap;

use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE, DEFAULT_HASH_SIZE_U8,
};

use crate::drive::flags::StorageFlags;

use crate::drive::Drive;

use crate::error::Error;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;

use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn add_estimation_costs_for_remove_document_to_primary_storage_v0(
        primary_key_path: [&[u8]; 5],
        document_type: DocumentTypeRef,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // we just have the elements
        let approximate_size = if document_type.documents_read_only() {
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
