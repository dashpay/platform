use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation;
use crate::error::Error;
use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

impl DriveHighLevelDocumentOperationConverter for DocumentTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match self {
            DocumentTransitionAction::CreateAction(document_create_transition) => {
                document_create_transition.into_high_level_document_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            DocumentTransitionAction::ReplaceAction(document_replace_transition) => {
                document_replace_transition.into_high_level_document_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            DocumentTransitionAction::DeleteAction(document_delete_transition) => {
                document_delete_transition.into_high_level_document_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            DocumentTransitionAction::TransferAction(document_transfer_transition) => {
                document_transfer_transition.into_high_level_document_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            DocumentTransitionAction::BumpIdentityDataContractNonce(
                bump_identity_contract_nonce_action,
            ) => bump_identity_contract_nonce_action
                .into_high_level_drive_operations(epoch, platform_version),
        }
    }
}
