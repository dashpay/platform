use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, SystemOperation};
use crate::util::batch::{
    DocumentOperationType, DriveOperation, IdentityOperationType, SystemOperationType,
};
use crate::util::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use dpp::block::epoch::Epoch;

use crate::error::drive::DriveError;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreditWithdrawalTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .identity_credit_withdrawal_transition
        {
            0 => {
                let identity_id = self.identity_id();
                let nonce = self.nonce();
                let balance_to_remove = self.amount();
                let prepared_withdrawal_document = self.prepared_withdrawal_document_owned();

                let drive_operations = vec![
                    IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        balance_to_remove,
                    }),
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.into_buffer(),
                        nonce,
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
                    SystemOperation(SystemOperationType::RemoveFromSystemCredits {
                        amount: balance_to_remove,
                    }),
                ];

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "IdentityCreditWithdrawalTransitionAction::into_high_level_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
