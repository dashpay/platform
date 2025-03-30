use crate::state_transition::batch_transition::batched_transition::{
    BatchedTransition, BatchedTransitionRef, DocumentPurchaseTransition, DocumentTransferTransition,
};
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::batch_transition::{
    DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
    TokenBurnTransition, TokenClaimTransition, TokenConfigUpdateTransition,
    TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition,
    TokenMintTransition, TokenTransferTransition, TokenUnfreezeTransition,
};
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::transition::TokenOrderAdjustPriceTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::transition::TokenOrderBuyLimitTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_cancel_transition::transition::TokenOrderCancelTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::transition::TokenOrderSellLimitTransition;

impl BatchTransitionResolversV0 for BatchedTransition {
    fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        match self {
            BatchedTransition::Document(document) => document.as_transition_create(),
            BatchedTransition::Token(_) => None,
        }
    }

    fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        match self {
            BatchedTransition::Document(document) => document.as_transition_replace(),
            BatchedTransition::Token(_) => None,
        }
    }

    fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        match self {
            BatchedTransition::Document(document) => document.as_transition_delete(),
            BatchedTransition::Token(_) => None,
        }
    }

    fn as_transition_transfer(&self) -> Option<&DocumentTransferTransition> {
        match self {
            BatchedTransition::Document(document) => document.as_transition_transfer(),
            BatchedTransition::Token(_) => None,
        }
    }

    fn as_transition_purchase(&self) -> Option<&DocumentPurchaseTransition> {
        match self {
            BatchedTransition::Document(document) => document.as_transition_purchase(),
            BatchedTransition::Token(_) => None,
        }
    }

    fn as_transition_token_burn(&self) -> Option<&TokenBurnTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_burn(),
        }
    }

    fn as_transition_token_mint(&self) -> Option<&TokenMintTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_mint(),
        }
    }

    fn as_transition_token_transfer(&self) -> Option<&TokenTransferTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_transfer(),
        }
    }

    fn as_transition_token_freeze(&self) -> Option<&TokenFreezeTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_freeze(),
        }
    }

    fn as_transition_token_unfreeze(&self) -> Option<&TokenUnfreezeTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_unfreeze(),
        }
    }

    fn as_transition_token_destroy_frozen_funds(
        &self,
    ) -> Option<&TokenDestroyFrozenFundsTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_destroy_frozen_funds(),
        }
    }

    fn as_transition_token_claim(&self) -> Option<&TokenClaimTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_claim(),
        }
    }

    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_emergency_action(),
        }
    }

    fn as_transition_token_config_update(&self) -> Option<&TokenConfigUpdateTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_config_update(),
        }
    }

    fn as_transition_token_order_buy_limit(&self) -> Option<&TokenOrderBuyLimitTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_order_buy_limit(),
        }
    }

    fn as_transition_token_order_sell_limit(&self) -> Option<&TokenOrderSellLimitTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_order_sell_limit(),
        }
    }

    fn as_transition_token_order_cancel(&self) -> Option<&TokenOrderCancelTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_order_cancel(),
        }
    }

    fn as_transition_token_order_adjust_price(&self) -> Option<&TokenOrderAdjustPriceTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_order_adjust_price(),
        }
    }
}

impl BatchTransitionResolversV0 for BatchedTransitionRef<'_> {
    fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        match self {
            BatchedTransitionRef::Document(document) => document.as_transition_create(),
            BatchedTransitionRef::Token(_) => None,
        }
    }

    fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        match self {
            BatchedTransitionRef::Document(document) => document.as_transition_replace(),
            BatchedTransitionRef::Token(_) => None,
        }
    }

    fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        match self {
            BatchedTransitionRef::Document(document) => document.as_transition_delete(),
            BatchedTransitionRef::Token(_) => None,
        }
    }

    fn as_transition_transfer(&self) -> Option<&DocumentTransferTransition> {
        match self {
            BatchedTransitionRef::Document(document) => document.as_transition_transfer(),
            BatchedTransitionRef::Token(_) => None,
        }
    }

    fn as_transition_purchase(&self) -> Option<&DocumentPurchaseTransition> {
        match self {
            BatchedTransitionRef::Document(document) => document.as_transition_purchase(),
            BatchedTransitionRef::Token(_) => None,
        }
    }

    fn as_transition_token_burn(&self) -> Option<&TokenBurnTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_burn(),
        }
    }

    fn as_transition_token_mint(&self) -> Option<&TokenMintTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_mint(),
        }
    }

    fn as_transition_token_transfer(&self) -> Option<&TokenTransferTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_transfer(),
        }
    }

    fn as_transition_token_freeze(&self) -> Option<&TokenFreezeTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_freeze(),
        }
    }

    fn as_transition_token_unfreeze(&self) -> Option<&TokenUnfreezeTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_unfreeze(),
        }
    }

    fn as_transition_token_destroy_frozen_funds(
        &self,
    ) -> Option<&TokenDestroyFrozenFundsTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_destroy_frozen_funds(),
        }
    }

    fn as_transition_token_claim(&self) -> Option<&TokenClaimTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_claim(),
        }
    }

    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_emergency_action(),
        }
    }

    fn as_transition_token_config_update(&self) -> Option<&TokenConfigUpdateTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_config_update(),
        }
    }

    fn as_transition_token_order_buy_limit(&self) -> Option<&TokenOrderBuyLimitTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_order_buy_limit(),
        }
    }

    fn as_transition_token_order_sell_limit(&self) -> Option<&TokenOrderSellLimitTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_order_sell_limit(),
        }
    }

    fn as_transition_token_order_cancel(&self) -> Option<&TokenOrderCancelTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_order_cancel(),
        }
    }

    fn as_transition_token_order_adjust_price(&self) -> Option<&TokenOrderAdjustPriceTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_order_adjust_price(),
        }
    }
}
