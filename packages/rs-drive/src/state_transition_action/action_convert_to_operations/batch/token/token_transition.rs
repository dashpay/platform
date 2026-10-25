use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::tokens::token_event::TokenEvent;
use dpp::version::PlatformVersion;
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::TokenBurnTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::TokenConfigUpdateTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::TokenFreezeTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::TokenMintTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::TokenClaimTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::TokenDirectPurchaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::TokenUnfreezeTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_shield_transition_action::TokenShieldTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_unshield_transition_action::TokenUnshieldTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_to_pool_transition_action::TokenMintToPoolTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_from_pool_transition_action::TokenBurnFromPoolTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_to_pool_transition_action::TokenClaimToPoolTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_to_pool_transition_action::TokenDirectPurchaseToPoolTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_ask_transition_action::TokenPlaceAskTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_fill_ask_transition_action::TokenFillAskTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_cancel_order_transition_action::TokenCancelOrderTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_bid_transition_action::TokenPlaceBidTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_fill_bid_transition_action::TokenFillBidTransitionActionAccessorsV0;
use dpp::tokens::token_order_side::TokenOrderSide;
use dpp::shielded::serialized_actions_digest;

impl DriveHighLevelBatchOperationConverter for TokenTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        // The token marketplace's history, worked out before the action is consumed and appended
        // after its own operations.
        let marketplace_history = match &self {
            TokenTransitionAction::PlaceAskAction(_)
            | TokenTransitionAction::FillAskAction(_)
            | TokenTransitionAction::CancelOrderAction(_)
            | TokenTransitionAction::PlaceBidAction(_)
            | TokenTransitionAction::FillBidAction(_) => {
                self.token_history_operation(owner_id, platform_version)?
            }
            _ => None,
        };
        match self {
            TokenTransitionAction::BurnAction(token_burn_transition) => token_burn_transition
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::MintAction(token_mint_transition) => token_mint_transition
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::TransferAction(token_transfer_transition) => {
                token_transfer_transition.into_high_level_batch_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            TokenTransitionAction::FreezeAction(token_freeze_action) => token_freeze_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::UnfreezeAction(token_unfreeze_action) => token_unfreeze_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::ClaimAction(token_claim) => token_claim
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::EmergencyActionAction(token_emergency_action) => {
                token_emergency_action.into_high_level_batch_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            TokenTransitionAction::DestroyFrozenFundsAction(token_destroy_frozen_funds) => {
                token_destroy_frozen_funds.into_high_level_batch_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            TokenTransitionAction::ConfigUpdateAction(token_config_update) => token_config_update
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::DirectPurchaseAction(direct_purchase) => direct_purchase
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::SetPriceForDirectPurchaseAction(set_price) => {
                set_price.into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
            }
            TokenTransitionAction::ShieldAction(shield) => {
                shield.into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
            }
            TokenTransitionAction::UnshieldAction(unshield) => {
                unshield.into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
            }
            TokenTransitionAction::ShieldedTransferAction(shielded_transfer) => shielded_transfer
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::MintToPoolAction(mint_to_pool) => mint_to_pool
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::BurnFromPoolAction(burn_from_pool) => burn_from_pool
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::ClaimToPoolAction(claim_to_pool) => claim_to_pool
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::DirectPurchaseToPoolAction(direct_purchase_to_pool) => {
                direct_purchase_to_pool.into_high_level_batch_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            TokenTransitionAction::PlaceAskAction(place_ask) => place_ask
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
                .map(|operations| with_history(operations, marketplace_history)),
            TokenTransitionAction::FillAskAction(fill_ask) => fill_ask
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
                .map(|operations| with_history(operations, marketplace_history)),
            TokenTransitionAction::CancelOrderAction(cancel_order) => cancel_order
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
                .map(|operations| with_history(operations, marketplace_history)),
            TokenTransitionAction::PlaceBidAction(place_bid) => place_bid
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
                .map(|operations| with_history(operations, marketplace_history)),
            TokenTransitionAction::FillBidAction(fill_bid) => fill_bid
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version)
                .map(|operations| with_history(operations, marketplace_history)),
        }
    }
}

/// `operations` followed by the history operation, when there is one.
fn with_history<'b>(
    mut operations: Vec<DriveOperation<'b>>,
    history: Option<DriveOperation<'b>>,
) -> Vec<DriveOperation<'b>> {
    operations.extend(history);
    operations
}

impl TokenTransitionAction {
    /// The operation recording this transition in the token history contract when its token
    /// keeps the history of its kind (`keeps_history`): the event `associated_token_event`
    /// describes, owned by the signer and keyed by the signer's identity contract nonce.
    ///
    /// The record's id is derived from the token, the signer, the event's kind and that nonce,
    /// not from the order. Two marketplace transitions of one kind by one signer under one nonce
    /// would share it, which only a batch of more than one transition could produce: sound while
    /// a batch carries one (`SystemLimits::max_transitions_in_documents_batch`).
    fn token_history_operation<'b>(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<DriveOperation<'b>>, Error> {
        if !self.keeps_history()? {
            return Ok(None);
        }
        Ok(Some(DriveOperation::TokenOperation(
            TokenOperationType::TokenHistory {
                token_id: self.base().token_id(),
                owner_id,
                nonce: self.base().identity_contract_nonce(),
                event: self.associated_token_event(platform_version)?,
            },
        )))
    }

