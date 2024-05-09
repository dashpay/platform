use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

impl Drive {
    /// Updates a serialized document given a contract id and returns the associated fee.
    #[inline(always)]
    pub(super) fn update_document_for_contract_id_v0(
        &self,
        serialized_document: &[u8],
        contract_id: [u8; 32],
        document_type: &str,
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

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Document(DocumentError::DataContractNotFound))?;

        let contract = &contract_fetch_info.contract;

        let document_type = contract.document_type_for_name(document_type)?;

        let document = Document::from_bytes(serialized_document, document_type, platform_version)?;

        let document_info =
            DocumentRefAndSerialization((&document, serialized_document, storage_flags));

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
