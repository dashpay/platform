use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
    BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0,
};
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
impl DriveHighLevelOperationConverter for BumpIdentityDataContractNonceAction {
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
            .bump_identity_data_contract_nonce
        {
            0 => {
                let identity_id = self.identity_id();
                let data_contract_id = self.data_contract_id();

                let identity_contract_nonce = self.identity_contract_nonce();

                Ok(vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: identity_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "BumpIdentityDataContractNonceAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionV0;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;

    fn make_action() -> BumpIdentityDataContractNonceAction {
        BumpIdentityDataContractNonceAction::V0(BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from([0xAA; 32]),
            data_contract_id: Identifier::from([0xBB; 32]),
            identity_contract_nonce: 99,
            user_fee_increase: 7,
        })
    }

    #[test]
    fn test_into_high_level_drive_operations_returns_update_identity_contract_nonce() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                identity_id,
                contract_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*contract_id, [0xBB; 32]);
                assert_eq!(*nonce, 99);
            }
            other => panic!("expected UpdateIdentityContractNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_with_zero_values() {
        let action =
            BumpIdentityDataContractNonceAction::V0(BumpIdentityDataContractNonceActionV0 {
                identity_id: Identifier::from([0x00; 32]),
                data_contract_id: Identifier::from([0x00; 32]),
                identity_contract_nonce: 0,
                user_fee_increase: 0,
            });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                identity_id,
                contract_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0x00; 32]);
                assert_eq!(*contract_id, [0x00; 32]);
                assert_eq!(*nonce, 0);
            }
            other => panic!("expected UpdateIdentityContractNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_identity_and_contract_ids_are_distinct() {
        let action =
            BumpIdentityDataContractNonceAction::V0(BumpIdentityDataContractNonceActionV0 {
                identity_id: Identifier::from([0x11; 32]),
                data_contract_id: Identifier::from([0x22; 32]),
                identity_contract_nonce: 1,
                user_fee_increase: 0,
            });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                identity_id,
                contract_id,
                ..
            }) => {
                assert_ne!(identity_id, contract_id);
            }
            other => panic!("expected UpdateIdentityContractNonce, got {:?}", other),
        }
    }
}
