use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllReference, Mix};

use dpp::data_contract::document_type::DocumentTypeRef;

use std::collections::HashMap;

use crate::drive::constants::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
    DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE,
};
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;

use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::version::PlatformVersion;

impl Drive {
    /// Adds the estimation layer for removing one entry from a document type's
    /// primary-key tree.
    ///
    /// A keep-history type stores current pointers there rather than document
    /// items, so its layer is sized from the reference shape the writer
    /// installs; a summable type's pointers additionally carry the document's
    /// contribution. Every other document type is estimated exactly as v0 does.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_remove_document_to_primary_storage_v1(
        primary_key_path: [&[u8]; 5],
        document_type: DocumentTypeRef,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if !document_type.documents_keep_history() {
            return Self::add_estimation_costs_for_remove_document_to_primary_storage_v0(
                primary_key_path,
                document_type,
                estimated_costs_only_with_layer_info,
                platform_version,
            );
        }

        let approximate_size = if document_type.documents_mutable() {
            Some((
                AVERAGE_NUMBER_OF_UPDATES as u16,
                AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
            ))
        } else {
            None
        };
        let flags_size = Some(StorageFlags::approximate_size(true, approximate_size));
        let references = if document_type.documents_summable().is_some() {
            Mix {
                subtrees_size: None,
                items_size: None,
                references_size: None,
                items_with_sum_item_size: None,
                references_with_sum_item_size: Some((
                    32,
                    DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE,
                    flags_size,
                    1,
                )),
            }
        } else {
            AllReference(32, DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE, flags_size)
        };
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(primary_key_path),
            EstimatedLayerInformation {
                tree_type: document_type.primary_key_tree_type(platform_version)?,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: references,
            },
        );

        Ok(())
    }
}
