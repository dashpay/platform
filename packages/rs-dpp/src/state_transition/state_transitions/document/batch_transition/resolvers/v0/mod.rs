use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
use crate::state_transition::batch_transition::batched_transition::{
    DocumentPurchaseTransition, DocumentTransferTransition,
};
use crate::state_transition::batch_transition::{
    DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
    TokenBurnTransition, TokenFreezeTransition, TokenMintTransition, TokenTransferTransition,
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
}
