use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::drive::object_size_info::OwnedDocumentInfo;
use crate::error::Error;
use dpp::block::epoch::Epoch;

use dpp::document::Document;
use dpp::prelude::Identifier;
use std::borrow::Cow;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0, DocumentFromCreateTransition};
use dpp::version::PlatformVersion;

impl DriveHighLevelDocumentOperationConverter for DocumentCreateTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let data_contract_id = self.base().data_contract_id();

        let document_type_name = self.base().document_type_name().clone();

        let document =
            Document::try_from_owned_create_transition(self, owner_id, platform_version)?;

        let storage_flags = StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

        Ok(vec![DocumentOperation(
            DocumentOperationType::AddDocument {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentOwnedInfo((document, Some(Cow::Owned(storage_flags)))),
                    owner_id: Some(owner_id.into_buffer()),
                },
                contract_id: data_contract_id,
                document_type_name: Cow::Owned(document_type_name),
                override_document: false,
            },
        )])
    }
}
