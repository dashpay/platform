mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
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
    /// entries must exist; a missing one fails the batch at apply time
    /// (state validation probes them beforehand).
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
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.delete_index_only_document_for_contract_operations(
            document,
            contract,
            document_type,
            None,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }
}
