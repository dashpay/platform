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
    ///
    /// # Parameters
    /// - `voter_pro_tx_hash`: the voting masternode's pro tx hash.
    /// - `strength`: its voting power: 1 for a masternode, 4 for an evonode.
    /// - `vote`: the vote, with the previous answer to remove when it changed.
    /// - `block_info`: the block that registers it.
    /// - `transaction`: the transaction to read and write in.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The fee of the operations applied.
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
            Some(0) => self.register_yes_no_identity_vote_v0(
                voter_pro_tx_hash,
                strength,
                vote,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_yes_no_identity_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "register_yes_no_identity_vote".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// The operations of [`Self::register_yes_no_identity_vote`].
    ///
    /// # Parameters
    /// - `voter_pro_tx_hash`: the voting masternode's pro tx hash.
    /// - `strength`: its voting power: 1 for a masternode, 4 for an evonode.
    /// - `vote`: the vote, with the previous answer to remove when it changed.
    /// - `transaction`: the transaction to read in.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The operations that register the vote.
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
            Some(0) => self.register_yes_no_identity_vote_operations_v0(
                voter_pro_tx_hash,
                strength,
                vote,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_yes_no_identity_vote_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "register_yes_no_identity_vote_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
