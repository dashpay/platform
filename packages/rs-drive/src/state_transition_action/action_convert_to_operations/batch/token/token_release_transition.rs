use dpp::block::epoch::Epoch;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionResolvedRecipient;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_release_transition_action::{TokenReleaseTransitionAction, TokenReleaseTransitionActionAccessorsV0};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};

impl DriveHighLevelBatchOperationConverter for TokenReleaseTransitionAction {
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
            .token_release_transition
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

                match self.recipient() {
                    TokenDistributionResolvedRecipient::Identity(identity) => {
                        ops.push(TokenOperation(TokenOperationType::TokenMint {
                            token_id: self.token_id(),
                            identity_balance_holder_id: *identity,
                            mint_amount: self.amount(),
                            allow_first_mint: false,
                        }));
                    }
                    TokenDistributionResolvedRecipient::ResolvedEvonodesByParticipation(
                        weighted_identities,
                    ) => {
                        ops.push(TokenOperation(TokenOperationType::TokenMintMany {
                            token_id: self.token_id(),
                            mint_amount: self.amount(),
                            recipients: weighted_identities.clone(),
                            allow_first_mint: false,
                        }));
                    }
                }

                match self.distribution_type() {
                    TokenDistributionType::PreProgrammed => {
                        ops.push(TokenOperation(
                            TokenOperationType::TokenMarkPreProgrammedReleaseAsDistributed {
                                token_id: self.token_id(),
                                identity_id: Default::default(),
                                release_time: 0,
                            },
                        ));
                    }
                    TokenDistributionType::Perpetual => {}
                }

                let token_configuration = self.base().token_configuration()?;
                ops.push(TokenOperation(TokenOperationType::TokenHistory {
                    token_id: self.token_id(),
                    owner_id,
                    nonce: identity_contract_nonce,
                    event: TokenEvent::Release(
                        self.recipient().clone(),
                        self.distribution_type(),
                        self.amount(),
                        self.public_note_owned(),
                    ),
                }));

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenReleaseTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
