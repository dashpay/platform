mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a contested resource contest start info.
    pub fn fetch_contested_document_vote_poll_stored_info(
        &self,
        contested_document_resource_vote_poll_with_contract_info: &ContestedDocumentResourceVotePollWithContractInfo,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            Option<FeeResult>,
            Option<ContestedDocumentVotePollStoredInfo>,
        ),
        Error,
    > {
        match platform_version
            .drive
            .methods
            .vote
            .fetch
            .fetch_contested_document_vote_poll_stored_info
        {
            0 => self.fetch_contested_document_vote_poll_stored_info_v0(
                contested_document_resource_vote_poll_with_contract_info,
                epoch,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contested_document_vote_poll_stored_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
