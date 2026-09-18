use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::batch::v1::{
    BatchTransitionActionV1, BatchedTransitionActionV1,
};
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

impl DriveHighLevelBatchOperationConverter for BatchedTransitionActionV1 {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match self {
            BatchedTransitionActionV1::Batched(batched_action) => batched_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            BatchedTransitionActionV1::DocumentErase(erase_action) => erase_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
        }
    }
}

/// Generation 1 of the batch conversion: consumes batch action format 1, whose
/// items may include the erase of a keep-history document, and emits the
/// operations of every item in the order the batch listed them.
impl DriveHighLevelOperationConverter for BatchTransitionActionV1 {
    fn into_high_level_drive_operations<'b>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .documents_batch_transition
        {
            1 => {
                let owner_id = self.owner_id();
                let transitions = self.transitions_owned();
                Ok(transitions
                    .into_iter()
                    .map(|transition| {
                        transition.into_high_level_batch_drive_operations(
                            epoch,
                            owner_id,
                            platform_version,
                        )
                    })
                    .collect::<Result<Vec<Vec<DriveOperation>>, Error>>()?
                    .into_iter()
                    .flatten()
                    .collect())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "BatchTransitionActionV1::into_high_level_drive_operations".to_string(),
                known_versions: vec![1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
    use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
        BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0,
    };
    use crate::util::batch::IdentityOperationType;

    fn bump(nonce: u64) -> BatchedTransitionActionV1 {
        BatchedTransitionActionV1::Batched(BatchedTransitionAction::BumpIdentityDataContractNonce(
            BumpIdentityDataContractNonceAction::V0(BumpIdentityDataContractNonceActionV0 {
                identity_id: Identifier::from([0xAA; 32]),
                data_contract_id: Identifier::from([0xBB; 32]),
                identity_contract_nonce: nonce,
                user_fee_increase: 0,
            }),
        ))
    }

    #[test]
    fn should_emit_the_operations_of_every_item_in_batch_order() {
        let action = BatchTransitionActionV1 {
            owner_id: Identifier::from([0xCC; 32]),
            transitions: vec![bump(7), bump(8)],
            user_fee_increase: 0,
        };
        let epoch = Epoch::new(0).expect("epoch");

        let operations = action
            .into_high_level_drive_operations(&epoch, PlatformVersion::latest())
            .expect("operations");

        let nonces: Vec<u64> = operations
            .iter()
            .map(|operation| match operation {
                DriveOperation::IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce { nonce, .. },
                ) => *nonce,
                other => panic!("expected a nonce update, got {other:?}"),
            })
            .collect();
        assert_eq!(nonces, vec![7, 8]);
    }

    #[test]
    fn should_refuse_a_protocol_version_whose_converter_only_knows_format_0() {
        let action = BatchTransitionActionV1 {
            owner_id: Identifier::from([0xCC; 32]),
            transitions: vec![bump(7)],
            user_fee_increase: 0,
        };
        let epoch = Epoch::new(0).expect("epoch");

        let result = action.into_high_level_drive_operations(
            &epoch,
            PlatformVersion::get(13).expect("protocol 13"),
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnknownVersionMismatch {
                received: 0,
                ..
            }))
        ));
    }
}
