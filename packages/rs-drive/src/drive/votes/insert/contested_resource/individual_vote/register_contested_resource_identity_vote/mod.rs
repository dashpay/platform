mod v0;

use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use std::collections::HashMap;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::votes::contested_document_resource_vote::ContestedDocumentResourceVote;
use grovedb::{EstimatedLayerInformation, TransactionArg};

impl Drive {
    pub fn register_contested_resource_identity_vote(
        &self,
        voter_pro_tx_hash: [u8; 32],
        vote: ContestedDocumentResourceVote,
        block_info: &BlockInfo,
        apply: bool,
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
                vote,
                block_info,
                apply,
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

    pub fn register_contested_resource_identity_vote_operations(
        &self,
        voter_pro_tx_hash: [u8; 32],
        vote: ContestedDocumentResourceVote,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
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
                vote,
                block_info,
                estimated_costs_only_with_layer_info,
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
}
