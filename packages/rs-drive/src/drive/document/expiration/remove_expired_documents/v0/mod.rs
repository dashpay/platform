use crate::drive::contract::DataContractFetchInfo;
use crate::drive::document::expiration::paths::{
    documents_expirations_path_vec, encode_expiration_time,
};
use crate::drive::document::expiration::pricing::document_expires_at;
use crate::drive::document::expiration::{
    DocumentExpirationEntry, ExpiredDocument, RemovedExpiredDocuments,
};
use crate::drive::document::paths::contract_documents_primary_key_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::{DocumentOperationType, DriveOperation};
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::operations::delete::DeleteOptions;
use grovedb::{BackwardsReferences, Element, MaybeTree, TransactionArg, TreeType};
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
        // Maintenance nobody is billed for: the reads' costs are dropped.
        let expired = self.fetch_expired_documents(
            block_info.time_ms,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )?;
        if expired.expiry_times.is_empty() {
            return Ok(removed);
        }

        // The deletions and the final batch live in helpers of their own: this runs at the
        // end of every block, and their large temporaries must not sit in the frame that is
        // live while the expirations tree is read.
        let mut orphans: Vec<&ExpiredDocument> = vec![];
        for expired_document in &expired.documents {
            if self.delete_expired_document(
                expired_document,
                block_info,
                transaction,
                platform_version,
            )? {
                removed.deleted_documents += 1;
            } else {
                orphans.push(expired_document);
            }
        }
        self.drop_orphaned_entries_and_empty_expiry_times(
            &orphans,
            &expired.expiry_times,
            &mut removed,
            transaction,
            platform_version,
        )?;
        Ok(removed)
    }

    /// Deletes one expired document in a batch of its own, `false` when its entry has no
    /// document to delete (see [`Self::expired_document_contract`]). The fee result is
    /// discarded: the document prepaid its deletion when it was created, and whatever it
    /// refunds (nothing, as it carries no storage flags) goes to nobody.
    #[inline(never)]
    fn delete_expired_document(
        &self,
        expired_document: &ExpiredDocument,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let Some(contract) =
            self.expired_document_contract(expired_document, transaction, platform_version)?
        else {
            return Ok(false);
        };
        self.apply_drive_operations(
            vec![DriveOperation::DocumentOperation(
                DocumentOperationType::DeleteExpiredDocument {
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
        Ok(true)
    }

    /// Removes the entries left without a document to delete, then drops every tree of an
    /// expiry time found empty: the deletions removed their documents' entries, and a
    /// document deleted earlier by its owner or a moderator removed its own. A tree still
    /// holding documents (more than this block could delete) stays.
    #[inline(never)]
    fn drop_orphaned_entries_and_empty_expiry_times(
        &self,
        orphans: &[&ExpiredDocument],
        expiry_times: &[TimestampMillis],
        removed: &mut RemovedExpiredDocuments,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations: Vec<LowLevelDriveOperation> = vec![];
        for orphan in orphans {
            self.remove_document_expiration_operations(
                orphan.document_id.to_buffer(),
                orphan.expires_at_ms,
                DocumentExpirationEntry::serialized_size(&orphan.document_type_name),
                &mut None,
                transaction,
                &mut operations,
                platform_version,
            )?;
            removed.orphaned_entries += 1;
        }
        let expirations_path = documents_expirations_path_vec();
        for expires_at_ms in expiry_times {
            let queued_before = count_grove_operations(&operations);
            self.batch_delete_with_options(
                expirations_path.as_slice().into(),
                &encode_expiration_time(*expires_at_ms),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(TreeType::NormalTree)),
                },
                DeleteOptions {
                    backwards_references: BackwardsReferences::DontCheck,
                    allow_deleting_non_empty_trees: false,
                    deleting_non_empty_trees_returns_error: false,
                    base_root_storage_is_free: true,
                    validate_tree_at_path_exists: false,
                },
                transaction,
                &mut operations,
                &platform_version.drive,
            )?;
            if count_grove_operations(&operations) > queued_before {
                removed.dropped_expiry_times += 1;
            }
        }
        if count_grove_operations(&operations) > 0 {
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                operations,
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        Ok(())
    }

    /// The contract to delete an expired document through, `None` when the entry has no
    /// document to delete: its contract, document type or document is gone, the type declares
    /// no time to live, or the document expires at another time than its entry says. None of
    /// these can happen (contracts, document types and a type's time to live are permanent,
    /// and every deletion removes the document's entry), so each is logged and only the entry
    /// removed: a deletion that fails would fail the block.
    #[inline(never)]
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

fn count_grove_operations(operations: &[LowLevelDriveOperation]) -> usize {
    operations
        .iter()
        .filter(|operation| matches!(operation, LowLevelDriveOperation::GroveOperation(_)))
        .count()
}
