mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// The operations removing the end date entries of identity contender vote polls whose
    /// phase ended at the given times, and each time's tree once these were its only entries.
    pub fn remove_identity_contender_vote_poll_end_date_query_operations(
        &self,
        vote_polls: &[(&IdentityContenderVotePoll, TimestampMillis)],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .remove_identity_contender_vote_poll_end_date_query_operations
        {
            0 => self.remove_identity_contender_vote_poll_end_date_query_operations_v0(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_identity_contender_vote_poll_end_date_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
