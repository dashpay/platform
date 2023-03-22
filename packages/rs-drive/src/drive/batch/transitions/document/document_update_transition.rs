use std::borrow::Cow;
use dpp::data_contract::DriveContractExt;
use dpp::document::Document;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::{DocumentBaseTransitionAction, DocumentReplaceTransition};
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::object_size_info::DocumentInfo::DocumentWithoutSerialization;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

impl DriveHighLevelDocumentOperationConverter for DocumentReplaceTransition {
    fn to_high_level_document_drive_operations(self, epoch: &Epoch) -> Result<Vec<DriveOperation>, Error> {
        let DocumentUpdateTransitionAction {
            base, updated_at, data
        } = self;

        let DocumentBaseTransitionAction {
            id, document_type_name, data_contract_id, data_contract
        } = base;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        let document = Document {
            id,
            owner_id,
            properties: data.into_btree_string_map()?,
            revision: document_type.initial_revision(),
            created_at,
            updated_at,
        };

        let storage_flags = StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(DocumentOperation(DocumentOperationType::UpdateDocument {
            owned_document_info: OwnedDocumentInfo { document_info: DocumentWithoutSerialization((document, Some(Cow::Owned(storage_flags)))), owner_id: Some(owner_id.into_buffer()) },
            contract_id: data_contract_id,
            document_type_name: document_type_name.as_str(),
        }));

        Ok(drive_operations)
    }
}