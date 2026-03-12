use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::bump_address_input_nonces_action::{
    BumpAddressInputNonceActionAccessorsV0, BumpAddressInputNoncesAction,
};
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::AddressFundsOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for BumpAddressInputNoncesAction {
    fn into_high_level_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .bump_identity_nonce
        {
            0 => {
                // For bump address input nonces, we need to set the balance for each input address
                // The nonce is already updated in the input
                let operations: Vec<DriveOperation<'b>> = self
                    .inputs_with_remaining_balance()
                    .iter()
                    .map(|(address, (nonce, balance))| {
                        AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                            address: *address,
                            nonce: *nonce,
                            balance: *balance,
                        })
                    })
                    .collect();

                Ok(operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "BumpAddressInputNoncesAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNoncesActionV0;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn test_into_high_level_drive_operations_with_single_input() {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xBB; 20]), (10_u32, 5000_u64));

        let action = BumpAddressInputNoncesAction::V0(BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: inputs,
            fee_strategy: vec![],
            user_fee_increase: 0,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xBB; 20]));
                assert_eq!(*nonce, 10);
                assert_eq!(*balance, 5000);
            }
            other => panic!("expected SetBalanceToAddress, got {:?}", other),
        }
    }

    #[test]
    fn test_into_high_level_drive_operations_with_multiple_inputs() {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (1_u32, 1000_u64));
        inputs.insert(PlatformAddress::P2sh([0xCC; 20]), (2_u32, 2000_u64));

        let action = BumpAddressInputNoncesAction::V0(BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: inputs,
            fee_strategy: vec![],
            user_fee_increase: 0,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 2);
        // All should be SetBalanceToAddress operations with correct payload values.
        // BTreeMap iteration is ordered by key, so P2pkh([0xAA; 20]) comes before P2sh([0xCC; 20]).
        // Collect all (address, nonce, balance) tuples for easy verification.
        let mut results: Vec<(PlatformAddress, u32, u64)> = Vec::new();
        for op in &ops {
            match op {
                AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                    address,
                    nonce,
                    balance,
                }) => {
                    results.push((*address, *nonce, *balance));
                }
                other => panic!("expected SetBalanceToAddress, got {:?}", other),
            }
        }

        assert!(results.contains(&(PlatformAddress::P2pkh([0xAA; 20]), 1, 1000)));
        assert!(results.contains(&(PlatformAddress::P2sh([0xCC; 20]), 2, 2000)));
    }

    #[test]
    fn test_into_high_level_drive_operations_with_empty_inputs() {
        let action = BumpAddressInputNoncesAction::V0(BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: BTreeMap::new(),
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
