use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use dpp::data_contract::document_type::{DocumentTypeRef, IndexLevel};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
    CONTRACT_DOCUMENTS_PATH_HEIGHT, DEFAULT_HASH_SIZE_U8,
};
use crate::drive::document::{
    contract_document_type_path_vec, contract_documents_primary_key_path, document_reference_size,
    unique_event_id,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentEstimatedAverageSize, DocumentOwnedInfo,
};
use crate::drive::object_size_info::DriveKeyInfo::KeyRef;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::document::Document;

use crate::drive::grove_operations::BatchDeleteApplyType::{
    StatefulBatchDelete, StatelessBatchDelete,
};
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, PathInfo};
use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;

impl Drive {
    /// Prepares the operations for deleting a document.
    pub(super) fn delete_document_for_contract_operations_v0(
        &self,
        document_id: [u8; 32],
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        if !document_type.documents_read_only() {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "this document type is not mutable and can not be deleted",
            )));
        }

        if document_type.document_revisions() {
            return Err(Error::Drive(
                DriveError::InvalidDeletionOfDocumentThatKeepsHistory(
                    "this document type keeps history and therefore can not be deleted",
                ),
            ));
        }

        // first we need to construct the path for documents on the contract
        // the path is
        //  * Document andDataContract root tree
        //  *DataContract ID recovered from document
        //  * 0 to signify Documents and notDataContract
        let contract_documents_primary_key_path = contract_documents_primary_key_path(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );

        let direct_query_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                contract,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(
                    document_type.estimated_size(platform_version)? as u32
                ),
            }
        } else {
            DirectQueryType::StatefulDirectQuery
        };

        // next we need to get the document from storage
        let document_element: Option<Element> = self.grove_get_raw(
            (&contract_documents_primary_key_path).into(),
            document_id.as_slice(),
            direct_query_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        let document_info = if let DirectQueryType::StatelessDirectQuery { query_target, .. } =
            direct_query_type
        {
            DocumentEstimatedAverageSize(query_target.len())
        } else if let Some(document_element) = &document_element {
            if let Element::Item(data, element_flags) = document_element {
                let document =
                    Document::from_bytes(data.as_slice(), document_type, platform_version)?;
                let storage_flags = StorageFlags::map_cow_some_element_flags_ref(element_flags)?;
                DocumentOwnedInfo((document, storage_flags))
            } else {
                return Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                    "document being deleted is not an item",
                )));
            }
        } else {
            return Err(Error::Drive(DriveError::DeletingDocumentThatDoesNotExist(
                "document being deleted does not exist",
            )));
        };

        // third we need to delete the document for it's primary key
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
        Ok(batch_operations)
    }
}
