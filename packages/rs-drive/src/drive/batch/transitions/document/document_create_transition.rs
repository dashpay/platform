use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentWithoutSerialization;
use crate::drive::object_size_info::OwnedDocumentInfo;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use dpp::data_contract::DriveContractExt;
use dpp::document::document_transition::{
    DocumentBaseTransitionAction, DocumentCreateTransitionAction,
};
use dpp::document::Document;
use dpp::prelude::Identifier;
use std::borrow::Cow;

impl DriveHighLevelDocumentOperationConverter for DocumentCreateTransitionAction {
    fn into_high_level_document_drive_operations(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
    ) -> Result<Vec<DriveOperation>, Error> {
        let DocumentCreateTransitionAction {
            base,
            created_at,
            updated_at,
            data,
        } = self;

        let DocumentBaseTransitionAction {
            id,
            document_type_name,
            data_contract_id,
            data_contract,
        } = base;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        let document = Document {
            id,
            owner_id,
            properties: data,
            revision: document_type.initial_revision(),
            created_at,
            updated_at,
        };

        let storage_flags = StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

        let mut drive_operations = vec![];
        drive_operations.push(DocumentOperation(DocumentOperationType::AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentWithoutSerialization((
                    document,
                    Some(Cow::Owned(storage_flags)),
                )),
                owner_id: Some(owner_id.into_buffer()),
            },
            contract_id: data_contract_id,
            document_type_name: Cow::Owned(document_type_name),
            override_document: false,
        }));

        Ok(drive_operations)
    }
}
