use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::AddressFundsOperation;
use dpp::block::epoch::Epoch;
use platform_version::version::PlatformVersion;

impl DriveHighLevelOperationConverter for AddressFundsTransferTransitionAction {
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
            .address_funds_transfer_transition
        {
            0 => {
                let (inputs, outputs) = self.inputs_with_remaining_balance_and_outputs_owned();
                let mut drive_operations = vec![];

                for (address, (nonce, remaining_balance)) in inputs {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address,
                            nonce,
                            balance: remaining_balance,
                        },
                    ));
                }

                for (address, balance_to_add) in outputs {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address,
                            balance_to_add,
                        },
                    ));
                }
                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "AddressFundsTransferTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_action() -> AddressFundsTransferTransitionAction {
        let addr_input = PlatformAddress::P2pkh([0xAA; 20]);
        let addr_output = PlatformAddress::P2pkh([0xBB; 20]);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr_input, (1_u32, 5000_u64));

        let mut outputs = BTreeMap::new();
        outputs.insert(addr_output, 3000_u64);

        AddressFundsTransferTransitionAction::V0(AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            outputs,
            fee_strategy: vec![],
            user_fee_increase: 0,
        })
    }

    #[test]
    fn test_produces_set_balance_and_add_balance_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // 1 SetBalanceToAddress (input) + 1 AddBalanceToAddress (output)
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn test_first_op_is_set_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xAA; 20]));
                assert_eq!(*nonce, 1);
                assert_eq!(*balance, 5000);
            }
            other => panic!("expected SetBalanceToAddress, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_is_add_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            AddressFundsOperation(AddressFundsOperationType::AddBalanceToAddress {
                address,
                balance_to_add,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xBB; 20]));
                assert_eq!(*balance_to_add, 3000);
            }
            other => panic!("expected AddBalanceToAddress, got {:?}", other),
        }
    }

    #[test]
    fn test_empty_inputs_and_outputs() {
        let action =
            AddressFundsTransferTransitionAction::V0(AddressFundsTransferTransitionActionV0 {
                inputs_with_remaining_balance: BTreeMap::new(),
                outputs: BTreeMap::new(),
                fee_strategy: vec![],
                user_fee_increase: 0,
            });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert!(ops.is_empty());
    }
}
