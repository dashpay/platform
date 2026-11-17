use dpp::block::epoch::Epoch;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::v0::TokenTransferTransitionActionAccessorsV0;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};

impl DriveHighLevelBatchOperationConverter for TokenTransferTransitionAction {
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
            .token_transfer_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();

                let identity_contract_nonce = self.base().identity_contract_nonce();

                let token_id = self.token_id();

                let recipient_id = self.recipient_id();

                let amount = self.amount();

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                // A random transfer (protocol version 15) moves the most it moves from the
                // sender into a reservation and pays the recipient once the block's random
                // beacon is known (`Drive::settle_random_token_amounts`), and its history holds
                // the range's minimum until then. Only a version 1 transfer carries a random minimum amount,
                // and no protocol version before 15 decodes one
                // (`StateTransition::active_version_range`), so every earlier version that
                // selects this generation takes the other branch, as it always did.
                let history_amount = match self.random_minimum_amount() {
                    Some(minimum) => {
                        ops.push(TokenOperation(
                            TokenOperationType::TokenReserveRandomTransfer {
                                token_id,
                                sender_id: owner_id,
                                recipient_id,
                                identity_contract_nonce,
                                minimum,
                                maximum: amount,
                            },
                        ));
                        minimum
                    }
                    None => {
                        ops.push(TokenOperation(TokenOperationType::TokenTransfer {
                            token_id,
                            sender_id: owner_id,
                            recipient_id,
                            amount,
                        }));
                        amount
                    }
                };

                let random_minimum_amount = self.random_minimum_amount();
                let token_configuration = self.base().token_configuration()?;
                if token_configuration.keeps_history().keeps_transfer_history() {
                    let (public_note, shared_encrypted_note, private_encrypted_note) =
                        self.notes_owned();
                    let event = TokenEvent::Transfer(
                        recipient_id,
                        public_note,
                        shared_encrypted_note,
                        private_encrypted_note,
                        history_amount,
                    );
                    // A random transfer's history document id also hashes its range, so a
                    // proof of the document binds the range the amount was drawn from
                    ops.push(TokenOperation(match random_minimum_amount {
                        Some(minimum) => TokenOperationType::TokenRandomAmountHistory {
                            token_id,
                            owner_id,
                            nonce: identity_contract_nonce,
                            event,
                            minimum,
                            maximum: amount,
                        },
                        None => TokenOperationType::TokenHistory {
                            token_id,
                            owner_id,
                            nonce: identity_contract_nonce,
                            event,
                        },
                    }));
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenTransferTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
