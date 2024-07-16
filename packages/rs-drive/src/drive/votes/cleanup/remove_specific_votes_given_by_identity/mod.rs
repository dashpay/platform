mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// We remove the references of specific votes given by an identity when the vote poll ends
    pub fn remove_specific_vote_references_given_by_identity(
        &self,
        identity_id: &Identifier,
        votes: &[&Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .cleanup
            .remove_specific_vote_references_given_by_identity
        {
            0 => self.remove_specific_vote_references_given_by_identity_v0(
                identity_id,
                votes,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_specific_votes_given_by_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
