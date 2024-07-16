mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use crate::drive::votes::resolved::votes::ResolvedVote;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// Registers a vote associated with a specific identity using the given voter's ProRegTx hash.
    /// This function applies the vote to the blockchain state if specified, within the context of the given block.
    ///
    /// # Parameters
    ///
    /// - `voter_pro_tx_hash`: A 32-byte array representing the ProRegTx hash of the voter.
    /// - `strength`: the strength of the vote, masternodes have 1, evonodes have 4
    /// - `vote`: The vote to be registered, encapsulating the decision made by the voter.
    /// - `block_info`: Reference to the block information at the time of the vote.
    /// - `apply`: A boolean flag indicating whether the vote should be immediately applied to the state.
    /// - `transaction`: Contextual transaction arguments that may affect the processing of the vote.
    /// - `platform_version`: Reference to the platform version to ensure compatibility of the vote registration method.
    ///
    /// # Returns
    ///
    /// Returns a `Result<FeeResult, Error>`, where `FeeResult` includes any fees applied as a result of registering the vote,
    /// and `Error` captures any issues encountered during the process.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The platform version is unknown or unsupported, resulting in a version mismatch error.
    /// - There is a failure in processing the vote due to transaction or blockchain state issues.
    ///
    pub fn register_identity_vote(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedVote,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .register_identity_vote
        {
            0 => self.register_identity_vote_v0(
                voter_pro_tx_hash,
                strength,
                vote,
                previous_resource_vote_choice_to_remove,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_identity_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Prepares and returns a list of low-level drive operations necessary for registering a vote,
    /// based on the voter's ProRegTx hash and current block information. This method can also estimate costs
    /// if required, which helps in preparing for the actual application of the vote.
    ///
    /// # Parameters
    ///
    /// - `voter_pro_tx_hash`: A 32-byte array representing the ProRegTx hash of the voter.
    /// - `vote`: The vote to be registered, detailing the decision made.
    /// - `block_info`: Reference to current block information.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to an optional HashMap that, if provided,
    ///   will be filled with estimated cost and layer information required for processing the vote.
    /// - `transaction`: Contextual transaction arguments that may influence the generation of operations.
    /// - `platform_version`: Reference to the platform version to ensure compatibility of the operation generation method.
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<LowLevelDriveOperation>, Error>`, where `Vec<LowLevelDriveOperation>` contains the detailed operations needed,
    /// and `Error` captures any issues encountered during the operation preparation.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The platform version is unknown or unsupported, resulting in a version mismatch error.
    /// - There are issues generating the necessary operations due to transaction inconsistencies or blockchain state errors.
    ///
    pub fn register_identity_vote_operations(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedVote,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .register_identity_vote
        {
            0 => self.register_identity_vote_operations_v0(
                voter_pro_tx_hash,
                strength,
                vote,
                previous_resource_vote_choice_to_remove,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_identity_vote_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
