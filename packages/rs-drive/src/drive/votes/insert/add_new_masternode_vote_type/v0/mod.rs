use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::drive::object_size_info::OwnedDocumentInfo;
use crate::error::Error;

impl Drive {
    pub(super) fn add_new_masternode_vote_type_v0(
        &self,
        owned_document_info: OwnedDocumentInfo,
        data_contract_id: Identifier,
        document_type_name: &str,
        override_document: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        
    }
}
