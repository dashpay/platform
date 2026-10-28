use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::{GroupOperationType, TokenOperationType};
use crate::util::batch::DriveOperation::{GroupOperation, IdentityOperation, TokenOperation};

impl TokenDestroyFrozenFundsTransitionAction {
    /// Generation 0 (protocol version 14 and earlier): bumps the signer's nonce, records the
    /// signature of a group action, and when the action is performed burns the frozen balance
    /// and records the destroy in the token's history.
    pub(super) fn into_high_level_batch_drive_operations_v0<'b>(
        self,
        owner_id: Identifier,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let data_contract_id = self.base().data_contract_id();

        let identity_contract_nonce = self.base().identity_contract_nonce();

        let mut ops = vec![IdentityOperation(
            IdentityOperationType::UpdateIdentityContractNonce {
                identity_id: owner_id.into_buffer(),
                contract_id: data_contract_id.into_buffer(),
                nonce: identity_contract_nonce,
            },
        )];

        if let Some(GroupStateTransitionResolvedInfo {
            group_contract_position,
            action_id,
            action_is_proposer,
            signer_power,
            ..
        }) = self.base().store_in_group()
        {
            let event = TokenEvent::DestroyFrozenFunds(
                self.frozen_identity_id(),
                self.amount(),
                self.public_note().cloned(),
            );

            let initialize_with_insert_action_info = if *action_is_proposer {
                Some(GroupAction::V0(GroupActionV0 {
                    contract_id: self.base().data_contract_id(),
                    proposer_id: owner_id,
                    token_contract_position: self.base().token_position(),
                    event: GroupActionEvent::TokenEvent(event),
                }))
            } else {
                None
            };

            ops.push(GroupOperation(GroupOperationType::AddGroupAction {
                contract_id: data_contract_id,
                group_contract_position: *group_contract_position,
                initialize_with_insert_action_info,
                action_id: *action_id,
                signer_identity_id: owner_id,
                signer_power: *signer_power,
                closes_group_action: self.base().perform_action(),
            }));
        }

        if self.base().perform_action() {
            ops.push(TokenOperation(TokenOperationType::TokenBurn {
                token_id: self.token_id(),
                identity_balance_holder_id: self.frozen_identity_id(),
                burn_amount: self.amount(),
            }));

            ops.push(TokenOperation(TokenOperationType::TokenHistory {
                token_id: self.token_id(),
                owner_id,
                nonce: identity_contract_nonce,
                event: TokenEvent::DestroyFrozenFunds(
                    self.frozen_identity_id(),
                    self.amount(),
                    self.public_note_owned(),
                ),
            }));
        }

        Ok(ops)
    }
}
