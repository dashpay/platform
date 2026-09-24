mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Removes finished polls of every kind from the end date index, and each end date's tree
    /// once nothing is left under it. `vote_polls` holds every poll closed in this block with
    /// its end date, so the count at one end date covers both kinds: a partial count would
    /// delete a tree that still holds a poll of the other kind.
    pub fn remove_vote_poll_end_date_query_operations(
        &self,
        vote_polls: &[(Identifier, TimestampMillis)],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .cleanup
            .remove_vote_poll_end_date_query_operations
        {
            0 => self.remove_vote_poll_end_date_query_operations_v0(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_vote_poll_end_date_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
