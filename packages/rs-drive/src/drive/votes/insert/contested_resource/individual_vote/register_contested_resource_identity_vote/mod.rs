mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// Registers a vote for a contested resource based on the voter's identifier,
    /// vote poll, and the specific vote choice.
    ///
    /// # Parameters
    ///
    /// - `voter_pro_tx_hash`: A 32-byte array representing the ProRegTx hash of the voter.
    /// - `vote_poll`: The specific contested document resource vote poll context.
    /// - `vote_choice`: The choice made by the voter on the contested resource.
    /// - `block_info`: Reference to the block information at the time of the vote.
    /// - `apply`: Boolean flag indicating whether to apply the vote to the database immediately.
    /// - `transaction`: Transaction arguments providing context for this operation.
    /// - `platform_version`: Reference to the platform version against which the operation is executed.
    ///
    /// # Returns
    ///
    /// Returns a `Result` that, on success, includes the `FeeResult` detailing any fees applied as a result of the vote.
    /// On failure, it returns an `Error`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The platform version is unknown or unsupported.
    /// - There is an issue processing the transaction or applying it to the database.
    ///

    pub fn register_contested_resource_identity_vote(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        vote_choice: ResourceVoteChoice,
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
            .register_contested_resource_identity_vote
        {
            0 => self.register_contested_resource_identity_vote_v0(
                voter_pro_tx_hash,
                strength,
                vote_poll,
                vote_choice,
                previous_resource_vote_choice_to_remove,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_contested_resource_identity_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gathers and returns low-level drive operations needed to register a vote for a contested resource,
    /// considering the voter's identifier, vote poll, and vote choice, optionally estimating costs.
    ///
    /// # Parameters
    ///
    /// - `voter_pro_tx_hash`: A 32-byte array representing the ProRegTx hash of the voter.
    /// - `vote_poll`: The specific contested document resource vote poll context.
    /// - `vote_choice`: The choice made by the voter on the contested resource.
    /// - `block_info`: Reference to the block information at the time of the vote.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to an optional HashMap that, if provided,
    ///   will be filled with estimated costs and layer information necessary for processing the vote.
    /// - `transaction`: Transaction arguments providing context for this operation.
    /// - `platform_version`: Reference to the platform version against which the operation is executed.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `LowLevelDriveOperation` detailing the necessary operations
    /// to execute the vote registration, or an `Error` if the operation cannot be completed.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The platform version is unknown or unsupported.
    /// - Any low-level drive operation fails due to transaction or database inconsistencies.
    ///
    pub fn register_contested_resource_identity_vote_operations(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        vote_choice: ResourceVoteChoice,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .register_contested_resource_identity_vote
        {
            0 => self.register_contested_resource_identity_vote_operations_v0(
                voter_pro_tx_hash,
                strength,
                vote_poll,
                vote_choice,
                previous_resource_vote_choice_to_remove,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_contested_resource_identity_vote_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
