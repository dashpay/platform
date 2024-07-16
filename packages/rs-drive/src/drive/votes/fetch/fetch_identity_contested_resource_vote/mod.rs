mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a specific identity vote.
    pub fn fetch_identity_contested_resource_vote(
        &self,
        masternode_pro_tx_hash: Identifier,
        vote_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(ResourceVoteChoice, PreviousVoteCount)>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .fetch
            .fetch_identity_contested_resource_vote
        {
            0 => self.fetch_identity_contested_resource_vote_v0(
                masternode_pro_tx_hash,
                vote_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contested_resource_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
