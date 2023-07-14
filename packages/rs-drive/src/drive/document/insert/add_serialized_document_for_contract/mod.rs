mod v0;

use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::result::FeeResult;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;
use serde::Deserialize;
use std::borrow::Cow;

impl Drive {
    /// Deserializes a document and adds it to a contract.
    ///
    /// # Parameters
    /// * `serialized_document`: The serialized document.
    /// * `contract`: The contract.
    /// * `document_type_name`: The document type name.
    /// * `owner_id`: The owner ID.
    /// * `override_document`: Whether to override the document.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `storage_flags`: The storage flags.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn add_serialized_document_for_contract(
        &self,
        serialized_document: &[u8],
        contract: &DataContract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        match drive_version
            .methods
            .document
            .insert
            .add_serialized_document_for_contract
        {
            0 => self.add_serialized_document_for_contract_v0(
                serialized_document,
                contract,
                document_type_name,
                owner_id,
                override_document,
                block_info,
                apply,
                storage_flags,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_serialized_document_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
