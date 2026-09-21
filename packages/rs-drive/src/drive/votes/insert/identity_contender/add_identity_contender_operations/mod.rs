mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::IdentityContenderInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds an identity as a contender of an identity contender vote poll and applies the
    /// operations. Whether the poll is still in its join phase is the caller's check: this
    /// only refuses an identity that is a contender already.
    pub fn add_identity_contender(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        identity_id: Identifier,
        contender_info: IdentityContenderInfo,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .add_identity_contender_operations
        {
            0 => self.add_identity_contender_v0(
                vote_poll,
                identity_id,
                contender_info,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_identity_contender".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations adding an identity as a contender of an identity contender vote poll:
    /// its tree, its record and its votes tree.
    #[allow(clippy::too_many_arguments)]
    pub fn add_identity_contender_operations(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        identity_id: Identifier,
        contender_info: IdentityContenderInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .add_identity_contender_operations
        {
            0 => self.add_identity_contender_operations_v0(
                vote_poll,
                identity_id,
                contender_info,
                estimated_costs_only_with_layer_info,
                previous_batch_operations,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_identity_contender_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
