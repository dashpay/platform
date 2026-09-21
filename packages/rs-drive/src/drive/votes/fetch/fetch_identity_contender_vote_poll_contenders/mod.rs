mod v0;

use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderWithTally;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// The contenders of an identity contender vote poll in identity id order, each with the
    /// tally of the votes towards it, at most `limit` of them. Empty for a poll that never
    /// opened or already ended.
    pub fn fetch_identity_contender_vote_poll_contenders(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<IdentityContenderWithTally>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .fetch_identity_contender_vote_poll_contenders
        {
            0 => self.fetch_identity_contender_vote_poll_contenders_v0(
                vote_poll,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contender_vote_poll_contenders".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
