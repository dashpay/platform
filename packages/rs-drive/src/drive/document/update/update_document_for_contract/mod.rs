mod v0;

use std::borrow::Cow;
use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::Document;
use dpp::data_contract::DataContract;
use crate::drive::Drive;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::result::FeeResult;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Updates a document and returns the associated fee.
    ///
    /// # Parameters
    /// * `document`: The document to be updated.
    /// * `contract`: The contract for the document.
    /// * `document_type`: The document type.
    /// * `owner_id`: The optional owner ID.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `storage_flags`: Optional storage flags.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn update_document_for_contract(
        &self,
        document: &Document,
        contract: &DataContract,
        document_type: &DocumentType,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        match drive_version.methods.document.update.update_document_for_contract {
            0 => {
                self.update_document_for_contract_v0(
                    document,
                    contract,
                    document_type,
                    owner_id,
                    block_info,
                    apply,
                    storage_flags,
                    transaction,
                )
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_document_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}