
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use dpp::data_contract::document_type::{DocumentTypeRef, IndexLevel};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use dpp::data_contract::DataContract;
use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
    CONTRACT_DOCUMENTS_PATH_HEIGHT, DEFAULT_HASH_SIZE_U8,
};
use crate::drive::document::{
    contract_document_type_path_vec, contract_documents_primary_key_path, document_reference_size,
    unique_event_id,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentEstimatedAverageSize, DocumentOwnedInfo,
};
use crate::drive::object_size_info::DriveKeyInfo::KeyRef;
use dpp::block::extended_block_info::BlockInfo;
use dpp::document::Document;

use crate::drive::grove_operations::BatchDeleteApplyType::{
    StatefulBatchDelete, StatelessBatchDelete,
};
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, PathInfo};
use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;

use crate::fee::result::FeeResult;
use dpp::block::epoch::Epoch;
use dpp::version::drive_versions::DriveVersion;


impl Drive {
    /// Deletes a document and returns the associated fee.
    /// The contract CBOR is given instead of the contract itself.
    pub(super) fn delete_document_for_contract_id_v0(
        &self,
        document_id: [u8; 32],
        contract_id: [u8; 32],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
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
                drive_version,
            )?
            .ok_or(Error::Document(DocumentError::ContractNotFound))?;

        let contract = &contract_fetch_info.contract;

        self.delete_document_for_contract_apply_and_add_to_operations(
            document_id,
            contract,
            document_type_name,
            owner_id,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut drive_operations,
            drive_version
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }
}