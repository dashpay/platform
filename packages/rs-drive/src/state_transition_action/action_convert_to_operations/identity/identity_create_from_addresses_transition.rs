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

                // Add balance to change output if present
                if let Some((address, balance_to_add)) = self.output() {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address,
                            balance_to_add,
                        },
                    ));
                }

                for (address, (nonce, remaining_balance)) in
                    self.inputs_with_remaining_balance_owned()
                {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address,
                            nonce,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::identity::IdentityPublicKey;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_action_with_output() -> IdentityCreateFromAddressesTransitionAction {
        let platform_version = PlatformVersion::latest();
        let (key, _private) =
            IdentityPublicKey::random_masternode_transfer_key(1, Some(42), platform_version)
                .expect("expected a random key");
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1_u32, 900_u64));

        IdentityCreateFromAddressesTransitionAction::V0(
            IdentityCreateFromAddressesTransitionActionV0 {
                inputs_with_remaining_balance: inputs,
                output: Some((PlatformAddress::P2sh([0x22; 20]), 100)),
                fee_strategy: AddressFundsFeeStrategy::default(),
                public_keys: vec![key],
                identity_id: Identifier::from([0xAA; 32]),
                fund_identity_amount: 800,
                user_fee_increase: 0,
            },
        )
    }

    fn make_action_no_output() -> IdentityCreateFromAddressesTransitionAction {
        let platform_version = PlatformVersion::latest();
        let (key, _private) =
            IdentityPublicKey::random_masternode_transfer_key(1, Some(42), platform_version)
                .expect("expected a random key");
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1_u32, 0_u64));

        IdentityCreateFromAddressesTransitionAction::V0(
            IdentityCreateFromAddressesTransitionActionV0 {
                inputs_with_remaining_balance: inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::default(),
                public_keys: vec![key],
                identity_id: Identifier::from([0xAA; 32]),
                fund_identity_amount: 1000,
                user_fee_increase: 0,
            },
        )
    }

    #[test]
    fn test_with_output_produces_add_identity_add_balance_and_set_balance() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddNewIdentity + AddBalanceToAddress (output) + SetBalanceToAddress (1 input)
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_is_add_new_identity() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            IdentityOperation(IdentityOperationType::AddNewIdentity {
                identity,
                is_masternode_identity,
            }) => {
                assert!(!is_masternode_identity);
                use dpp::identity::accessors::IdentityGettersV0;
                assert_eq!(identity.id(), Identifier::from([0xAA; 32]));
                assert_eq!(identity.balance(), 800); // fund_identity_amount
            }
            other => panic!("expected AddNewIdentity, got {:?}", other),
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

        // AddNewIdentity + SetBalanceToAddress (1 input), no AddBalanceToAddress
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

    #[test]
    fn test_input_balances_are_set() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Find the SetBalanceToAddress operation
        let set_balance_op = ops.iter().find(|op| {
            matches!(
                op,
                AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress { .. })
            )
        });
        assert!(set_balance_op.is_some(), "expected SetBalanceToAddress op");

        match set_balance_op.unwrap() {
            AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0x11; 20]));
                assert_eq!(*nonce, 1);
                assert_eq!(*balance, 900);
            }
            _ => unreachable!(),
        }
    }
}
