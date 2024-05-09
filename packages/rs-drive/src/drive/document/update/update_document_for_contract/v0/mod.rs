use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

impl Drive {
    /// Updates a document and returns the associated fee.
    #[inline(always)]
    pub(super) fn update_document_for_contract_v0(
        &self,
        document: &Document,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let document_info = DocumentRefInfo((document, storage_flags));

        self.update_document_for_contract_apply_and_add_to_operations(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info,
                    owner_id,
                },
                contract,
                document_type,
            },
            &block_info,
            estimated_costs_only_with_layer_info,
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
        )?;
        Ok(fees)
    }
}
