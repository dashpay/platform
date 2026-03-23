use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation::{AddressFundsOperation, IdentityOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityTopUpFromAddressesTransitionAction {
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
            .identity_top_up_from_addresses_transition
        {
            0 => {
                let identity_id = self.identity_id();
                let topup_amount = self.topup_amount();
                let output = self.output();
                let inputs = self.inputs_with_remaining_balance_owned();

                let mut drive_operations = vec![IdentityOperation(
                    IdentityOperationType::AddToIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        added_balance: topup_amount,
                    },
                )];

                // Update input address balances
                for (address, (nonce, remaining_balance)) in inputs {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address,
                            nonce,
                            balance: remaining_balance,
                        },
                    ));
                }

                // Handle output address if present
                if let Some((output_address, output_balance)) = output {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address: output_address,
                            balance_to_add: output_balance,
                        },
                    ));
                }

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "IdentityTopUpFromAddressesTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_action_with_output() -> IdentityTopUpFromAddressesTransitionAction {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1_u32, 900_u64));

        IdentityTopUpFromAddressesTransitionAction::V0(
            IdentityTopUpFromAddressesTransitionActionV0 {
                inputs_with_remaining_balance: inputs,
                output: Some((PlatformAddress::P2pkh([0x33; 20]), 200)),
                fee_strategy: vec![],
                identity_id: Identifier::from([0xAA; 32]),
                topup_amount: 1200,
                user_fee_increase: 0,
            },
        )
    }

    fn make_action_no_output() -> IdentityTopUpFromAddressesTransitionAction {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1_u32, 0_u64));

        IdentityTopUpFromAddressesTransitionAction::V0(
            IdentityTopUpFromAddressesTransitionActionV0 {
                inputs_with_remaining_balance: inputs,
                output: None,
                fee_strategy: vec![],
                identity_id: Identifier::from([0xAA; 32]),
                topup_amount: 1000,
                user_fee_increase: 0,
            },
        )
    }

    #[test]
    fn test_with_output_produces_add_balance_set_balance_and_add_balance_to_address() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddToIdentityBalance + SetBalanceToAddress (1 input) + AddBalanceToAddress (output)
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_adds_to_identity_balance() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id,
                added_balance,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*added_balance, 1200);
            }
            other => panic!("expected AddToIdentityBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_no_output_skips_add_balance_to_address() {
        let action = make_action_no_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddToIdentityBalance + SetBalanceToAddress (1 input), no AddBalanceToAddress
        assert_eq!(ops.len(), 2);

        // Ensure no AddBalanceToAddress operations
        for op in &ops {
            assert!(
                !matches!(
                    op,
                    AddressFundsOperation(AddressFundsOperationType::AddBalanceToAddress { .. })
                ),
                "should not have AddBalanceToAddress when output is None"
            );
        }
    }
}
