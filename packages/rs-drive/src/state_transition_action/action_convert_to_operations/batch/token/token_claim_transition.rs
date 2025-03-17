use dpp::block::epoch::Epoch;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionResolvedRecipient;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::{TokenClaimTransitionAction, TokenClaimTransitionActionAccessorsV0};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};

impl DriveHighLevelBatchOperationConverter for TokenClaimTransitionAction {
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
            .token_claim_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();

                let identity_contract_nonce = self.base().identity_contract_nonce();

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                match self.distribution_info() {
                    TokenDistributionInfo::Perpetual(
                        _,
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(identity),
                    )
                    | TokenDistributionInfo::PreProgrammed(_, identity)
                    | TokenDistributionInfo::Perpetual(
                        _,
                        TokenDistributionResolvedRecipient::Identity(identity),
                    )
                    | TokenDistributionInfo::Perpetual(
                        _,
                        TokenDistributionResolvedRecipient::Evonode(identity),
                    ) => {
                        ops.push(TokenOperation(TokenOperationType::TokenMint {
                            token_id: self.token_id(),
                            identity_balance_holder_id: *identity,
                            mint_amount: self.amount(),
                            allow_first_mint: false,
                            allow_saturation: true,
                        }));
                    }
                }

                match self.distribution_info() {
                    TokenDistributionInfo::PreProgrammed(release_time, recipient) => {
                        ops.push(TokenOperation(
                            TokenOperationType::TokenMarkPreProgrammedReleaseAsDistributed {
                                token_id: self.token_id(),
                                recipient_id: *recipient,
                                release_time: *release_time,
                            },
                        ));
                    }
                    TokenDistributionInfo::Perpetual(claim_moment, _) => {
                        ops.push(TokenOperation(
                            TokenOperationType::TokenMarkPerpetualReleaseAsDistributed {
                                token_id: self.token_id(),
                                recipient_id: owner_id,
                                cycle_start_moment: *claim_moment,
                            },
                        ));
                    }
                }
                ops.push(TokenOperation(TokenOperationType::TokenHistory {
                    token_id: self.token_id(),
                    owner_id,
                    nonce: identity_contract_nonce,
                    event: TokenEvent::Claim(
                        self.distribution_info().into(),
                        self.amount(),
                        self.public_note_owned(),
                    ),
                }));

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenClaimTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
