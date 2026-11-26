mod v0;

use crate::drive::votes::PreviousVoteCount;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// A masternode's current answer to a yes/no poll and how many times it voted on it, or
    /// `None` when it never did.
    ///
    /// # Parameters
    /// - `masternode_pro_tx_hash`: the masternode.
    /// - `vote_poll_id`: the poll's unique id.
    /// - `transaction`: the transaction to read in.
    /// - `drive_operations`: where the read's cost is added.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The answer and how many times the masternode voted on the poll, or `None`.
    pub fn fetch_identity_yes_no_vote(
        &self,
        masternode_pro_tx_hash: Identifier,
        vote_poll_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(YesNoAbstainVoteChoice, PreviousVoteCount)>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .fetch_identity_yes_no_vote
        {
            Some(0) => self.fetch_identity_yes_no_vote_v0(
                masternode_pro_tx_hash,
                vote_poll_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_yes_no_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_identity_yes_no_vote".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
