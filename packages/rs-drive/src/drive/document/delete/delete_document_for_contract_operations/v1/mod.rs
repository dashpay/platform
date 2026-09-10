use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};

use dpp::data_contract::document_type::DocumentTypeRef;

use std::collections::HashMap;

use crate::drive::constants::DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE;
use crate::drive::document::lifecycle::{DocumentLifecycleRecord, DOCUMENT_LIFECYCLE_RECORD_SIZE};
use crate::drive::document::paths::{
    contract_documents_primary_key_path, document_lifecycle_path, DOCUMENT_LIFECYCLE_TREE_KEY,
};
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::util::object_size_info::DocumentInfo::{
    DocumentEstimatedAverageSize, DocumentOwnedInfo,
};
use crate::util::storage_flags::StorageFlags;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::document::Document;

use crate::drive::Drive;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchInsertTreeApplyType, QueryType};
use crate::util::object_size_info::PathKeyElementInfo::{
    PathKeyElement, PathKeyUnknownElementSize,
};
use crate::util::object_size_info::PathKeyInfo::PathKey;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};

use crate::error::drive::DriveError;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::identifier::Identifier;

use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;

impl Drive {
    /// Prepares the operations for deleting a document.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_document_for_contract_operations_v1(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        deleter_id: Option<Identifier>,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if !document_type.documents_can_be_deleted() {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "this document type is not mutable and can not be deleted",
            )));
        }

        self.force_delete_document_for_contract_operations_v1(
            document_id,
            contract,
            document_type,
            block_info,
            deleter_id,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )
    }

    /// Prepares the operations for deleting a document.
    ///
    /// Deleting a keep-history document removes it from every ordinary read
    /// without touching a single retained revision: the current pointer and the
    /// index references that lead to it go, and a lifecycle record recording the
    /// deletion time takes their place. The revisions stay where they are, and
    /// only an erase can remove them.
    ///
    /// Every other document type is deleted exactly as v0 deletes it.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn force_delete_document_for_contract_operations_v1(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        deleter_id: Option<Identifier>,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if !document_type.documents_keep_history() {
            return self.force_delete_document_for_contract_operations_v0(
                document_id,
                contract,
                document_type,
                block_info,
                deleter_id,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            );
        }

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_documents_primary_key_path = contract_documents_primary_key_path(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );

        let query_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                contract,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            // The read follows the current pointer into the type's history, so
            // the dry run has to pay for the reference hop as well as the
            // revision it lands on. `DirectQueryType` cannot express that: its
            // conversion hardcodes an empty reference-size list.
            QueryType::StatelessQuery {
                in_tree_type: document_type.primary_key_tree_type(platform_version)?,
                query_target: QueryTargetValue(
                    document_type.estimated_size(platform_version)? as u32
                ),
                estimated_reference_sizes: vec![DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE],
            }
        } else {
            QueryType::StatefulQuery
        };

        // Resolves the pointer, so this is the current revision's item, whose
        // flags are the ones the index references were written with.
        let document_element: Option<Element> = self.grove_get(
            (&contract_documents_primary_key_path).into(),
            document_id.as_slice(),
            query_type.clone(),
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        let document_info = if let QueryType::StatelessQuery { query_target, .. } = query_type {
            DocumentEstimatedAverageSize(query_target.len())
        } else if let Some(document_element) = &document_element {
            let Element::Item(data, element_flags) = document_element else {
                return Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                    "the current pointer of a keep-history document did not resolve to an item",
                )));
            };
            let document = Document::from_bytes(data.as_slice(), document_type, platform_version)?;
            let storage_flags = StorageFlags::map_cow_some_element_flags_ref(element_flags)?;
            DocumentOwnedInfo((document, storage_flags))
        } else {
            return Err(Error::Drive(DriveError::DeletingDocumentThatDoesNotExist(
                "document being deleted does not exist",
            )));
        };

        // The pointer goes; the revisions it named stay in the history tree.
        self.remove_document_from_primary_storage(
            document_id,
            document_type,
            contract_documents_primary_key_path,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        let document_and_contract_info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info,
                owner_id: None,
            },
            contract,
            document_type,
        };

        self.remove_indices_for_top_index_level_for_contract_operations(
            &document_and_contract_info,
            &previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        // The record is the deleter's byte, refunded to them when the terminal
        // erase chunk removes it. A caller with no signer writes it unflagged,
        // and it then refunds nobody, exactly as the unflagged structural bytes
        // of a non-deletable contract do.
        let record_flags = deleter_id
            .map(|id| StorageFlags::new_single_epoch(block_info.epoch.index, Some(id.to_buffer())));
        self.add_lifecycle_record_operations(
            document_id,
            contract,
            document_type,
            DocumentLifecycleRecord::deleted_at(block_info.time_ms),
            record_flags.as_ref(),
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        Ok(batch_operations)
    }

    /// Creates the document type's lifecycle tree if it does not exist yet and
    /// writes one record into it.
    ///
    /// The tree is created on demand so that a document type whose documents are
    /// never deleted never pays for one, and the record is flagged to the
    /// deleter, who is refunded when the terminal erase chunk removes it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_lifecycle_record_operations(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        record: DocumentLifecycleRecord,
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let lifecycle_path =
            document_lifecycle_path(contract.id_ref().as_bytes(), document_type.name().as_str());
        let element_flags = StorageFlags::map_to_some_element_flags(storage_flags);
        let flags_len = storage_flags
            .map(StorageFlags::serialized_size)
            .unwrap_or_default();

        if let Some(layers) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_lifecycle_record(
                contract,
                document_type,
                layers,
                platform_version,
            )?;
        }

        let tree_apply_type = if estimated_costs_only_with_layer_info.is_some() {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len,
            }
        } else {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        };

        let mut document_type_path = crate::drive::document::paths::contract_document_type_path_vec(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );
        let lifecycle_tree_key = vec![DOCUMENT_LIFECYCLE_TREE_KEY];
        self.batch_insert_empty_tree_if_not_exists::<0>(
            PathKey((std::mem::take(&mut document_type_path), lifecycle_tree_key)),
            TreeType::NormalTree,
            storage_flags,
            tree_apply_type,
            transaction,
            &mut None,
            batch_operations,
            &platform_version.drive,
        )?;

        if estimated_costs_only_with_layer_info.is_some() {
            self.batch_insert::<0>(
                PathKeyUnknownElementSize((
                    KeyInfoPath::from_known_owned_path(lifecycle_path),
                    KeyInfo::KnownKey(document_id.to_vec()),
                    Element::required_item_space(
                        DOCUMENT_LIFECYCLE_RECORD_SIZE,
                        flags_len,
                        &platform_version.drive.grove_version,
                    )?,
                )),
                batch_operations,
                &platform_version.drive,
            )
        } else {
            self.batch_insert::<0>(
                PathKeyElement((
                    lifecycle_path,
                    document_id.to_vec(),
                    Element::Item(record.serialize(), element_flags),
                )),
                batch_operations,
                &platform_version.drive,
            )
        }
    }
}
