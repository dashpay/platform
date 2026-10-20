mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Prepares the operations for deleting an **indexOnly** document.
    ///
    /// indexOnly documents have no primary-storage row, so there is nothing
    /// to fetch by id: the caller reconstructs the document from the delete
    /// transition's property values and owner, and every index entry is
    /// recomputed from it — the exact mirror of what the create wrote. The
    /// surviving entries must match the row commitment. Paths already
    /// drained from expired TTL buckets are skipped in validation and apply.
    ///
    /// # Parameters
    ///
    /// * `document`: The document, reconstructed from the transition's values and owner.
    /// * `contract`: The contract of the document.
    /// * `document_type`: The document's type, which must be indexOnly and deletable.
    /// * `previous_batch_operations`: The operations already in the batch, read before the
    ///   stored state.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `block_time_ms`: The block time; outside estimation, the TTL buckets expired by then
    ///   are drained first, and entries already drained are skipped.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn delete_index_only_document_for_contract_operations(
        &self,
        document: Document,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if estimated_costs_only_with_layer_info.is_none() {
            self.prepare_document_time_range_ttl(
                contract,
                document_type,
                block_time_ms,
                transaction,
                platform_version,
            )?;
        }
        self.delete_index_only_document_for_contract_operations_without_ttl_drain(
            document,
            contract,
            document_type,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            block_time_ms,
            transaction,
            platform_version,
        )
    }

    /// Build against post-drain state. The caller must prepare the whole
    /// batch before invoking this method; no cleanup occurs during conversion.
    ///
    /// # Parameters
    ///
    /// * `document`: The document, reconstructed from the transition's values and owner.
    /// * `contract`: The contract of the document.
    /// * `document_type`: The document's type, which must be indexOnly and deletable.
    /// * `previous_batch_operations`: The operations already in the batch, read before the
    ///   stored state.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `block_time_ms`: The block time; entries in TTL buckets expired and drained by then are
    ///   skipped.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the row-commitment probe reads and the removal
    ///   of every index entry of the document.
    /// * `Err(Error)` when the method version is unknown, the document type is not indexOnly or
    ///   its documents cannot be deleted, an index entry is missing or carries another
    ///   document's row commitment, or a read or an operation fails.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn delete_index_only_document_for_contract_operations_without_ttl_drain(
        &self,
        document: Document,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_index_only_document_for_contract_operations
        {
            0 => self.delete_index_only_document_for_contract_operations_v0(
                document,
                contract,
                document_type,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_index_only_document_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Deletes an indexOnly document (reconstructed from its property
    /// values and owner) and applies the operations, returning the fee.
    /// `apply: false` runs the worst-case estimation instead — the same
    /// dry-run contract every other document operation follows.
    /// `previous_fee_versions` carries the historical fee-version context
    /// deletion refunds are priced against, exactly as on
    /// `delete_document_for_contract`.
    ///
    /// # Parameters
    ///
    /// * `document`: The document, reconstructed from the transition's values and owner.
    /// * `contract`: The contract of the document.
    /// * `document_type`: The document's type, which must be indexOnly and deletable.
    /// * `block_info`: The block being executed; its time drives the TTL drain and its epoch
    ///   prices the fee.
    /// * `apply`: Whether to apply the operations or only estimate their cost.
    /// * `transaction`: The GroveDB transaction; without one, an applied deletion runs in a
    ///   transaction of its own that is committed at the end.
    /// * `platform_version`: The platform version.
    /// * `previous_fee_versions`: The fee versions of earlier epochs the refunds are priced
    ///   against.
    ///
    /// # Returns
    ///
    /// * `Ok(FeeResult)` with the fee of the deletion (applied or estimated) and its refunds.
    /// * `Err(Error)` when the method version is unknown, the document type is not indexOnly or
    ///   its documents cannot be deleted, no document with exactly these values exists for the
    ///   owner, or a read, the batch apply, the commit or the fee calculation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn delete_index_only_document_for_contract(
        &self,
        document: Document,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_index_only_document_for_contract
        {
            0 => self.delete_index_only_document_for_contract_v0(
                document,
                contract,
                document_type,
                block_info,
                apply,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_index_only_document_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
