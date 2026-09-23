mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Removes a masternode's entries for these yes/no polls from the identity votes index of
    /// the decisions branch.
    pub fn remove_yes_no_vote_references_given_by_identity_operations(
        &self,
        masternode_pro_tx_hash: &Identifier,
        vote_poll_ids: &[&Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .remove_yes_no_vote_references_given_by_identity
        {
            0 => self.remove_yes_no_vote_references_given_by_identity_operations_v0(
                masternode_pro_tx_hash,
                vote_poll_ids,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_yes_no_vote_references_given_by_identity_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
