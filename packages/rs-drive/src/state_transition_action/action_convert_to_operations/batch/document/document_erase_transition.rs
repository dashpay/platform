use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;

use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::v0::DocumentEraseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::DocumentEraseTransitionAction;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl DriveHighLevelBatchOperationConverter for DocumentEraseTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
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
            .document_erase_transition
        {
            0 => {
                let base = self.base_owned();
                let data_contract_id = base.data_contract_id();
                let identity_contract_nonce = base.identity_contract_nonce();

                // No token operation: erase carries no token cost, and its
                // structure validation refuses a transition that offers one.
                Ok(vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    DocumentOperation(DocumentOperationType::EraseDocument {
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
                method: "DocumentEraseTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
