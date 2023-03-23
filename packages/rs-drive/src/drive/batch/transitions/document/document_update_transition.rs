use std::borrow::Cow;
use dpp::data_contract::DriveContractExt;
use dpp::document::Document;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::{DocumentBaseTransitionAction, DocumentReplaceTransition, DocumentReplaceTransitionAction};
use dpp::prelude::Identifier;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::object_size_info::DocumentInfo::DocumentWithoutSerialization;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

impl DriveHighLevelDocumentOperationConverter for DocumentReplaceTransitionAction {
    fn to_high_level_document_drive_operations(self, epoch: &Epoch, owner_id: Identifier) -> Result<Vec<DriveOperation>, Error> {
        let DocumentReplaceTransitionAction {
            base, revision, created_at, updated_at, data
        } = self;

        let DocumentBaseTransitionAction {
            id, document_type_name, data_contract_id, data_contract
        } = base;

        let document = Document {
            id,
            owner_id,
            properties: data.into_btree_string_map()?,
            revision: Some(revision),
            created_at,
            updated_at,
        };

        let storage_flags = StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

        let mut drive_operations = vec![];
        drive_operations.push(DocumentOperation(DocumentOperationType::UpdateDocument {
            owned_document_info: OwnedDocumentInfo { document_info: DocumentWithoutSerialization((document, Some(Cow::Owned(storage_flags)))), owner_id: Some(owner_id.into_buffer()) },
            contract_id: data_contract_id,
            document_type_name: document_type_name.as_str(),
        }));

        Ok(drive_operations)
    }
}