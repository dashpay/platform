use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

use dpp::identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransitionAction;

impl DriveHighLevelOperationConverter for IdentityCreditWithdrawalTransitionAction {
    fn into_high_level_drive_operations(
        self,
        _epoch: &Epoch,
    ) -> Result<Vec<DriveOperation>, Error> {
        let IdentityCreditWithdrawalTransitionAction {
            prepared_withdrawal_document,
            ..
        } = self;

        let drive_operations = vec![DocumentOperation(
            DocumentOperationType::AddWithdrawalDocument {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentWithoutSerialization((
                        prepared_withdrawal_document,
                        None,
                    )),
                    owner_id: None,
                },
            },
        )];

        Ok(drive_operations)
    }
}
