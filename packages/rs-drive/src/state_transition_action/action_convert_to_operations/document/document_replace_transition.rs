use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::document::DriveHighLevelDocumentOperationConverter;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;

use dpp::document::Document;
use dpp::prelude::Identifier;
use std::borrow::Cow;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentFromReplaceTransitionAction, DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::drive::DriveError;

impl DriveHighLevelDocumentOperationConverter for DocumentReplaceTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .document_replace_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let document_type_name = self.base().document_type_name().clone();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let contract_fetch_info = self.base().data_contract_fetch_info();
                let document = Document::try_from_owned_replace_transition_action(
                    self,
                    owner_id,
                    platform_version,
                )?;

                let storage_flags =
                    StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

                Ok(vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    DocumentOperation(DocumentOperationType::UpdateDocument {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentOwnedInfo((
                                document,
                                Some(Cow::Owned(storage_flags)),
                            )),
                            owner_id: Some(owner_id.into_buffer()),
                        },
                        contract_info: DataContractInfo::DataContractFetchInfo(contract_fetch_info),
                        document_type_info: DocumentTypeInfo::DocumentTypeName(document_type_name),
                    }),
                ])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "DocumentReplaceTransitionAction::into_high_level_document_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
