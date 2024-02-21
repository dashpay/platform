mod v0;

use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::ContestedDocumentResourceVoteType;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    pub fn register_contested_resource_identity_vote(
        &self,
        voter_pro_tx_hash: Identifier,
        vote: ContestedDocumentResourceVoteType,
        block_info: &BlockInfo,
        identity_nonce: IdentityNonce,
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
                identity_nonce,
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
        vote: ContestedDocumentResourceVoteType,
        block_info: &BlockInfo,
        identity_nonce: IdentityNonce,
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
                identity_nonce,
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
