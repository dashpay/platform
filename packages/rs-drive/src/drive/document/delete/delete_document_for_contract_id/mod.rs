mod v0;

use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use dpp::data_contract::document_type::{DocumentType, IndexLevel};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use dpp::data_contract::DataContract;
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
use dpp::block::extended_block_info::BlockInfo;
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
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;

use crate::fee::result::FeeResult;
use dpp::block::epoch::Epoch;
use dpp::version::drive_versions::DriveVersion;


impl Drive {
    /// Deletes a document and returns the associated fee.
    /// The contract CBOR is given instead of the contract itself.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract_id`: The ID of the contract that contains the document.
    /// * `document_type_name`: The name of the document type.
    /// * `owner_id`: The owner ID of the document.
    /// * `block_info`: The block information.
    /// * `apply`: Boolean flag indicating if the operation should be applied.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn delete_document_for_contract_id(
        &self,
        document_id: [u8; 32],
        contract_id: [u8; 32],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        match drive_version.methods.document.delete.delete_document_for_contract_id {
            0 => self.delete_document_for_contract_id_v0(
                document_id,
                contract_id,
                document_type_name,
                owner_id,
                block_info,
                apply,
                transaction,
                drive_version
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_id".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}