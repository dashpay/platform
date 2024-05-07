use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::votes::Vote;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    pub(super) fn register_identity_vote_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        vote: Vote,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match vote {
            Vote::ResourceVote(contested_document_resource_vote_type) => self
                .register_contested_resource_identity_vote(
                    voter_pro_tx_hash,
                    contested_document_resource_vote_type,
                    block_info,
                    apply,
                    transaction,
                    platform_version,
                ),
        }
    }

    pub(super) fn register_identity_vote_operations_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        vote: Vote,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match vote {
            Vote::ResourceVote(contested_document_resource_vote_type) => self
                .register_contested_resource_identity_vote_operations(
                    voter_pro_tx_hash,
                    contested_document_resource_vote_type,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                ),
        }
    }
}
