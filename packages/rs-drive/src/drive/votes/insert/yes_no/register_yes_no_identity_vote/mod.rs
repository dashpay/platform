mod v0;

use crate::drive::votes::resolved::votes::resolved_yes_no_vote::ResolvedYesNoVote;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Registers a masternode's vote on a yes/no poll: its strength under the chosen answer's
    /// sum tree, the previous answer removed if the vote changed, and the identity votes index
    /// updated.
    pub fn register_yes_no_identity_vote(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedYesNoVote,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .register_yes_no_identity_vote
        {
            0 => self.register_yes_no_identity_vote_v0(
                voter_pro_tx_hash,
                strength,
                vote,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_yes_no_identity_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations of [`Self::register_yes_no_identity_vote`].
    pub fn register_yes_no_identity_vote_operations(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedYesNoVote,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .register_yes_no_identity_vote
        {
            0 => self.register_yes_no_identity_vote_operations_v0(
                voter_pro_tx_hash,
                strength,
                vote,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_yes_no_identity_vote_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
