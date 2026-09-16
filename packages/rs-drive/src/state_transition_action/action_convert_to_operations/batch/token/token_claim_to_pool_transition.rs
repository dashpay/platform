use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_to_pool_transition_action::{TokenClaimToPoolTransitionAction, TokenClaimToPoolTransitionActionAccessorsV0};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;

impl DriveHighLevelBatchOperationConverter for TokenClaimToPoolTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .token_claim_to_pool_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let token_id = self.token_id();

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                ops.push(TokenOperation(TokenOperationType::TokenMintToPool {
                    token_id,
                    amount: self.amount(),
                    allow_first_mint: false,
                    notes: self.notes(),
                }));

                match self.distribution_info() {
                    TokenDistributionInfo::PreProgrammed(release_time, recipient) => {
                        ops.push(TokenOperation(
                            TokenOperationType::TokenMarkPreProgrammedReleaseAsDistributed {
                                token_id,
                                recipient_id: *recipient,
                                release_time: *release_time,
                            },
                        ));
                    }
                    TokenDistributionInfo::Perpetual(claim_moment, _) => {
                        ops.push(TokenOperation(
                            TokenOperationType::TokenMarkPerpetualReleaseAsDistributed {
                                token_id,
                                recipient_id: owner_id,
                                cycle_start_moment: *claim_moment,
                            },
                        ));
                    }
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenClaimToPoolTransitionAction::into_high_level_batch_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
