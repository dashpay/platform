use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use crate::error::Error;
use dpp::block::epoch::Epoch;

use dpp::identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransitionAction;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransitionAction;

impl DriveHighLevelOperationConverter for IdentityCreditWithdrawalTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let IdentityCreditWithdrawalTransitionAction {
            prepared_withdrawal_document,
            identity_id,
            revision,
            ..
        } = self;

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::UpdateIdentityRevision {
                identity_id: identity_id.into_buffer(),
                revision,
            }),
            DocumentOperation(DocumentOperationType::AddWithdrawalDocument {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentOwnedInfo((
                        prepared_withdrawal_document,
                        None,
                    )),
                    owner_id: None,
                },
            }),
        ];

        Ok(drive_operations)
    }
}
