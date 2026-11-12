mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use platform_version::version::ProtocolVersion;

impl Drive {
    /// The protocol version a contest that is under way was opened at, when it was opened in
    /// an earlier epoch than the block's.
    ///
    /// A contest's poll is named by its index values as the protocol version it was opened at
    /// writes them, and its prefunded balance and end date are kept under that poll. A
    /// contender joining it under a later protocol version names the same poll by writing its
    /// values the same way, so it reads the version here.
    ///
    /// # Parameters
    /// - `stored_info`: The stored info of the contest.
    /// - `epoch`: The epoch of the block, which the read is billed in.
    /// - `platform_version`: The platform version.
    ///
    /// # Returns
    /// The fee of the read and the protocol version of the epoch the contest started in.
    /// `None`, and no fee, for a contest that is not under way, for one that started in the
    /// block's own epoch, which ran the block's protocol version, and before protocol version
    /// 15, which reads nothing.
    pub fn fetch_contest_opening_protocol_version(
        &self,
        stored_info: &ContestedDocumentVotePollStoredInfo,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<ProtocolVersion>), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .fetch
            .fetch_contest_opening_protocol_version
        {
            None => Ok((FeeResult::default(), None)),
            Some(0) => {
                self.fetch_contest_opening_protocol_version_v0(stored_info, epoch, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contest_opening_protocol_version".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
