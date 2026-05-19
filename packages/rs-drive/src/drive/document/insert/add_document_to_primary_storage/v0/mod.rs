use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use dpp::data_contract::document_type::DocumentPropertyType;

use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::SiblingReference;

use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};

use std::collections::HashMap;
use std::option::Option::None;

use crate::drive::constants::STORAGE_FLAGS_SIZE;
use crate::util::object_size_info::DocumentInfo::{
    DocumentAndSerialization, DocumentEstimatedAverageSize, DocumentOwnedInfo,
    DocumentRefAndSerialization, DocumentRefInfo,
};
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyUnknownElementSize,
};
use crate::util::object_size_info::PathKeyInfo::{PathFixedSizeKeyRef, PathKeySize};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods};

use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::DocumentV0Getters;

use crate::drive::document::paths::{
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_keeping_history_primary_key_path_for_unknown_document_id,
    contract_documents_keeping_history_storage_time_reference_path_size,
    contract_documents_primary_key_path,
};
use crate::drive::document::read_document_sum_contribution;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb_version::version::GroveVersion;

/// Build the primary-storage element wrapping a serialized document.
///
/// Returns `Element::ItemWithSumItem` when the doctype's primary
/// key tree is sum-bearing (`primary_key_sum_property = Some(_)`)
/// so the document's named integer property propagates into the
/// SumTree's running aggregate; `Element::Item` otherwise.
/// Centralizes the `match &primary_key_sum_property` branch that
/// would otherwise be open-coded at every insert site (5 in
/// `add_document_to_primary_storage_0`'s three top-level arms).
fn build_primary_element(
    document: &Document,
    serialized_document: Vec<u8>,
    element_flags: Option<Vec<u8>>,
    primary_key_sum_property: Option<&str>,
) -> Result<Element, Error> {
    match primary_key_sum_property {
        Some(prop_name) => {
            let sum_value = read_document_sum_contribution(document, prop_name)?;
            Ok(Element::new_item_with_sum_item_with_flags(
                serialized_document,
                sum_value,
                element_flags,
            ))
        }
        None => Ok(Element::Item(serialized_document, element_flags)),
    }
}

/// Estimated primary-storage element space for the cost-only path
/// (`DocumentEstimatedAverageSize` arms). Sum-aware parallel of
/// [`build_primary_element`]: returns `required_item_with_sum_item_space`
/// (10 extra bytes for the `i64` sum_value varint) when the
/// doctype is sum-bearing; `required_item_space` otherwise.
/// Keeping this in lock-step with `build_primary_element` is
/// load-bearing — fee estimation and applied execution must stay
/// in sync on summable inserts.
fn required_primary_element_space(
    max_size: u32,
    primary_key_sum_property: Option<&str>,
    grove_version: &GroveVersion,
) -> Result<u32, Error> {
    Ok(if primary_key_sum_property.is_some() {
        Element::required_item_with_sum_item_space(max_size, STORAGE_FLAGS_SIZE, grove_version)?
    } else {
        Element::required_item_space(max_size, STORAGE_FLAGS_SIZE, grove_version)?
    })
}

impl Drive {
    /// Adds a document to primary storage.
    /// If a document isn't sent to this function then we are just calling to know the query and
    /// insert operations
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_document_to_primary_storage_0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        block_info: &BlockInfo,
        insert_without_check: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;

        // The primary-key tree variant. Resolves to one of:
        //   NormalTree (default)
        //   CountTree / ProvableCountTree (count surfaces, pre-v3)
        //   SumTree / ProvableSumTree (sum surfaces, v3+)
        //   CountSumTree / ProvableCountSumTree (combined, v3+)
        // per the dispatch table in
        // `crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType::primary_key_tree_type`.
        let primary_key_tree_type = document_type.primary_key_tree_type(platform_version)?;

