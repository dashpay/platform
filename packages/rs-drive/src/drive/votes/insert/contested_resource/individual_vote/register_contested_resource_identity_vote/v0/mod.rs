use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::{ContestedDocumentResourceVoteType, Vote};
use grovedb::TransactionArg;
use dpp::prelude::Identifier;
use platform_version::version::PlatformVersion;

impl Drive {
    pub fn register_contested_resource_identity_vote_v0(
        &self,
        voter_pro_tx_hash: Identifier,
        vote: ContestedDocumentResourceVoteType,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        // let's start by creating a batch of operations
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                vote.contract_id.to_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Document(DocumentError::DataContractNotFound))?;
    }
}
