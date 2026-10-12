use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
use crate::state_transition::batch_transition::batched_transition::{
    DocumentPurchaseTransition, DocumentTransferTransition,
};
use crate::state_transition::batch_transition::token_direct_purchase_transition::TokenDirectPurchaseTransition;
use crate::state_transition::batch_transition::{
    DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
    TokenBurnFromPoolTransition, TokenBurnTransition, TokenClaimToPoolTransition,
    TokenClaimTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition,
    TokenDirectPurchaseToPoolTransition, TokenEmergencyActionTransition, TokenFreezeTransition,
    TokenMintToPoolTransition, TokenMintTransition, TokenSetPriceForDirectPurchaseTransition,
    TokenShieldTransition, TokenShieldedTransferTransition, TokenTransferTransition,
    TokenUnshieldTransition,
};

pub trait BatchTransitionResolversV0 {
    fn as_transition_create(&self) -> Option<&DocumentCreateTransition>;
    fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition>;
    fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition>;
    fn as_transition_transfer(&self) -> Option<&DocumentTransferTransition>;
    fn as_transition_purchase(&self) -> Option<&DocumentPurchaseTransition>;
    fn as_transition_token_burn(&self) -> Option<&TokenBurnTransition>;
    fn as_transition_token_mint(&self) -> Option<&TokenMintTransition>;
    fn as_transition_token_transfer(&self) -> Option<&TokenTransferTransition>;
    fn as_transition_token_freeze(&self) -> Option<&TokenFreezeTransition>;
    fn as_transition_token_unfreeze(&self) -> Option<&TokenUnfreezeTransition>;
    fn as_transition_token_destroy_frozen_funds(
        &self,
    ) -> Option<&TokenDestroyFrozenFundsTransition>;

    fn as_transition_token_claim(&self) -> Option<&TokenClaimTransition>;
    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition>;

    fn as_transition_token_config_update(&self) -> Option<&TokenConfigUpdateTransition>;
    fn as_transition_token_direct_purchase(&self) -> Option<&TokenDirectPurchaseTransition>;
    fn as_transition_token_set_price_for_direct_purchase(
        &self,
    ) -> Option<&TokenSetPriceForDirectPurchaseTransition>;
    fn as_transition_token_shield(&self) -> Option<&TokenShieldTransition>;
    fn as_transition_token_unshield(&self) -> Option<&TokenUnshieldTransition>;
    fn as_transition_token_shielded_transfer(&self) -> Option<&TokenShieldedTransferTransition>;
    fn as_transition_token_mint_to_pool(&self) -> Option<&TokenMintToPoolTransition>;
    fn as_transition_token_burn_from_pool(&self) -> Option<&TokenBurnFromPoolTransition>;
    fn as_transition_token_claim_to_pool(&self) -> Option<&TokenClaimToPoolTransition>;
    fn as_transition_token_direct_purchase_to_pool(
        &self,
    ) -> Option<&TokenDirectPurchaseToPoolTransition>;
}
