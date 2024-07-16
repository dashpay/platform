use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;

use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

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
use crate::util::object_size_info::DocumentAndContractInfo;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyUnknownElementSize,
};

use crate::util::grove_operations::BatchInsertApplyType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::DocumentV0Getters;

use crate::drive::votes::paths::vote_contested_resource_contract_documents_storage_path;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::version::PlatformVersion;

impl Drive {
    /// Adds a document to primary storage.
    /// If a document isn't sent to this function then we are just calling to know the query and
    /// insert operations
    pub(super) fn add_contested_document_to_primary_storage_0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
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
        let primary_key_path = vote_contested_resource_contract_documents_storage_path(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );
        // if we are trying to get estimated costs we should add this level
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_add_contested_document_to_primary_storage(
                document_and_contract_info,
                primary_key_path,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        if insert_without_check {
            let path_key_element_info = match &document_and_contract_info
                .owned_document_info
                .document_info
            {
                DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                    let element = Element::Item(
                        serialized_document.to_vec(),
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
                DocumentAndSerialization((document, serialized_document, storage_flags)) => {
                    let element = Element::Item(
                        serialized_document.to_vec(),
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
                DocumentRefInfo((document, storage_flags)) => {
                    let serialized_document = document
                        .serialize(document_and_contract_info.document_type, platform_version)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
                DocumentEstimatedAverageSize(average_size) => PathKeyUnknownElementSize((
                    KeyInfoPath::from_known_path(primary_key_path),
                    KeyInfo::MaxKeySize {
                        unique_id: document_type.unique_id_for_storage().to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    },
                    Element::required_item_space(
                        *average_size,
                        STORAGE_FLAGS_SIZE,
                        &platform_version.drive.grove_version,
                    )?,
                )),
                DocumentOwnedInfo((document, storage_flags)) => {
                    let serialized_document = document
                        .serialize(document_and_contract_info.document_type, platform_version)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
            };
            self.batch_insert(path_key_element_info, drive_operations, drive_version)?;
        } else {
            let path_key_element_info = match &document_and_contract_info
                .owned_document_info
                .document_info
            {
                DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                    let element = Element::Item(
                        serialized_document.to_vec(),
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
                DocumentAndSerialization((document, serialized_document, storage_flags)) => {
                    let element = Element::Item(
                        serialized_document.to_vec(),
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
                DocumentOwnedInfo((document, storage_flags)) => {
                    let serialized_document = document
                        .serialize(document_and_contract_info.document_type, platform_version)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
                DocumentRefInfo((document, storage_flags)) => {
                    let serialized_document = document
                        .serialize(document_and_contract_info.document_type, platform_version)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
                    );
                    PathFixedSizeKeyRefElement((
                        primary_key_path,
                        document.id_ref().as_slice(),
                        element,
                    ))
                }
                DocumentEstimatedAverageSize(max_size) => PathKeyUnknownElementSize((
                    KeyInfoPath::from_known_path(primary_key_path),
                    KeyInfo::MaxKeySize {
                        unique_id: document_type.unique_id_for_storage().to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    },
                    Element::required_item_space(
                        *max_size,
                        STORAGE_FLAGS_SIZE,
                        &platform_version.drive.grove_version,
                    )?,
                )),
            };
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertApplyType::StatefulBatchInsert
            } else {
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_using_sums: false,
                    target: QueryTargetValue(document_type.estimated_size(platform_version)? as u32),
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
                    "item already exists in insert contested",
                )));
            }
        }
        Ok(())
    }
}
