use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::result::FeeResult;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds a document using bincode serialization
    pub(super) fn add_document_v0(
        &self,
        owned_document_info: OwnedDocumentInfo,
        data_contract_id: Identifier,
        document_type_name: &str,
        override_document: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                data_contract_id.into_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                drive_version,
            )?
            .ok_or(Error::Document(DocumentError::ContractNotFound))?;

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
            drive_version,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }
}
