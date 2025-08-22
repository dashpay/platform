use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::util::batch::DriveOperation;
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

impl DriveHighLevelBatchOperationConverter for TokenTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
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
        }
    }
}

impl TokenTransitionAction {
    /// Gets the associated token event for the transition action
    pub fn associated_token_event(&self) -> TokenEvent {
        match self {
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
                    destroy_action.amount(),
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
        }
    }
}
