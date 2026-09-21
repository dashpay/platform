mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The operations removing a masternode's references to its votes on the given identity
    /// contender vote polls, once the polls ended and the votes themselves are gone.
    pub fn remove_identity_contender_vote_references_given_by_identity_operations(
        &self,
        identity_id: &Identifier,
        vote_poll_ids: &[&Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .remove_identity_contender_vote_references_given_by_identity_operations
        {
            0 => self.remove_identity_contender_vote_references_given_by_identity_operations_v0(
                identity_id,
                vote_poll_ids,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_identity_contender_vote_references_given_by_identity_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