    /// Gets the associated token event for the transition action: the event its conversion at
    /// `platform_version` records in the token history
    pub fn associated_token_event(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<TokenEvent, Error> {
        Ok(match self {
            TokenTransitionAction::BurnAction(burn_action) => TokenEvent::Burn(
                burn_action.burn_amount(),
                burn_action.burn_from_identifier(),
                burn_action.public_note().cloned(),
            ),
            TokenTransitionAction::MintAction(mint_action) => TokenEvent::Mint(
                mint_action.mint_amount(),
                mint_action.identity_balance_holder_id(),
                mint_action.public_note().cloned(),
            ),
            TokenTransitionAction::TransferAction(transfer_action) => {
                let (public_note, shared_encrypted_note, private_encrypted_note) =
                    transfer_action.notes();
                TokenEvent::Transfer(
                    transfer_action.recipient_id(),
                    public_note,
                    shared_encrypted_note,
                    private_encrypted_note,
                    transfer_action.amount(),
                )
            }
            TokenTransitionAction::FreezeAction(freeze_action) => TokenEvent::Freeze(
                freeze_action.identity_to_freeze_id(),
                freeze_action.public_note().cloned(),
            ),
            TokenTransitionAction::UnfreezeAction(unfreeze_action) => TokenEvent::Unfreeze(
                unfreeze_action.frozen_identity_id(),
                unfreeze_action.public_note().cloned(),
            ),
            TokenTransitionAction::ClaimAction(release_action) => TokenEvent::Claim(
                release_action.distribution_info().into(),
                release_action.amount(),
                release_action.public_note().cloned(),
            ),
            TokenTransitionAction::EmergencyActionAction(emergency_action) => {
                TokenEvent::EmergencyAction(
                    emergency_action.emergency_action(),
                    emergency_action.public_note().cloned(),
                )
            }
            TokenTransitionAction::DestroyFrozenFundsAction(destroy_action) => {
                TokenEvent::DestroyFrozenFunds(
                    destroy_action.frozen_identity_id(),
                    destroy_action.destroyed_amount(platform_version)?,
                    destroy_action.public_note().cloned(),
                )
            }
            TokenTransitionAction::ConfigUpdateAction(config_update) => TokenEvent::ConfigUpdate(
                config_update.update_token_configuration_item().clone(),
                config_update.public_note().cloned(),
            ),
            TokenTransitionAction::DirectPurchaseAction(purchase_action) => {
                TokenEvent::DirectPurchase(
                    purchase_action.token_count(),
                    purchase_action.total_agreed_price(),
                )
            }
            TokenTransitionAction::SetPriceForDirectPurchaseAction(set_price_action) => {
                TokenEvent::ChangePriceForDirectPurchase(
                    set_price_action.price().cloned(),
                    set_price_action.public_note().cloned(),
                )
            }
            TokenTransitionAction::ShieldAction(shield_action) => {
                TokenEvent::Shield(shield_action.amount())
            }
            TokenTransitionAction::UnshieldAction(unshield_action) => {
                TokenEvent::Unshield(unshield_action.recipient_id(), unshield_action.amount())
            }
            TokenTransitionAction::ShieldedTransferAction(_) => TokenEvent::ShieldedTransfer,
            TokenTransitionAction::MintToPoolAction(action) => TokenEvent::MintToPool(
                action.amount(),
                serialized_actions_digest(action.actions()).into(),
                action.public_note().cloned(),
            ),
            TokenTransitionAction::BurnFromPoolAction(action) => TokenEvent::BurnFromPool(
                action.amount(),
                serialized_actions_digest(action.actions()).into(),
                action.public_note().cloned(),
            ),
            TokenTransitionAction::ClaimToPoolAction(action) => {
                TokenEvent::ClaimToPool(action.amount())
            }
            TokenTransitionAction::DirectPurchaseToPoolAction(action) => {
                TokenEvent::DirectPurchaseToPool(action.token_count(), action.total_agreed_price())
            }
            TokenTransitionAction::PlaceAskAction(action) => TokenEvent::OrderPlaced(
                action.order_id(),
                TokenOrderSide::Ask,
                action.amount(),
                action.price(),
                action.min_fill(),
                action.decimals(),
            ),
            TokenTransitionAction::FillAskAction(action) => TokenEvent::OrderFilled(
                action.order().id,
                TokenOrderSide::Ask,
                action.order().maker_id,
                action.fill_amount(),
                action.credits(),
                action.order().decimals,
            ),
            TokenTransitionAction::CancelOrderAction(action) => {
                TokenEvent::OrderCancelled(action.order().id, action.order().side)
            }
            TokenTransitionAction::PlaceBidAction(action) => TokenEvent::OrderPlaced(
                action.order_id(),
                TokenOrderSide::Bid,
                action.amount(),
                action.price(),
                action.min_fill(),
                action.decimals(),
            ),
            TokenTransitionAction::FillBidAction(action) => TokenEvent::OrderFilled(
                action.order().id,
                TokenOrderSide::Bid,
                action.order().maker_id,
                action.fill_amount(),
                action.credits(),
                action.order().decimals,
            ),
        })
    }
}