        // The name (if any) of the integer property whose value each
        // document contributes to the primary-key sum tree.
        // `Some(name)` flips every primary-storage element below from
        // `Element::Item` to `Element::ItemWithSumItem` via
        // [`build_primary_element`], and pairs every
        // `DocumentEstimatedAverageSize` cost arm with
        // [`required_primary_element_space`]'s sum-aware branch.
        // Centralizing both there keeps execution and estimation in
        // lock-step on summable inserts. The DPP validator
        // guarantees the named property exists, is an integer type,
        // and is in `required` — so the lookup + i64 conversion in
        // `read_document_sum_contribution` is infallible at the
        // contract level (a `CorruptedCodeExecution` would mean
        // contract validation was bypassed).
        let primary_key_sum_property: Option<String> =
            document_type.documents_summable().map(|s| s.to_string());

        let primary_key_path = contract_documents_primary_key_path(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );
        // if we are trying to get estimated costs we should add this level
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_add_document_to_primary_storage(
                document_and_contract_info,
                primary_key_path,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        if document_type.documents_keep_history() {
            let (path_key_info, storage_flags) = if document_and_contract_info
                .owned_document_info
                .document_info
                .is_document_size()
            {
                (
                    PathKeySize(
                        KeyInfoPath::from_known_path(primary_key_path),
                        KeyInfo::MaxKeySize {
                            unique_id: document_type.unique_id_for_storage().to_vec(),
                            max_size: DEFAULT_HASH_SIZE_U8,
                        },
                    ),
                    StorageFlags::optional_default_as_ref(),
                )
            } else {
                let inserted_storage_flags = if contract.config().can_be_deleted() {
                    document_and_contract_info
                        .owned_document_info
                        .document_info
                        .get_storage_flags_ref()
                } else {
                    // there are no need for storage flags if the contract can not be deleted
                    // as this tree can never be deleted
                    None
                };
                (
                    PathFixedSizeKeyRef((
                        primary_key_path,
                        document_and_contract_info
                            .owned_document_info
                            .document_info
                            .get_document_id_as_slice()
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "can not get document id from estimated document",
                            )))?,
                    )),
                    inserted_storage_flags,
                )
            };

            // The per-document history subtree is always NormalTree.
            // The parent (primary key tree) may be CountTree/ProvableCountTree.
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: primary_key_tree_type,
                    tree_type: TreeType::NormalTree,
                    flags_len: storage_flags
                        .map(|s| s.serialized_size())
                        .unwrap_or_default(),
                }
            };
            // we first insert an empty tree if the document is new
            // The per-document subtree is always NormalTree (it holds history entries)
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info,
                TreeType::NormalTree,
                storage_flags,
                apply_type,
                transaction,
                &mut None, //not going to have multiple same documents in same batch
                drive_operations,
                drive_version,
            )?;
            let encoded_time = DocumentPropertyType::encode_date_timestamp(block_info.time_ms);
            let path_key_element_info =
                match &document_and_contract_info.owned_document_info.document_info {
                    DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document.to_vec(),
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        let document_id_in_primary_path =
                            contract_documents_keeping_history_primary_key_path_for_document_id(
                                contract.id_ref().as_bytes(),
                                document_type.name().as_str(),
                                document.id_ref().as_slice(),
                            );
                        PathFixedSizeKeyRefElement((
                            document_id_in_primary_path,
                            encoded_time.as_slice(),
                            element,
                        ))
                    }
                    DocumentAndSerialization((document, serialized_document, storage_flags)) => {
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document.to_vec(),
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        let document_id_in_primary_path =
                            contract_documents_keeping_history_primary_key_path_for_document_id(
                                contract.id_ref().as_bytes(),
                                document_type.name().as_str(),
                                document.id_ref().as_slice(),
                            );
                        PathFixedSizeKeyRefElement((
                            document_id_in_primary_path,
                            encoded_time.as_slice(),
                            element,
                        ))
                    }
                    DocumentOwnedInfo((document, storage_flags)) => {
                        let serialized_document = document.serialize(
                            document_and_contract_info.document_type,
                            document_and_contract_info.contract,
                            platform_version,
                        )?;
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document,
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        let document_id_in_primary_path =
                            contract_documents_keeping_history_primary_key_path_for_document_id(
                                contract.id_ref().as_bytes(),
                                document_type.name().as_str(),
                                document.id_ref().as_slice(),
                            );
                        PathFixedSizeKeyRefElement((
                            document_id_in_primary_path,
                            encoded_time.as_slice(),
                            element,
                        ))
                    }
                    DocumentRefInfo((document, storage_flags)) => {
                        let serialized_document = document.serialize(
                            document_and_contract_info.document_type,
                            document_and_contract_info.contract,
                            platform_version,
                        )?;
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document,
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        let document_id_in_primary_path =
                            contract_documents_keeping_history_primary_key_path_for_document_id(
                                contract.id_ref().as_bytes(),
                                document_type.name().as_str(),
                                document.id_ref().as_slice(),
                            );
                        PathFixedSizeKeyRefElement((
                            document_id_in_primary_path,
                            encoded_time.as_slice(),
                            element,
                        ))
                    }
                    DocumentEstimatedAverageSize(max_size) => {
                        let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_unknown_document_id(
                            contract.id_ref().as_bytes(),
                            document_type,
                        );
                        // Mirror the live path on `primary_key_sum_property`:
                        // summable doctypes write `Element::ItemWithSumItem`
                        // (~10 extra bytes for the i64 sum_value varint), so
                        // the estimation must use the sum-aware helper to
                        // avoid undercharging keep-history inserts. Shared
                        // sum/non-sum dispatch lives in
                        // [`required_primary_element_space`].
                        let elem_size = required_primary_element_space(
                            *max_size,
                            primary_key_sum_property.as_deref(),
                            &platform_version.drive.grove_version,
                        )?;
                        PathKeyUnknownElementSize((
                            document_id_in_primary_path,
                            KnownKey(encoded_time.clone()),
                            elem_size,
                        ))
                    }
                };
            self.batch_insert(path_key_element_info, drive_operations, drive_version)?;
            let path_key_element_info = if document_and_contract_info
                .owned_document_info
                .document_info
                .is_document_size()
            {
                let document_id_in_primary_path =
                    contract_documents_keeping_history_primary_key_path_for_unknown_document_id(
                        contract.id_ref().as_bytes(),
                        document_type,
                    );
                let reference_max_size =
                    contract_documents_keeping_history_storage_time_reference_path_size(
                        document_type.name().len() as u32,
                    );
                PathKeyUnknownElementSize((
                    document_id_in_primary_path,
                    KnownKey(vec![0]),
                    Element::required_item_space(
                        reference_max_size,
                        STORAGE_FLAGS_SIZE,
                        &platform_version.drive.grove_version,
                    )?,
                ))
            } else {
                // we should also insert a reference at 0 to the current value
                // todo: we could construct this only once
                let document_id_in_primary_path =
                    contract_documents_keeping_history_primary_key_path_for_document_id(
                        contract.id_ref().as_bytes(),
                        document_type.name().as_str(),
                        document_and_contract_info
                            .owned_document_info
                            .document_info
                            .get_document_id_as_slice()
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "can not get document id from estimated document",
                            )))?,
                    );
                PathFixedSizeKeyRefElement((
                    document_id_in_primary_path,
                    &[0],
                    Element::Reference(
                        SiblingReference(encoded_time),
                        Some(1),
                        StorageFlags::map_to_some_element_flags(storage_flags),
                    ),
                ))
            };

            self.batch_insert(path_key_element_info, drive_operations, drive_version)?;
        } else if insert_without_check {
            let path_key_element_info =
                match &document_and_contract_info.owned_document_info.document_info {
                    DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document.to_vec(),
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                    DocumentAndSerialization((document, serialized_document, storage_flags)) => {
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document.to_vec(),
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                    DocumentRefInfo((document, storage_flags)) => {
                        let serialized_document = document.serialize(
                            document_and_contract_info.document_type,
                            document_and_contract_info.contract,
                            platform_version,
                        )?;
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document,
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                    DocumentEstimatedAverageSize(average_size) => {
                        // Same sum-aware branch as the keep-history and
                        // trailing-else arms — see
                        // [`required_primary_element_space`] for the
                        // shared sum/non-sum dispatch.
                        let elem_size = required_primary_element_space(
                            *average_size,
                            primary_key_sum_property.as_deref(),
                            &platform_version.drive.grove_version,
                        )?;
                        PathKeyUnknownElementSize((
                            KeyInfoPath::from_known_path(primary_key_path),
                            KeyInfo::MaxKeySize {
                                unique_id: document_type.unique_id_for_storage().to_vec(),
                                max_size: DEFAULT_HASH_SIZE_U8,
                            },
                            elem_size,
                        ))
                    }
                    DocumentOwnedInfo((document, storage_flags)) => {
                        let serialized_document = document.serialize(
                            document_and_contract_info.document_type,
                            document_and_contract_info.contract,
                            platform_version,
                        )?;
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document,
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                };
            self.batch_insert(path_key_element_info, drive_operations, drive_version)?;
        } else {
            let path_key_element_info =
                match &document_and_contract_info.owned_document_info.document_info {
                    DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document.to_vec(),
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                    DocumentAndSerialization((document, serialized_document, storage_flags)) => {
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document.to_vec(),
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                    DocumentOwnedInfo((document, storage_flags)) => {
                        let serialized_document = document.serialize(
                            document_and_contract_info.document_type,
                            document_and_contract_info.contract,
                            platform_version,
                        )?;
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document,
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                    DocumentRefInfo((document, storage_flags)) => {
                        let serialized_document = document.serialize(
                            document_and_contract_info.document_type,
                            document_and_contract_info.contract,
                            platform_version,
                        )?;
                        let element_flags =
                            StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags);
                        let element = build_primary_element(
                            document,
                            serialized_document,
                            element_flags,
                            primary_key_sum_property.as_deref(),
                        )?;
                        PathFixedSizeKeyRefElement((
                            primary_key_path,
                            document.id_ref().as_slice(),
                            element,
                        ))
                    }
                    DocumentEstimatedAverageSize(max_size) => {
                        // When the doctype's primary key tree is sum-bearing
                        // (`documents_summable: Some(_)`), the inserted element
                        // is `Element::ItemWithSumItem` — 10 extra bytes for the
                        // `i64` sum_value over plain `Item`. Shared sum/non-sum
                        // dispatch lives in [`required_primary_element_space`].
                        let elem_size = required_primary_element_space(
                            *max_size,
                            primary_key_sum_property.as_deref(),
                            &platform_version.drive.grove_version,
                        )?;
                        PathKeyUnknownElementSize((
                            KeyInfoPath::from_known_path(primary_key_path),
                            KeyInfo::MaxKeySize {
                                unique_id: document_type.unique_id_for_storage().to_vec(),
                                max_size: DEFAULT_HASH_SIZE_U8,
                            },
                            elem_size,
                        ))
                    }
                };
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertApplyType::StatefulBatchInsert
            } else {
                // Include the i64 sum_value (10-byte worst-case varint) in
                // the stateless target size when the doctype is summable —
                // mirrors the element-size adjustment above.
                let base_target = document_type.estimated_size(platform_version)? as u32;
                let target_size = if primary_key_sum_property.is_some() {
                    base_target.saturating_add(10)
                } else {
                    base_target
                };
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: primary_key_tree_type,
                    target: QueryTargetValue(target_size),
                }
            };
            let inserted = self.batch_insert_if_not_exists(
                path_key_element_info,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            )?;
            if !inserted {
                return Err(Error::Drive(DriveError::CorruptedDocumentAlreadyExists(
                    "item already exists",
                )));
            }
        }
        Ok(())
    }
}
