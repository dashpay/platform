use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{AddressFundsOperation, IdentityOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::IdentityCreditTransferToAddressesTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use dpp::block::epoch::Epoch;
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

                let recipient_addresses = self.recipient_addresses_owned();

                let mut drive_operations = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.into_buffer(),
                        nonce,
                    }),
                    IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        balance_to_remove: recipient_addresses.values().sum(),
                    }),
                ];

                for (address, credits) in recipient_addresses {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address,
                            balance_to_add: credits,
                        },
                    ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::v0::IdentityCreditTransferToAddressesTransitionActionV0;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_action() -> IdentityCreditTransferToAddressesTransitionAction {
        let mut recipients = BTreeMap::new();
        recipients.insert(PlatformAddress::P2pkh([0x11; 20]), 1000_u64);
        recipients.insert(PlatformAddress::P2sh([0x22; 20]), 2000_u64);

        IdentityCreditTransferToAddressesTransitionAction::V0(
            IdentityCreditTransferToAddressesTransitionActionV0 {
                recipient_addresses: recipients,
                identity_id: Identifier::from([0xAA; 32]),
                nonce: 55,
                user_fee_increase: 1,
            },
        )
    }

    #[test]
    fn test_produces_nonce_remove_and_add_balance_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // UpdateIdentityNonce + RemoveFromIdentityBalance + 2 AddBalanceToAddress
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn test_first_op_is_update_nonce() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*nonce, 55);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_removes_total_from_identity_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                identity_id,
                balance_to_remove,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*balance_to_remove, 3000); // 1000 + 2000
            }
            other => panic!(
                "expected RemoveFromIdentityBalance, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_remaining_ops_are_add_balance_to_address() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        for op in &ops[2..] {
            assert!(
                matches!(
                    op,
                    AddressFundsOperation(AddressFundsOperationType::AddBalanceToAddress { .. })
                ),
                "expected AddBalanceToAddress, got {:?}",
                op
            );
        }
    }
}
