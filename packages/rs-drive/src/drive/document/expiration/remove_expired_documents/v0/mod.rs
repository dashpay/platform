use crate::drive::contract::DataContractFetchInfo;
use crate::drive::document::expiration::pricing::{
    document_expires_at, document_type_weighted_index_levels,
};
use crate::drive::document::expiration::{
    DocumentExpirationEntry, ExpiredDocument, RemovedExpiredDocuments,
};
use crate::drive::document::paths::contract_documents_primary_key_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use std::borrow::Cow;
use std::sync::Arc;

/// An expired document read and checked against state, ready to delete.
struct ExpiredDocumentToDelete {
    contract: Arc<DataContractFetchInfo>,
    document: Document,
    storage_flags: Option<StorageFlags>,
    /// 1 plus the weighted index levels of its type
    weight: u64,
}

impl Drive {
    #[inline(always)]
    pub(super) fn remove_expired_documents_v0(
        &self,
        block_info: &BlockInfo,
        limit: u16,
        weight_budget: u32,
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

        // Every removal of the block goes into one batch, each built against those queued
        // before it: every emptiness check sees them, so the tree of an expiry time, like an
        // index value's, goes with its last entry whichever removal takes it.
        let mut operations: Vec<LowLevelDriveOperation> = vec![];
        let mut weight: u64 = 0;
        for expired_document in &expired {
            let to_delete =
                self.expired_document_to_delete(expired_document, transaction, platform_version)?;
            // An orphan's removal weighs 1. The block's first removal always runs, so the
            // backlog drains whatever a document weighs.
            let removal_weight = to_delete.as_ref().map_or(1, |to_delete| to_delete.weight);
            let removals = removed.deleted_documents + removed.orphaned_entries;
            if removals > 0 && weight.saturating_add(removal_weight) > u64::from(weight_budget) {
                break;
            }
            weight = weight.saturating_add(removal_weight);
            let Some(ExpiredDocumentToDelete {
                contract,
                document,
                storage_flags,
                ..
            }) = to_delete
            else {
                self.remove_document_expiration_operations(
                    expired_document.document_id.to_buffer(),
                    expired_document.expires_at_ms,
                    DocumentExpirationEntry::serialized_size(&expired_document.document_type_name),
                    &mut None,
                    &None,
                    transaction,
                    &mut operations,
                    platform_version,
                )?;
                removed.orphaned_entries += 1;
                continue;
            };
            let document_type = contract
                .contract
                .document_type_optional_for_name(&expired_document.document_type_name)
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "the type of an expired document to delete was found when it was read",
                )))?;
            // The deletion a document's own owner runs, from the document already read, and
            // without the `canBeDeleted` guard.
            let document_operations = self.delete_read_document_for_contract_operations_v0(
                expired_document.document_id,
                DocumentOwnedInfo((document, storage_flags.map(Cow::Owned))),
                &contract.contract,
                document_type,
                Some(&mut operations),
                &mut None,
                block_info.time_ms,
                transaction,
                vec![],
                platform_version,
            )?;
            operations.extend(document_operations);
            removed.deleted_documents += 1;
        }
        // No fee: the documents prepaid their deletion when they were created, and what they
        // would refund (nothing, as they carry no storage flags) is owed to nobody.
        if !operations.is_empty() {
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                operations,
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        Ok(removed)
    }

    /// Reads and checks the document an expired entry points to, `None` when the entry has no
    /// document to delete: its contract, document type or document is gone, the type declares
    /// no time to live, the stored document does not decode, or it expires at another time
    /// than its entry says. None of these can happen (contracts, document types and a type's
    /// time to live are permanent, and every deletion removes the document's entry), so each
    /// is logged and only the entry removed: a deletion that fails would fail the block.
    fn expired_document_to_delete(
        &self,
        expired_document: &ExpiredDocument,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ExpiredDocumentToDelete>, Error> {
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
        let (serialized, element_flags) = match element {
            Some(Element::Item(serialized, flags))
            | Some(Element::ItemWithSumItem(serialized, _, flags)) => (serialized, flags),
            Some(_) => return orphan("the stored document is not an item"),
            None => return orphan("the document does not exist"),
        };
        let Ok(document) = Document::from_bytes(&serialized, document_type, platform_version)
        else {
            return orphan("the stored document does not decode");
        };
        let Ok(storage_flags) = StorageFlags::map_some_element_flags_ref(&element_flags) else {
            return orphan("the stored document's storage flags do not decode");
        };
        let Some(created_at) = document.created_at() else {
            return orphan("the document has no creation time");
        };
        if document_expires_at(created_at, ttl_seconds)? != expired_document.expires_at_ms {
            return orphan("the document expires at another time than its entry");
        }
        let weight = document_type_weighted_index_levels(document_type).saturating_add(1);
        Ok(Some(ExpiredDocumentToDelete {
            contract,
            document,
            storage_flags,
            weight,
        }))
    }
}
