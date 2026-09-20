mod v0;

use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
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

    /// The record of one document, if a moderator deleted a document of that id, with the fee
    /// of the read. The document type must be one whose removal tree exists: one that sets
    /// `canBeDeletedByModerators`.
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

    /// A read names at least one and at most `max_returned_elements` records, whether by id or
    /// as a page, and no id twice: what bounds the proof the verifier accepts.
    pub(in crate::drive::contract::moderation) fn check_contract_document_removals_query(
        query: &ContractDocumentRemovalsQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let max_limit = platform_version.drive_abci.query.max_returned_elements;
        match &query.selection {
            ContractDocumentRemovalsSelection::DocumentIds(ids) => {
                if ids.is_empty() || ids.len() > max_limit as usize {
                    return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "contract document removals must name between 1 and {} document ids, got {}",
                        max_limit,
                        ids.len()
                    ))));
                }
                let mut sorted = ids.clone();
                sorted.sort_unstable();
                sorted.dedup();
                if sorted.len() != ids.len() {
                    return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                        "contract document removals name a document id twice".to_string(),
                    )));
                }
            }
            ContractDocumentRemovalsSelection::Page { limit, .. } => {
                if *limit == 0 || *limit > max_limit {
                    return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "contract document removals limit must be between 1 and {}, got {}",
                        max_limit, limit
                    ))));
                }
            }
        }
        Ok(())
    }
}
