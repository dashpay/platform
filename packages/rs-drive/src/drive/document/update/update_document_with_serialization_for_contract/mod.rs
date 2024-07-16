mod v0;

use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use grovedb::TransactionArg;
use std::borrow::Cow;

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
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .update
            .update_document_with_serialization_for_contract
        {
            0 => self.update_document_with_serialization_for_contract_v0(
                document,
                serialized_document,
                contract,
                document_type_name,
                owner_id,
                block_info,
                apply,
                storage_flags,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_document_with_serialization_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
