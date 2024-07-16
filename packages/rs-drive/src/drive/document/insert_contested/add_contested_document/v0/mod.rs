use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::fee::fee_result::FeeResult;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use grovedb::TransactionArg;

impl Drive {
    /// Adds a contested document using bincode serialization
    #[inline(always)]
    pub(super) fn add_contested_document_v0(
        &self,
        owned_document_info: OwnedDocumentInfo,
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        insert_without_check: bool,
        also_insert_vote_poll_stored_info: Option<ContestedDocumentVotePollStoredInfo>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contested_document_resource_vote_poll
                    .contract
                    .id()
                    .into_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Document(DocumentError::DataContractNotFound))?;

        let contract = &contract_fetch_info.contract;

        let document_type = contract.document_type_for_name(
            contested_document_resource_vote_poll
                .document_type_name
                .as_str(),
        )?;

        let document_and_contract_info = DocumentAndContractInfo {
            owned_document_info,
            contract,
            document_type,
        };
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.add_contested_document_for_contract_apply_and_add_to_operations(
            document_and_contract_info,
            contested_document_resource_vote_poll,
            insert_without_check,
            block_info,
            true,
            apply,
            also_insert_vote_poll_stored_info,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok(fees)
    }
}
