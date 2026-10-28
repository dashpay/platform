use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use dpp::tokens::token_order_side::TokenOrderSide;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{FrozenListings, TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::{GroupOperationType, TokenOperationType};
use crate::util::batch::DriveOperation::{DocumentOperation, GroupOperation, IdentityOperation, TokenOperation};
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};

impl TokenDestroyFrozenFundsTransitionAction {
    /// Generation 1 (protocol version 15): generation 0's operations, and with them the
    /// destruction of the frozen identity's open listings the transformer read. Without listings
    /// it is generation 0. With them, one `TokenDestroyFrozenHolding` burns the balance, empties
    /// each listing's escrow and lowers the supply by everything destroyed; a `DeleteDocument`
    /// per listing deletes its `ask` document, each built against the batch's operations before
    /// it (`apply_drive_operations` 2), so index trees the listings share are removed and
    /// refunded; the history and group-action events record everything destroyed.
    pub(super) fn into_high_level_batch_drive_operations_v1<'b>(
        self,
        owner_id: Identifier,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let Some(FrozenListings {
            listings,
            token_market_contract,
        }) = self.frozen_listings().cloned()
        else {
            return self.into_high_level_batch_drive_operations_v0(owner_id);
        };

        let data_contract_id = self.base().data_contract_id();

        let identity_contract_nonce = self.base().identity_contract_nonce();

        let destroyed_amount = self.destroyed_amount()?;

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
                destroyed_amount,
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
            ops.push(TokenOperation(
                TokenOperationType::TokenDestroyFrozenHolding {
                    token_id: self.token_id(),
                    holder_id: self.frozen_identity_id(),
                    balance_amount: self.amount(),
                    order_ids: listings.iter().map(|listing| listing.id).collect(),
                    destroyed_amount,
                },
            ));
            for listing in &listings {
                ops.push(DocumentOperation(DocumentOperationType::DeleteDocument {
                    document_id: listing.id,
                    contract_info: DataContractInfo::DataContractFetchInfo(
                        token_market_contract.clone(),
                    ),
                    document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(
                        TokenOrderSide::Ask.document_type_name(),
                    ),
                }));
            }

            ops.push(TokenOperation(TokenOperationType::TokenHistory {
                token_id: self.token_id(),
                owner_id,
                nonce: identity_contract_nonce,
                event: TokenEvent::DestroyFrozenFunds(
                    self.frozen_identity_id(),
                    destroyed_amount,
                    self.public_note_owned(),
                ),
            }));
        }

        Ok(ops)
    }
}
