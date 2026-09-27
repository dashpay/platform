use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Generation 1 differs from generation 0 in one thing: when the caller supplies no
    /// transaction and the operations are applied, the write and its pricing share one
    /// owned transaction that is committed only after `Drive::calculate_fee` succeeded.
    /// Generation 0 applied the batch (committing it on its own without a caller
    /// transaction) and priced it afterwards, so from protocol version 15, where pricing an
    /// owner-attributed storage removal without the fee history is an error, a call passing
    /// no history persisted the write and then failed. It now fails before anything is
    /// written. With a caller transaction nothing is committed by Drive in either
    /// generation.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_document_v1(
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
                data_contract_id.into_buffer(),
                Some(&block_info.epoch),
                true,
                caller_transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Document(DocumentError::DataContractNotFound))?;

        let contract = &contract_fetch_info.contract;

        let document_type = contract.document_type_for_name(document_type_name)?;

        let document_and_contract_info = DocumentAndContractInfo {
            owned_document_info,
            contract,
            document_type,
        };
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.add_document_for_contract_apply_and_add_to_operations(
            document_and_contract_info,
            override_document,
            block_info,
            true,
            apply,
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
