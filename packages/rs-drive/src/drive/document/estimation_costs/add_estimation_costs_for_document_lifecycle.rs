use std::collections::HashMap;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::TreeType;

use crate::drive::document::lifecycle::DOCUMENT_LIFECYCLE_RECORD_SIZE;
use crate::drive::document::paths::{contract_document_type_path_vec, document_lifecycle_path};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;

impl Drive {
    /// Registers the layers a lifecycle record write or removal touches: the
    /// document type itself, whose lifecycle tree may have to be created, and
    /// the lifecycle tree holding one fixed-size record per deleted document.
    ///
    /// A dry run that leaves either layer unregistered fails outright, so both
    /// are always registered even when the tree already exists.
    pub(crate) fn add_estimation_costs_for_lifecycle_record(
        contract: &DataContract,
        document_type: DocumentTypeRef,
        layers: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let flags_size = Some(StorageFlags::approximate_size(true, None));
        layers.insert(
            KeyInfoPath::from_known_owned_path(contract_document_type_path_vec(
                contract.id_ref().as_bytes(),
                document_type.name().as_str(),
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                // The primary-key tree, the lifecycle tree, the history tree
                // and one tree per index.
                estimated_layer_count: ApproximateElements(
                    document_type.indexes().len() as u32 + 3,
                ),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, flags_size),
            },
        );
        layers.insert(
            KeyInfoPath::from_known_owned_path(document_lifecycle_path(
                contract.id_ref().as_bytes(),
                document_type.name().as_str(),
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(32, DOCUMENT_LIFECYCLE_RECORD_SIZE, flags_size),
            },
        );
        Ok(())
    }
}
