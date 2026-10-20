use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::contract::contract_fee_claim::v0::ContractFeeClaimTransitionActionV0;
use crate::state_transition_action::contract::contract_fee_claim::ContractFeeClaimTransitionAction;
use crate::util::batch::DriveOperation::{
    ContractFeePotOperation, ContractModerationOperation, IdentityOperation,
};
use crate::util::batch::{
    ContractFeePotOperationType, ContractModerationOperationType, DriveOperation,
    IdentityOperationType,
};
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ContractFeeClaimTransitionAction {
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
            .contract_fee_claim_transition
        {
            0 => {
                let last_claim = self.last_claim();
                let ContractFeeClaimTransitionAction::V0(ContractFeeClaimTransitionActionV0 {
                    claimant_id,
                    data_contract_id: contract_id,
                    identity_contract_nonce,
                    pot,
                    payouts,
                    settled_action_counts,
                    ..
                }) = self;

                // What leaves the pot is what reaches the balances: the credits only move.
                let paid_out = payouts.values().try_fold(0 as Credits, |total, amount| {
                    total
                        .checked_add(*amount)
                        .ok_or(Error::Fee(FeeError::Overflow(
                            "the payouts of a contract fee claim overflow credits",
                        )))
                })?;

                let mut operations = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: claimant_id.to_buffer(),
                        contract_id: contract_id.to_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    ContractFeePotOperation(ContractFeePotOperationType::DeductFromPot {
                        contract_id,
                        pot,
                        amount: paid_out,
                    }),
                ];
                operations.extend(payouts.into_iter().map(|(identity_id, added_balance)| {
                    IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        added_balance,
                    })
                }));
                // A seated moderation team's pot was split by the action counts, which start
                // over.
                if !settled_action_counts.is_empty() {
                    operations.push(ContractModerationOperation(
                        ContractModerationOperationType::RemoveActionCounts {
                            contract_id,
                            identity_ids: settled_action_counts,
                        },
                    ));
                }
                operations.push(ContractFeePotOperation(
                    ContractFeePotOperationType::SetLastClaim {
                        contract_id,
                        pot,
                        last_claim,
                    },
                ));
                Ok(operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ContractFeeClaimTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
