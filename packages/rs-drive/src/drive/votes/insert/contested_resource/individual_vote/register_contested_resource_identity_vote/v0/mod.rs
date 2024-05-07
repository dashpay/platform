use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::votes::contested_document_resource_vote::accessors::v0::ContestedDocumentResourceVoteGettersV0;
use dpp::voting::votes::contested_document_resource_vote::ContestedDocumentResourceVote;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    pub fn register_contested_resource_identity_vote_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        vote: ContestedDocumentResourceVote,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let vote_poll = vote.vote_poll();
        // let's start by creating a batch of operations
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                vote.vote_poll().contract_id.to_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Document(DocumentError::DataContractNotFound))?;
    }

    pub fn register_contested_resource_identity_vote_operations_v0(
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
        // let's start by creating a batch of operations
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // The vote at this point will have been verified as valid by rs-drive-abci
    }
}
