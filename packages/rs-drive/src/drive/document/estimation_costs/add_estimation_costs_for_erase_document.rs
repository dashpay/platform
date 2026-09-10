use std::collections::HashMap;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::AllProvableCountTrees;
use grovedb::TreeType;

use crate::drive::document::paths::{contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;

impl Drive {
    /// Registers the layers an erase chunk touches: the type's history tree,
    /// the document's own revision tree, and the lifecycle tree the record is
    /// written to or removed from.
    ///
    /// The revision tree is sized for a document that may retain far more
    /// revisions than a chunk removes, so its height is bounded rather than
    /// derived from the chunk size; an estimate keyed to the chunk would
    /// understate the merk path of a long history.
    pub(crate) fn add_estimation_costs_for_erase_document(
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        layers: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
            contract,
            layers,
            &platform_version.drive,
        )?;
        Self::add_estimation_costs_for_lifecycle_record(
            contract,
            document_type,
            layers,
            platform_version,
        )?;

        let flags_size = Some(StorageFlags::approximate_size(true, None));
        let mut history_root = contract_document_type_path_vec(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );
        history_root.push(vec![DOCUMENT_HISTORY_TREE_KEY]);
        layers.insert(
            KeyInfoPath::from_known_owned_path(history_root.clone()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(32, AllProvableCountTrees, flags_size),
            },
        );

        let mut history_path = history_root;
        history_path.push(document_id.to_vec());
        layers.insert(
            KeyInfoPath::from_known_owned_path(history_path),
            EstimatedLayerInformation {
                tree_type: TreeType::ProvableCountTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    16,
                    document_type.max_size(platform_version)? as u32,
                    flags_size,
                ),
            },
        );
        Ok(())
    }
}
