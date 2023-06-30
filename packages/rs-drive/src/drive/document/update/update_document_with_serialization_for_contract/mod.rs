mod v0;

use std::borrow::Cow;
use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::version::drive_versions::DriveVersion;
use crate::contract::Contract;
use crate::drive::Drive;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use crate::error::Error;
use crate::error::drive::DriveError;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::result::FeeResult;

impl Drive {
    /// Updates a document and returns the associated fee.
    ///
    /// # Parameters
    /// * `document`: The document to be updated.
    /// * `serialized_document`: The serialized document.
    /// * `contract`: The contract.
    /// * `document_type_name`: The document type name.
    /// * `owner_id`: The owner's ID.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `storage_flags`: The storage flags.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn update_document_with_serialization_for_contract(
        &self,
        document: &Document,
        serialized_document: &[u8],
        contract: &DataContract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        match drive_version.methods.document.update.update_document_with_serialization_for_contract {
            0 => {
                self.update_document_with_serialization_for_contract_v0(
                    document,
                    serialized_document,
                    contract,
                    document_type_name,
                    owner_id,
                    block_info,
                    apply,
                    storage_flags,
                    transaction,
                    drive_version,
                )
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_document_with_serialization_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}