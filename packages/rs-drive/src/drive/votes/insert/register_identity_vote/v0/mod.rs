use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::Vote;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use platform_version::version::PlatformVersion;

impl Drive {
    pub fn register_identity_vote_v0(
        &self,
        vote: Vote,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match vote {
            Vote::ContestedDocumentResourceVote(contested_document_resource_vote_type) => self
                .register_contested_resource_identity_vote(
                    contested_document_resource_vote_type,
                    block_info,
                    apply,
                    transaction,
                    platform_version,
                ),
        }
    }
}
