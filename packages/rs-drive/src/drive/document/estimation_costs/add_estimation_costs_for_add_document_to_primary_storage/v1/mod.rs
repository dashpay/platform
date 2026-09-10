use crate::drive::constants::DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE;
use crate::drive::document::paths::DOCUMENT_HISTORY_TREE_KEY;
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods};
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use grovedb::batch::{key_info::KeyInfo, KeyInfoPath};
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_add_document_to_primary_storage_v1(
        info: &DocumentAndContractInfo,
        primary_path: [&[u8]; 5],
        layers: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if !info.document_type.documents_keep_history() {
            return Self::add_estimation_costs_for_add_document_to_primary_storage_v0(
                info,
                primary_path,
                layers,
                platform_version,
            );
        }
        let flags_size = Some(StorageFlags::approximate_size(true, None));
        let references = if info.document_type.documents_summable().is_some() {
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
        layers.insert(
            KeyInfoPath::from_known_path(primary_path),
            EstimatedLayerInformation {
                tree_type: info.document_type.primary_key_tree_type(platform_version)?,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: references,
            },
        );
        let mut root = primary_path.map(|part| part.to_vec()).to_vec();
        root[4] = vec![DOCUMENT_HISTORY_TREE_KEY];
        layers.insert(
            KeyInfoPath::from_known_path(root.iter().map(Vec::as_slice)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(32, NoSumTrees, flags_size),
            },
        );
        let mut path = KeyInfoPath::from_known_path(root.iter().map(Vec::as_slice));
        path.push(
            info.owned_document_info
                .document_info
                .get_borrowed_document()
                .map_or_else(
                    || KeyInfo::MaxKeySize {
                        unique_id: info.document_type.unique_id_for_storage().to_vec(),
                        max_size: 32,
                    },
                    |document| KeyInfo::KnownKey(document.id().to_vec()),
                ),
        );
        layers.insert(
            path,
            EstimatedLayerInformation {
                tree_type: TreeType::ProvableCountTree,
                estimated_layer_count: ApproximateElements(
                    crate::drive::constants::AVERAGE_NUMBER_OF_UPDATES as u32,
                ),
                estimated_layer_sizes: AllItems(
                    16,
                    info.document_type.estimated_size(platform_version)? as u32,
                    flags_size,
                ),
            },
        );
        Ok(())
    }
}
