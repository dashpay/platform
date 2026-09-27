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
    /// Generation 1 differs from generation 0 in two things. When the caller supplies no
    /// transaction and the operations are applied, the write and its pricing share one
    /// owned transaction that is committed only after `Drive::calculate_fee` succeeded.
    /// Generation 0 applied the batch (committing it on its own without a caller
    /// transaction) and priced it afterwards, so from protocol version 15, where pricing an
    /// owner-attributed storage removal without the fee history is an error, a call passing
    /// no history persisted the write and then failed: the second contender of a contest
    /// resolved without locking removes the creator-flagged join-window end-date entry. It
    /// now fails before anything is written. And the contract fetch is priced with the
    /// write: generation 0 recorded the fetch in one operations vector and priced another.
    /// With a caller transaction nothing is committed by Drive in either generation.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contested_document_v1(
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
        // The contract cache keys its block-versus-committed behaviour on whether a
        // transaction is present. An owned transaction is not block execution, so the
        // contract is looked up through the caller's (`caller_transaction`) and only the
        // document write goes through the owned one.
        let caller_transaction = transaction;
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(caller_transaction);

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contested_document_resource_vote_poll
                    .contract
                    .id()
                    .into_buffer(),
                Some(&block_info.epoch),
                true,
                caller_transaction,
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
        // The contract fetch above recorded its cost in `drive_operations`; generation 0
        // replaced the vector here and never priced that read.
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
        // A pricing error drops the owned transaction with everything it wrote.
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }
        Ok(fees)
    }
}
