use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::data_contract::document_type::DocumentTypeRef;

use std::collections::HashMap;

use crate::util::object_size_info::DocumentInfo::{
    DocumentEstimatedAverageSize, DocumentOwnedInfo,
};

use dpp::data_contract::DataContract;
use dpp::document::Document;

use crate::drive::Drive;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};

use crate::error::drive::DriveError;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0Getters;

use dpp::version::PlatformVersion;

use dpp::block::block_info::BlockInfo;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;

impl Drive {
    /// The full fee-applying deletion for an indexOnly document: gather the
    /// operations, apply (or dry-run) the batch, and price it — the same
    /// orchestration `delete_document_for_contract`'s v0 performs for
    /// stored documents.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_index_only_document_for_contract_v0(
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
            previous_fee_versions,
        )
    }

    /// Prepares the operations for deleting an indexOnly document.
    ///
    /// There is no primary row to fetch or remove: the document handed in
    /// is reconstructed from the delete transition's values and owner, and
    /// the index walkers recompute every entry from it, keying each
    /// terminal removal by the index's terminal property value. Storage
    /// refunds need no document-level flags — grovedb reads each deleted
    /// entry's own element flags during the batch apply, exactly as it
    /// does for reference entries.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_index_only_document_for_contract_operations_v0(
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
        if !document_type.index_only() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "delete_index_only_document_for_contract_operations requires an indexOnly \
                 document type; use delete_document_for_contract_operations for stored \
                 documents",
            )));
        }

        if !document_type.documents_can_be_deleted() {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "this document type can not be deleted",
            )));
        }

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                contract,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let owner_id = Some(document.owner_id().to_buffer());

        let document_info = if estimated_costs_only_with_layer_info.is_some() {
            DocumentEstimatedAverageSize(document_type.estimated_size(platform_version)? as u32)
        } else {
            DocumentOwnedInfo((document, None))
        };

        let document_and_contract_info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info,
                owner_id,
            },
            contract,
            document_type,
        };

        self.remove_indices_for_top_index_level_for_contract_operations(
            &document_and_contract_info,
            &previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        Ok(batch_operations)
    }
}
