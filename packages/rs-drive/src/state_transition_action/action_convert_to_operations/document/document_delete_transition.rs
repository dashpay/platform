use crate::state_transition_action::action_convert_to_operations::document::DriveHighLevelDocumentOperationConverter;

use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;

use dpp::identifier::Identifier;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use dpp::version::PlatformVersion;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use crate::error::drive::DriveError;

impl DriveHighLevelDocumentOperationConverter for DocumentDeleteTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .document_delete_transition
        {
            0 => {
                let base = self.base_owned();

                let data_contract_id = base.data_contract_id();

                let identity_contract_nonce = base.identity_contract_nonce();

                Ok(vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    DocumentOperation(DocumentOperationType::DeleteDocument {
                        document_id: base.id(),
                        contract_info: DataContractInfo::DataContractFetchInfo(
                            base.data_contract_fetch_info(),
                        ),
                        document_type_info: DocumentTypeInfo::DocumentTypeName(
                            base.document_type_name_owned(),
                        ),
                    }),
                ])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentDeleteTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
