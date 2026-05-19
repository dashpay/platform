use crate::drive::constants::{AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE};
use crate::drive::document::paths::contract_documents_keeping_history_primary_key_path_for_document_id;
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods};

use crate::error::Error;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};

use crate::util::type_constants::{
    DEFAULT_FLOAT_SIZE, DEFAULT_FLOAT_SIZE_U8, DEFAULT_HASH_SIZE_U8,
};
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
    ///   information will be stored for the given key path.
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
        let primary_key_tree_type = document_type.primary_key_tree_type(platform_version)?;
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
                    tree_type: primary_key_tree_type,
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

            // Per-document subtree variant + layer-size mix.
            //
            // - Non-summable keep-history (the original case):
            //   `NormalTree` per-doc subtree, version bodies are
            //   plain `Item`s, `0`-key is a plain `Reference`.
            // - Summable keep-history (added by the
            //   `ReferenceWithSumItem`-on-`0`-key change):
            //   `SumTree` per-doc subtree (so its aggregate
            //   propagates to the doctype-level SumTree), version
            //   bodies STAY plain `Item`s (the sum is on the
            //   `0`-key reference, not the version), `0`-key is a
            //   `ReferenceWithSumItem`. The estimation has to
            //   match the applied layout byte-for-byte so dry-run
            //   fee estimation doesn't undercharge the inserts.
            let summable = document_type.documents_summable().is_some();
            let (per_doc_subtree_tree_type, references_size, references_with_sum_item_size) =
                if summable {
                    // ReferenceWithSumItem carries the same path bytes as
                    // a plain Reference plus a 10-byte worst-case varint
                    // for the i64 sum_value. We account for that by
                    // moving the ref slot into the sum-bearing column —
                    // grovedb computes the per-variant overhead
                    // internally from the column it's filed under.
                    (
                        TreeType::SumTree,
                        None,
                        Some((1, reference_size, average_flags_size, 1)),
                    )
                } else {
                    (
                        TreeType::NormalTree,
                        Some((1, reference_size, average_flags_size, 1)),
                        None,
                    )
                };
            // on the lower level we have many items by date, and 1 ref to the current item
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(document_id_in_primary_path),
                EstimatedLayerInformation {
                    tree_type: per_doc_subtree_tree_type,
                    estimated_layer_count: ApproximateElements(AVERAGE_NUMBER_OF_UPDATES as u32),
                    estimated_layer_sizes: Mix {
                        subtrees_size: None,
                        // Version bodies are always plain `Item`s under
                        // keep-history (regardless of summable) — the
                        // sum, if any, lives on the `0`-key reference.
                        items_size: Some((
                            DEFAULT_FLOAT_SIZE_U8,
                            document_type.estimated_size(platform_version)? as u32,
                            average_flags_size,
                            AVERAGE_NUMBER_OF_UPDATES,
                        )),
                        references_size,
                        items_with_sum_item_size: None,
                        references_with_sum_item_size,
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
                    tree_type: primary_key_tree_type,
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
