use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::util::batch::DriveOperation::{DataContractOperation, IdentityOperation};
use crate::util::batch::{DataContractOperationType, DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl DriveHighLevelOperationConverter for DataContractUpdateTransitionAction {
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
            .data_contract_update_transition
        {
            0 => {
                Ok(vec![
                    // We must create the contract
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: self.data_contract_ref().owner_id().into_buffer(),
                        contract_id: self.data_contract_ref().id().into_buffer(),
                        nonce: self.identity_contract_nonce(),
                    }),
                    DataContractOperation(DataContractOperationType::ApplyContract {
                        contract: Cow::Owned(self.data_contract()),
                        storage_flags: None,
                    }),
                ])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DataContractUpdateTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    fn make_action() -> DataContractUpdateTransitionAction {
        let platform_version = PlatformVersion::latest();
        let fixture =
            get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let dc = fixture.data_contract_owned();
        DataContractUpdateTransitionAction::V0(DataContractUpdateTransitionActionV0 {
            data_contract: dc,
            identity_contract_nonce: 99,
            user_fee_increase: 0,
        })
    }

    #[test]
    fn test_produces_two_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should produce: UpdateIdentityContractNonce, ApplyContract
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn test_first_op_is_update_identity_contract_nonce() {
        let action = make_action();
        let owner_id = action.data_contract_ref().owner_id().into_buffer();
        let contract_id = action.data_contract_ref().id().into_buffer();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                identity_id,
                contract_id: cid,
                nonce,
            }) => {
                assert_eq!(*identity_id, owner_id);
                assert_eq!(*cid, contract_id);
                assert_eq!(*nonce, 99);
            }
            other => panic!(
                "expected UpdateIdentityContractNonce, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_second_op_is_apply_contract() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            DataContractOperation(DataContractOperationType::ApplyContract {
                storage_flags,
                ..
            }) => {
                assert!(storage_flags.is_none());
            }
            other => panic!("expected ApplyContract, got {:?}", other),
        }
    }
}
