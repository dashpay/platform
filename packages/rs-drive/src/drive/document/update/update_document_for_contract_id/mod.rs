mod v0;

use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;

use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use grovedb::TransactionArg;
use std::borrow::Cow;

impl Drive {
    /// Updates a serialized document given a contract id and returns the associated fee.
    ///
    /// # Parameters
    /// * `serialized_document`: The serialized document to be updated.
    /// * `contract_id`: The id of the contract.
    /// * `document_type`: The type of the document.
    /// * `owner_id`: The id of the owner.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `storage_flags`: An optional storage flags.
    /// * `transaction`: The transaction argument.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn update_document_for_contract_id(
        &self,
        serialized_document: &[u8],
        contract_id: [u8; 32],
        document_type: &str,
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
            .update_document_for_contract_id
        {
            0 => self.update_document_for_contract_id_v0(
                serialized_document,
                contract_id,
                document_type,
                owner_id,
                block_info,
                apply,
                storage_flags,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_document_for_contract_id".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
