mod v0;

use dpp::data_contract::document_type::IndexLevel;

use dpp::block::block_info::BlockInfo;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;
use std::option::Option::None;

use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, STORAGE_FLAGS_SIZE};
use crate::drive::document::{
    contract_document_type_path_vec,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_keeping_history_primary_key_path_for_unknown_document_id,
    contract_documents_keeping_history_storage_time_reference_path_size,
    contract_documents_primary_key_path, document_reference_size, make_document_reference,
    unique_event_id,
};
use crate::drive::fee::calculate_fee;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentAndSerialization, DocumentEstimatedAverageSize, DocumentOwnedInfo,
    DocumentRefAndSerialization, DocumentRefInfo,
};
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::drive::object_size_info::KeyElementInfo::{KeyElement, KeyUnknownElementSize};
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyUnknownElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::{PathFixedSizeKeyRef, PathKeySize};
use crate::drive::object_size_info::{
    DocumentAndContractInfo, OwnedDocumentInfo, PathInfo, PathKeyElementInfo,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::data_contract::DataContract;

use crate::drive::grove_operations::DirectQueryType::{StatefulDirectQuery, StatelessDirectQuery};

use dpp::document::Document;
use dpp::prelude::Identifier;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;

impl Drive {
    /// Adds a document to primary storage.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: Information about the document and contract.
    /// * `block_info`: The block info.
    /// * `insert_without_check`: Whether to insert the document without check.
    /// * `estimated_costs_only_with_layer_info`: Information about the estimated costs only with layer.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(crate) fn add_document_to_primary_storage(
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
        match platform_version
            .drive
            .methods
            .document
            .insert
            .add_document_to_primary_storage
        {
            0 => self.add_document_to_primary_storage_0(
                document_and_contract_info,
                block_info,
                insert_without_check,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_document_to_primary_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
