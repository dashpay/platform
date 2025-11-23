use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::identity::identity_create_from_addresses::{
    IdentityCreateFromAddressesTransitionAction,
    IdentityFromIdentityCreateFromAddressesTransitionAction,
};
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation::{AddressFundsOperation, IdentityOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::identity::KeyOfTypeWithNonce;
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreateFromAddressesTransitionAction {
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
            .identity_create_from_addresses_transition
        {
            0 => {
                let identity =
                    Identity::try_from_borrowed_identity_create_from_addresses_transition_action(
                        &self,
                        platform_version,
                    )?;

                let mut drive_operations =
                    vec![IdentityOperation(IdentityOperationType::AddNewIdentity {
                        identity,
                        is_masternode_identity: false,
                    })];

                for (key_of_type, (nonce, remaining_balance)) in
                    self.inputs_with_remaining_balance_owned()
                {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            key_of_type_with_nonce: KeyOfTypeWithNonce { key_of_type, nonce },
                            balance: remaining_balance,
                        },
                    ));
                }

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "IdentityCreateFromAddressesTransitionAction::into_high_level_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
