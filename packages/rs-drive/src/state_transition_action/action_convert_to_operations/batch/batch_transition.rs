use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

impl DriveHighLevelBatchOperationConverter for BatchedTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match self {
            BatchedTransitionAction::DocumentAction(document_action) => document_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            BatchedTransitionAction::TokenAction(token_action) => token_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            BatchedTransitionAction::BumpIdentityDataContractNonce(
                bump_identity_contract_nonce_action,
            ) => bump_identity_contract_nonce_action
                .into_high_level_drive_operations(epoch, platform_version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
        BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0,
    };
    use crate::util::batch::IdentityOperationType;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_bump_identity_data_contract_nonce_batch_delegates_correctly() {
        let nonce_action =
            BumpIdentityDataContractNonceAction::V0(BumpIdentityDataContractNonceActionV0 {
                identity_id: Identifier::from([0xAA; 32]),
                data_contract_id: Identifier::from([0xBB; 32]),
                identity_contract_nonce: 77,
                user_fee_increase: 0,
            });

        let batched = BatchedTransitionAction::BumpIdentityDataContractNonce(nonce_action);
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();
        let owner_id = Identifier::from([0xCC; 32]);

        let ops = batched
            .into_high_level_batch_drive_operations(&epoch, owner_id, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            DriveOperation::IdentityOperation(
                IdentityOperationType::UpdateIdentityContractNonce {
                    identity_id,
                    contract_id,
                    nonce,
                },
            ) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*contract_id, [0xBB; 32]);
                assert_eq!(*nonce, 77);
            }
            other => panic!(
                "expected UpdateIdentityContractNonce, got {:?}",
                other
            ),
        }
    }
}
