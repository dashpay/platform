use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::base::DataContractBaseMethodsV0;
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use serde::Deserialize;
use std::borrow::Cow;

impl Drive {
    /// Deserializes a document and adds it to a contract.
    pub fn add_serialized_document_for_contract_v0(
        &self,
        serialized_document: &[u8],
        contract: &DataContract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let document_type = contract.document_type_for_name(document_type_name)?;

        let document = Document::from_bytes(serialized_document, document_type, platform_version)?;

        let document_info =
            DocumentRefAndSerialization((&document, serialized_document, storage_flags));

        self.add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info,
                    owner_id,
                },
                contract,
                document_type,
            },
            override_document,
            block_info,
            apply,
            transaction,
            platform_version,
        )
    }
}
