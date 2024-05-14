mod v0;

use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;

impl Drive {
    /// Adds a contested document to a contract.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: Information about the document and contract.
    /// * `override_document`: Whether to override the document.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn add_contested_document_for_contract(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePoll,
        insert_without_check: bool,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert_contested
            .add_contested_document_for_contract
        {
            0 => self.add_contested_document_for_contract_v0(
                document_and_contract_info,
                contested_document_resource_vote_poll,
                insert_without_check,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contested_document_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
