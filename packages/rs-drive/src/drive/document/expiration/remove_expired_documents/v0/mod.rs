use crate::drive::contract::DataContractFetchInfo;
use crate::drive::document::expiration::pricing::document_expires_at;
use crate::drive::document::expiration::{
    DocumentExpirationEntry, ExpiredDocument, RemovedExpiredDocuments,
};
use crate::drive::document::paths::contract_documents_primary_key_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::{DocumentOperationType, DriveOperation};
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use std::sync::Arc;

impl Drive {
    #[inline(always)]
    pub(super) fn remove_expired_documents_v0(
        &self,
        block_info: &BlockInfo,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<RemovedExpiredDocuments, Error> {
        let mut removed = RemovedExpiredDocuments::default();
        // Maintenance nobody is billed for: the read's costs are dropped.
        let expired = self.fetch_expired_documents(
            block_info.time_ms,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )?;

        let mut orphan_operations: Vec<LowLevelDriveOperation> = vec![];
        for expired_document in &expired {
            let Some(contract) =
                self.expired_document_contract(expired_document, transaction, platform_version)?
            else {
                // Only the entry goes; the orphans of this block share one batch.
                self.remove_document_expiration_operations(
                    expired_document.document_id.to_buffer(),
                    expired_document.expires_at_ms,
                    DocumentExpirationEntry::serialized_size(&expired_document.document_type_name),
                    &mut None,
                    &None,
                    transaction,
                    &mut orphan_operations,
                    platform_version,
                )?;
                removed.orphaned_entries += 1;
                continue;
            };
            // The fee result is discarded: the document prepaid its deletion when it was
            // created, and whatever it refunds (nothing, as it carries no storage flags) goes
            // to nobody.
            self.apply_drive_operations(
                vec![DriveOperation::DocumentOperation(
                    DocumentOperationType::ForceDeleteDocument {
                        document_id: expired_document.document_id,
                        contract_info: DataContractInfo::DataContractFetchInfo(contract),
                        document_type_info: DocumentTypeInfo::DocumentTypeName(
                            expired_document.document_type_name.clone(),
                        ),
                    },
                )],
                true,
                block_info,
                transaction,
                platform_version,
                None,
            )?;
            removed.deleted_documents += 1;
        }
        if !orphan_operations.is_empty() {
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                orphan_operations,
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        Ok(removed)
    }

    /// The contract to delete an expired document through, `None` when the entry has no
    /// document to delete: its contract, document type or document is gone, the type declares
    /// no time to live, or the document expires at another time than its entry says. None of
    /// these can happen (contracts, document types and a type's time to live are permanent,
    /// and every deletion removes the document's entry), so each is logged and only the entry
    /// removed: a deletion that fails would fail the block.
    fn expired_document_contract(
        &self,
        expired_document: &ExpiredDocument,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        let orphan = |reason: &str| {
            tracing::warn!(
                document_id = %expired_document.document_id,
                contract_id = %expired_document.contract_id,
                document_type = expired_document.document_type_name,
                expires_at_ms = expired_document.expires_at_ms,
                reason,
                "removing a document expiration entry without deleting a document"
            );
            Ok(None)
        };
        // Unbilled: the cleanup is paid for when each document is created.
        let (_, Some(contract)) = self.get_contract_with_fetch_info_and_fee(
            expired_document.contract_id.to_buffer(),
            None,
            true,
            transaction,
            platform_version,
        )?
        else {
            return orphan("the contract does not exist");
        };
        let Some(document_type) = contract
            .contract
            .document_type_optional_for_name(&expired_document.document_type_name)
        else {
            return orphan("the document type does not exist");
        };
        let Some(ttl_seconds) = document_type.documents_ttl_seconds() else {
            return orphan("the document type declares no time to live");
        };
        if document_type.documents_keep_history() || document_type.index_only() {
            return orphan("the document type keeps history or is indexOnly");
        }
        let primary_key_path = contract_documents_primary_key_path(
            contract.contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );
        let element = self.grove_get_raw_optional(
            (&primary_key_path).into(),
            expired_document.document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        let serialized = match element {
            Some(Element::Item(serialized, _)) | Some(Element::ItemWithSumItem(serialized, ..)) => {
                serialized
            }
            Some(_) => return orphan("the stored document is not an item"),
            None => return orphan("the document does not exist"),
        };
        let Ok(document) = Document::from_bytes(&serialized, document_type, platform_version)
        else {
            return orphan("the stored document does not decode");
        };
        let Some(created_at) = document.created_at() else {
            return orphan("the document has no creation time");
        };
        if document_expires_at(created_at, ttl_seconds)? != expired_document.expires_at_ms {
            return orphan("the document expires at another time than its entry");
        }
        Ok(Some(contract))
    }
}
