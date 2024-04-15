use crate::drive::Drive;
use crate::error::Error;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::Vote;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use platform_version::version::PlatformVersion;

impl Drive {
    pub fn register_identity_vote_for_identity_queries_v0(
        &self,
        voter_pro_tx_hash: Identifier,
        vote: Vote,
        block_info: &BlockInfo,
        identity_nonce: IdentityNonce,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match vote {
            Vote::ContestedDocumentResourceVote(contested_document_resource_vote_type) => self
                .register_contested_resource_identity_vote(
                    voter_pro_tx_hash,
                    contested_document_resource_vote_type,
                    block_info,
                    identity_nonce,
                    apply,
                    transaction,
                    platform_version,
                ),
        }
    }
}
