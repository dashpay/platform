use std::borrow::Cow;
use grovedb::TransactionArg;
use serde::Deserialize;
use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::version::drive_versions::DriveVersion;
use crate::contract::Contract;
use crate::drive::Drive;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use crate::error::Error;
use crate::fee::result::FeeResult;

impl Drive {
    /// Deserializes a document and adds it to a contract.
    pub fn add_serialized_document_for_contract_v0(
        &self,
        serialized_document: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        let document = Document::deserialize(serialized_document)?;

        let document_info =
            DocumentRefAndSerialization((&document, serialized_document, storage_flags));

        let document_type = contract.document_type_for_name(document_type_name)?;

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
            drive_version,
        )
    }
}