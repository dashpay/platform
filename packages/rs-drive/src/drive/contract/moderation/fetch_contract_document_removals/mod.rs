mod v0;

use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::config::moderation::ContractDocumentRemoval;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The records of the documents a contract's moderators deleted, within one document type:
    /// the ones of the ids named, or one page in document id order. A contract or a document
    /// type that keeps no records reads as none.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose moderators deleted the documents.
    /// * `query`: The document type and the selection: document ids, or a page.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ContractDocumentRemovalEntry>)` with the records found, in document id order;
    ///   empty when the contract or the document type keeps no records.
    /// * `Err(Error)` when the method version is unknown, the query names no ids, too many or a
    ///   repeated id, or has a limit of zero or above the maximum, a read fails, or a stored
    ///   record is malformed.
    pub fn fetch_contract_document_removals(
        &self,
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractDocumentRemovalEntry>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_document_removals
        {
            0 => self.fetch_contract_document_removals_v0(
                contract_id,
                query,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_document_removals".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The record of one document a moderator deleted, with the fee of the read, so that
    /// consensus validation can bill it. The document type must be one whose documents
    /// moderators may delete: its records tree exists since the type was created, so a
    /// missing record reads as `None` and nothing else does.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the document belonged to.
    /// * `document_type_name`: The document's type.
    /// * `document_id`: The document's id.
    /// * `epoch`: The epoch the read is priced in.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, Option<ContractDocumentRemoval>))`: the fee of the read and the
    ///   record, `None` when the document has none.
    /// * `Err(Error)` when the method version is unknown, the read fails, the record is
    ///   malformed, or the fee cannot be calculated.
    pub fn fetch_contract_document_removal_with_fee(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<ContractDocumentRemoval>), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_document_removals
        {
            0 => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let removal = self.fetch_contract_document_removal_add_to_operations_v0(
                    contract_id,
                    document_type_name,
                    document_id,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let fee = Drive::calculate_fee(
                    None,
                    Some(drive_operations),
                    epoch,
                    self.config.epochs_per_era,
                    platform_version,
                    None,
                )?;
                Ok((fee, removal))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_document_removal_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
