use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use dpp::data_contract::document_type::DocumentTypeRef;

use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use crate::drive::batch::drive_op_batch::{
    DocumentOperation, DocumentOperationsForContractDocumentType, UpdateOperationInfo,
};
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::defaults::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::{
    contract_document_type_path,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_primary_key_path, make_document_reference,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentOwnedInfo, DocumentRefAndSerialization, DocumentRefInfo,
};
use dpp::data_contract::DataContract;
use dpp::document::Document;

use crate::drive::object_size_info::PathKeyElementInfo::PathKeyRefElement;
use crate::drive::object_size_info::{
    DocumentAndContractInfo, DriveKeyInfo, OwnedDocumentInfo, PathKeyInfo,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef, KeySize};
use crate::error::document::DocumentError;
use dpp::block::block_info::BlockInfo;
#[cfg(feature = "data-contract-cbor-conversion")]
use dpp::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;
use dpp::document::serialization_traits::{
    DocumentCborMethodsV0, DocumentPlatformConversionMethodsV0,
};
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

use crate::drive::grove_operations::{
    BatchDeleteUpTreeApplyType, BatchInsertApplyType, BatchInsertTreeApplyType, DirectQueryType,
    QueryType,
};

impl Drive {
    /// Updates a serialized document given a contract CBOR and returns the associated fee.
    pub fn update_document_for_contract_cbor(
        &self,
        serialized_document: &[u8],
        contract_cbor: &[u8],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let contract = DataContract::from_cbor(contract_cbor, platform_version)?;

        let document = Document::from_cbor(serialized_document, None, owner_id, platform_version)?;

        let document_type = contract.document_type_for_name(document_type_name)?;

        let reserialized_document = document.serialize(document_type, platform_version)?;

        self.update_document_with_serialization_for_contract(
            &document,
            reserialized_document.as_slice(),
            &contract,
            document_type_name,
            owner_id,
            block_info,
            apply,
            storage_flags,
            transaction,
            platform_version,
        )
    }
}
