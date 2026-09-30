mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches how many contenders a contested document resource vote poll holds, counting at
    /// most `count_limit` of them.
    ///
    /// A poll started from protocol version 14 holds its choices in a count tree, and its
    /// contenders are counted from that tree's element in one read. A poll started before
    /// holds them in a plain tree, and its contenders are counted by a query of at most
    /// `count_limit` of their keys.
    ///
    /// # Parameters
    /// - `vote_poll`: The contested document resource vote poll.
    /// - `count_limit`: The most contenders counted; a poll holding more is reported as
    ///   holding `count_limit`.
    /// - `epoch`: The epoch the reads are billed in.
    /// - `transaction`: The transaction to read in.
    /// - `platform_version`: The platform version.
    ///
    /// # Returns
    /// The fee of the reads and the number of contenders, at most `count_limit`; 0 for a poll
    /// that has none, or does not exist.
    pub fn fetch_contested_document_vote_poll_contender_count(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        count_limit: u16,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, u16), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .fetch
            .fetch_contested_document_vote_poll_contender_count
        {
            0 => self.fetch_contested_document_vote_poll_contender_count_v0(
                vote_poll,
                count_limit,
                epoch,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contested_document_vote_poll_contender_count".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
