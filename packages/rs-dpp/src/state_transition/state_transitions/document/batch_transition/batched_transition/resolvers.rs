use crate::state_transition::batch_transition::batched_transition::{
    BatchedTransition, BatchedTransitionRef, DocumentPurchaseTransition, DocumentTransferTransition,
};
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::batch_transition::{
    DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
    TokenBurnTransition, TokenClaimTransition, TokenConfigUpdateTransition,
    TokenDestroyFrozenFundsTransition, TokenDirectPurchaseTransition,
    TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition,
    TokenSetPriceForDirectPurchaseTransition, TokenTransferTransition, TokenUnfreezeTransition,
};

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

    fn as_transition_token_direct_purchase(&self) -> Option<&TokenDirectPurchaseTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => token.as_transition_token_direct_purchase(),
        }
    }

    fn as_transition_token_set_price_for_direct_purchase(
        &self,
    ) -> Option<&TokenSetPriceForDirectPurchaseTransition> {
        match self {
            BatchedTransition::Document(_) => None,
            BatchedTransition::Token(token) => {
                token.as_transition_token_set_price_for_direct_purchase()
            }
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

    fn as_transition_token_direct_purchase(&self) -> Option<&TokenDirectPurchaseTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => token.as_transition_token_direct_purchase(),
        }
    }

    fn as_transition_token_set_price_for_direct_purchase(
        &self,
    ) -> Option<&TokenSetPriceForDirectPurchaseTransition> {
        match self {
            BatchedTransitionRef::Document(_) => None,
            BatchedTransitionRef::Token(token) => {
                token.as_transition_token_set_price_for_direct_purchase()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use crate::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_burn_transition::v0::TokenBurnTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_claim_transition::v0::TokenClaimTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_config_update_transition::v0::TokenConfigUpdateTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_destroy_frozen_funds_transition::v0::TokenDestroyFrozenFundsTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_emergency_action_transition::v0::TokenEmergencyActionTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_freeze_transition::v0::TokenFreezeTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_mint_transition::v0::TokenMintTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::v0::TokenSetPriceForDirectPurchaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
    use crate::state_transition::batch_transition::batched_transition::token_unfreeze_transition::v0::TokenUnfreezeTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
    use crate::state_transition::batch_transition::batched_transition::DocumentTransferTransition;
    use crate::state_transition::batch_transition::batched_transition::DocumentUpdatePriceTransition;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::tokens::emergency_action::TokenEmergencyAction;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    fn base() -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::default(),
            identity_contract_nonce: 1,
            document_type_name: "d".to_string(),
            data_contract_id: Identifier::default(),
        })
    }

    fn tok_base() -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0::default())
    }

    fn mk_doc_create() -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::Create(DocumentCreateTransition::V0(
            DocumentCreateTransitionV0 {
                base: base(),
                entropy: [0u8; 32],
                data: BTreeMap::new(),
                prefunded_voting_balance: None,
            },
        )))
    }

    fn mk_doc_replace() -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::Replace(DocumentReplaceTransition::V0(
            DocumentReplaceTransitionV0 {
                base: base(),
                revision: 1,
                data: BTreeMap::new(),
            },
        )))
    }

    fn mk_doc_delete() -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::Delete(DocumentDeleteTransition::V0(
            DocumentDeleteTransitionV0 { base: base() },
        )))
    }

    fn mk_doc_transfer() -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::Transfer(
            DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
                base: base(),
                revision: 1,
                recipient_owner_id: Identifier::default(),
            }),
        ))
    }

    fn mk_doc_purchase() -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::Purchase(
            DocumentPurchaseTransition::V0(DocumentPurchaseTransitionV0 {
                base: base(),
                revision: 1,
                price: 100,
            }),
        ))
    }

    fn mk_doc_update_price() -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::UpdatePrice(
            DocumentUpdatePriceTransition::V0(DocumentUpdatePriceTransitionV0 {
                base: base(),
                revision: 1,
                price: 100,
            }),
        ))
    }

    fn mk_tok_burn() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::Burn(TokenBurnTransition::V0(
            TokenBurnTransitionV0 {
                base: tok_base(),
                burn_amount: 10,
                public_note: None,
            },
        )))
    }

    fn mk_tok_mint() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::Mint(TokenMintTransition::V0(
            TokenMintTransitionV0 {
                base: tok_base(),
                issued_to_identity_id: None,
                amount: 10,
                public_note: None,
            },
        )))
    }

    fn mk_tok_transfer() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::Transfer(TokenTransferTransition::V0(
            TokenTransferTransitionV0 {
                base: tok_base(),
                recipient_id: Identifier::default(),
                amount: 10,
                public_note: None,
                shared_encrypted_note: None,
                private_encrypted_note: None,
            },
        )))
    }

    fn mk_tok_freeze() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::Freeze(TokenFreezeTransition::V0(
            TokenFreezeTransitionV0 {
                base: tok_base(),
                identity_to_freeze_id: Identifier::default(),
                public_note: None,
            },
        )))
    }

    fn mk_tok_unfreeze() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::Unfreeze(TokenUnfreezeTransition::V0(
            TokenUnfreezeTransitionV0 {
                base: tok_base(),
                frozen_identity_id: Identifier::default(),
                public_note: None,
            },
        )))
    }

    fn mk_tok_destroy_frozen() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::DestroyFrozenFunds(
            TokenDestroyFrozenFundsTransition::V0(TokenDestroyFrozenFundsTransitionV0 {
                base: tok_base(),
                frozen_identity_id: Identifier::default(),
                public_note: None,
            }),
        ))
    }

    fn mk_tok_claim() -> BatchedTransition {
        use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
        BatchedTransition::Token(TokenTransition::Claim(TokenClaimTransition::V0(
            TokenClaimTransitionV0 {
                base: tok_base(),
                distribution_type: TokenDistributionType::PreProgrammed,
                public_note: None,
            },
        )))
    }

    fn mk_tok_emergency() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::EmergencyAction(
            TokenEmergencyActionTransition::V0(TokenEmergencyActionTransitionV0 {
                base: tok_base(),
                emergency_action: TokenEmergencyAction::Pause,
                public_note: None,
            }),
        ))
    }

    fn mk_tok_config_update() -> BatchedTransition {
        use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
        BatchedTransition::Token(TokenTransition::ConfigUpdate(
            TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
                base: tok_base(),
                update_token_configuration_item:
                    TokenConfigurationChangeItem::TokenConfigurationNoChange,
                public_note: None,
            }),
        ))
    }

    fn mk_tok_direct_purchase() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::DirectPurchase(
            TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
                base: tok_base(),
                token_count: 5,
                total_agreed_price: 500,
            }),
        ))
    }

    fn mk_tok_set_price() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::SetPriceForDirectPurchase(
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: tok_base(),
                    price: None,
                    public_note: None,
                },
            ),
        ))
    }

    // -----------------------------------------------------------------------
    // BatchedTransition resolvers — Document variant returns Some for
    // matching document types, None for mismatched document types, and
    // None for all token types.
    // -----------------------------------------------------------------------

    #[test]
    fn document_create_resolves_only_to_create() {
        let bt = mk_doc_create();
        assert!(bt.as_transition_create().is_some());
        assert!(bt.as_transition_replace().is_none());
        assert!(bt.as_transition_delete().is_none());
        assert!(bt.as_transition_transfer().is_none());
        assert!(bt.as_transition_purchase().is_none());
        // token accessors must return None for document variant
        assert!(bt.as_transition_token_burn().is_none());
        assert!(bt.as_transition_token_mint().is_none());
        assert!(bt.as_transition_token_transfer().is_none());
        assert!(bt.as_transition_token_freeze().is_none());
        assert!(bt.as_transition_token_unfreeze().is_none());
        assert!(bt.as_transition_token_destroy_frozen_funds().is_none());
        assert!(bt.as_transition_token_claim().is_none());
        assert!(bt.as_transition_token_emergency_action().is_none());
        assert!(bt.as_transition_token_config_update().is_none());
        assert!(bt.as_transition_token_direct_purchase().is_none());
        assert!(bt
            .as_transition_token_set_price_for_direct_purchase()
            .is_none());
    }

    #[test]
    fn document_replace_resolves_only_to_replace() {
        let bt = mk_doc_replace();
        assert!(bt.as_transition_create().is_none());
        assert!(bt.as_transition_replace().is_some());
        assert!(bt.as_transition_delete().is_none());
        assert!(bt.as_transition_transfer().is_none());
        assert!(bt.as_transition_purchase().is_none());
    }

    #[test]
    fn document_delete_resolves_only_to_delete() {
        let bt = mk_doc_delete();
        assert!(bt.as_transition_delete().is_some());
        assert!(bt.as_transition_create().is_none());
        assert!(bt.as_transition_replace().is_none());
    }

    #[test]
    fn document_transfer_resolves_only_to_transfer() {
        let bt = mk_doc_transfer();
        assert!(bt.as_transition_transfer().is_some());
        assert!(bt.as_transition_purchase().is_none());
    }

    #[test]
    fn document_purchase_resolves_only_to_purchase() {
        let bt = mk_doc_purchase();
        assert!(bt.as_transition_purchase().is_some());
        assert!(bt.as_transition_transfer().is_none());
    }

    #[test]
    fn document_update_price_resolves_to_no_token_variants() {
        // UpdatePrice is a document-side variant with no accessor — all
        // document and token accessors must return None.
        let bt = mk_doc_update_price();
        assert!(bt.as_transition_create().is_none());
        assert!(bt.as_transition_replace().is_none());
        assert!(bt.as_transition_delete().is_none());
        assert!(bt.as_transition_transfer().is_none());
        assert!(bt.as_transition_purchase().is_none());
        assert!(bt.as_transition_token_burn().is_none());
    }

    // -----------------------------------------------------------------------
    // BatchedTransition resolvers — each token variant resolves to exactly
    // one token accessor, all others return None. This covers the 14 token
    // match arms that Document(_) => None short-circuits.
    // -----------------------------------------------------------------------

    #[test]
    fn token_burn_resolves_only_to_burn() {
        let bt = mk_tok_burn();
        assert!(bt.as_transition_token_burn().is_some());
        assert!(bt.as_transition_token_mint().is_none());
        assert!(bt.as_transition_create().is_none());
    }

    #[test]
    fn token_mint_resolves_only_to_mint() {
        let bt = mk_tok_mint();
        assert!(bt.as_transition_token_mint().is_some());
        assert!(bt.as_transition_token_burn().is_none());
    }

    #[test]
    fn token_transfer_resolves_only_to_token_transfer() {
        let bt = mk_tok_transfer();
        assert!(bt.as_transition_token_transfer().is_some());
        // Not to be confused with document transfer
        assert!(bt.as_transition_transfer().is_none());
    }

    #[test]
    fn token_freeze_resolves_only_to_freeze() {
        let bt = mk_tok_freeze();
        assert!(bt.as_transition_token_freeze().is_some());
        assert!(bt.as_transition_token_unfreeze().is_none());
    }

    #[test]
    fn token_unfreeze_resolves_only_to_unfreeze() {
        let bt = mk_tok_unfreeze();
        assert!(bt.as_transition_token_unfreeze().is_some());
        assert!(bt.as_transition_token_freeze().is_none());
    }

    #[test]
    fn token_destroy_frozen_resolves_only_to_destroy_frozen() {
        let bt = mk_tok_destroy_frozen();
        assert!(bt.as_transition_token_destroy_frozen_funds().is_some());
        assert!(bt.as_transition_token_freeze().is_none());
    }

    #[test]
    fn token_claim_resolves_only_to_claim() {
        let bt = mk_tok_claim();
        assert!(bt.as_transition_token_claim().is_some());
        assert!(bt.as_transition_token_mint().is_none());
    }

    #[test]
    fn token_emergency_resolves_only_to_emergency() {
        let bt = mk_tok_emergency();
        assert!(bt.as_transition_token_emergency_action().is_some());
        assert!(bt.as_transition_token_config_update().is_none());
    }

    #[test]
    fn token_config_update_resolves_only_to_config_update() {
        let bt = mk_tok_config_update();
        assert!(bt.as_transition_token_config_update().is_some());
        assert!(bt.as_transition_token_emergency_action().is_none());
    }

    #[test]
    fn token_direct_purchase_resolves_only_to_direct_purchase() {
        let bt = mk_tok_direct_purchase();
        assert!(bt.as_transition_token_direct_purchase().is_some());
        assert!(bt
            .as_transition_token_set_price_for_direct_purchase()
            .is_none());
    }

    #[test]
    fn token_set_price_resolves_only_to_set_price() {
        let bt = mk_tok_set_price();
        assert!(bt
            .as_transition_token_set_price_for_direct_purchase()
            .is_some());
        assert!(bt.as_transition_token_direct_purchase().is_none());
    }

    // -----------------------------------------------------------------------
    // BatchedTransitionRef resolvers — mirror tests, against the Ref impl.
    // -----------------------------------------------------------------------

    #[test]
    fn ref_document_create_resolves_only_to_create() {
        let bt = mk_doc_create();
        let r = bt.borrow_as_ref();
        assert!(r.as_transition_create().is_some());
        assert!(r.as_transition_replace().is_none());
        assert!(r.as_transition_delete().is_none());
        assert!(r.as_transition_transfer().is_none());
        assert!(r.as_transition_purchase().is_none());
        assert!(r.as_transition_token_burn().is_none());
        assert!(r.as_transition_token_mint().is_none());
        assert!(r.as_transition_token_transfer().is_none());
        assert!(r.as_transition_token_freeze().is_none());
        assert!(r.as_transition_token_unfreeze().is_none());
        assert!(r.as_transition_token_destroy_frozen_funds().is_none());
        assert!(r.as_transition_token_claim().is_none());
        assert!(r.as_transition_token_emergency_action().is_none());
        assert!(r.as_transition_token_config_update().is_none());
        assert!(r.as_transition_token_direct_purchase().is_none());
        assert!(r
            .as_transition_token_set_price_for_direct_purchase()
            .is_none());
    }

    #[test]
    fn ref_all_token_variants_short_circuit_document_accessors() {
        // For every token variant, all document-side accessors must be None.
        // This exercises the Token(_) => None arms in each document resolver.
        for bt in [
            mk_tok_burn(),
            mk_tok_mint(),
            mk_tok_transfer(),
            mk_tok_freeze(),
            mk_tok_unfreeze(),
            mk_tok_destroy_frozen(),
            mk_tok_claim(),
            mk_tok_emergency(),
            mk_tok_config_update(),
            mk_tok_direct_purchase(),
            mk_tok_set_price(),
        ] {
            let r = bt.borrow_as_ref();
            assert!(r.as_transition_create().is_none());
            assert!(r.as_transition_replace().is_none());
            assert!(r.as_transition_delete().is_none());
            assert!(r.as_transition_transfer().is_none());
            assert!(r.as_transition_purchase().is_none());
        }
    }

    #[test]
    fn ref_all_document_variants_short_circuit_token_accessors() {
        // For every document variant, all token-side accessors on the ref
        // must be None — covers the Document(_) => None arms on the Ref impl.
        for bt in [
            mk_doc_create(),
            mk_doc_replace(),
            mk_doc_delete(),
            mk_doc_transfer(),
            mk_doc_purchase(),
            mk_doc_update_price(),
        ] {
            let r = bt.borrow_as_ref();
            assert!(r.as_transition_token_burn().is_none());
            assert!(r.as_transition_token_mint().is_none());
            assert!(r.as_transition_token_transfer().is_none());
            assert!(r.as_transition_token_freeze().is_none());
            assert!(r.as_transition_token_unfreeze().is_none());
            assert!(r.as_transition_token_destroy_frozen_funds().is_none());
            assert!(r.as_transition_token_claim().is_none());
            assert!(r.as_transition_token_emergency_action().is_none());
            assert!(r.as_transition_token_config_update().is_none());
            assert!(r.as_transition_token_direct_purchase().is_none());
            assert!(r
                .as_transition_token_set_price_for_direct_purchase()
                .is_none());
        }
    }
}
