use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{AddressFundsOperation, IdentityOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::IdentityCreditTransferToAddressesTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use dpp::block::epoch::Epoch;
use dpp::identity::KeyOfTypeWithNonce;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreditTransferToAddressesTransitionAction {
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
            .identity_credit_transfer_to_addresses_transition
        {
            0 => {

                let identity_id = self.identity_id();
                let nonce = self.nonce();

                let recipient_keys = self.recipient_keys_owned();

                let mut drive_operations = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.into_buffer(),
                        nonce,
                    }),
                    IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        balance_to_remove: recipient_keys.values().sum(),
                    }),
                ];

                for (recipient_key, credits) in recipient_keys {
                    drive_operations.push(AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                        key_of_type_with_nonce: KeyOfTypeWithNonce {
                            key_of_type: recipient_key,
                            nonce: 0,
                        },
                        balance: credits,
                    }));
                }
                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "IdentityCreditTransferToAddressesTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
