mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::IdentityContenderInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// The record of one contender of an identity contender vote poll, or None when the
    /// identity is not a contender of it: not yet, never, or no longer once the poll ended.
    pub fn fetch_identity_contender_info(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityContenderInfo>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .fetch_identity_contender_info
        {
            0 => self.fetch_identity_contender_info_v0(
                vote_poll,
                identity_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contender_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
