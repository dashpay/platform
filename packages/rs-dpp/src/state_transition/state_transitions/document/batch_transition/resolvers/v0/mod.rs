use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
use crate::state_transition::batch_transition::batched_transition::{
    DocumentPurchaseTransition, DocumentTransferTransition,
};
use crate::state_transition::batch_transition::token_direct_purchase_transition::TokenDirectPurchaseTransition;
use crate::state_transition::batch_transition::{
    DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
    TokenBurnTransition, TokenClaimTransition, TokenConfigUpdateTransition,
    TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition,
    TokenMintTransition, TokenSetPriceForDirectPurchaseTransition, TokenTransferTransition,
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
}
