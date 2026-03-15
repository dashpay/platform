#[cfg(test)]
mod batch_transition_tests {
    use crate::identity::SecurityLevel;
    use crate::state_transition::batch_transition::batched_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransition;
    use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::DocumentPurchaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransition;
    use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use crate::state_transition::batch_transition::batched_transition::document_transition_action_type::{
        DocumentTransitionActionType, DocumentTransitionActionTypeGetter,
    };
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::token_burn_transition::v0::TokenBurnTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_burn_transition::TokenBurnTransition;
    use crate::state_transition::batch_transition::batched_transition::token_claim_transition::v0::TokenClaimTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_claim_transition::TokenClaimTransition;
    use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::TokenTransferTransition;
    use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
    use crate::state_transition::batch_transition::batched_transition::token_transition_action_type::{
        TokenTransitionActionType, TokenTransitionActionTypeGetter,
    };
    use crate::state_transition::batch_transition::batched_transition::{
        BatchedTransition, BatchedTransitionMutRef, BatchedTransitionRef,
    };
    use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use crate::state_transition::batch_transition::fields;
    use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
    use crate::state_transition::batch_transition::{
        BatchTransition, BatchTransitionV0, BatchTransitionV1,
    };
    use crate::state_transition::{
        FeatureVersioned, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
        StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned,
        StateTransitionType,
    };
    use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use platform_value::{BinaryData, Identifier};
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // Helper functions to reduce boilerplate
    // -----------------------------------------------------------------------

    fn make_base_transition(nonce: u64) -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::new([nonce as u8; 32]),
            identity_contract_nonce: nonce,
            document_type_name: "test_doc".to_string(),
            data_contract_id: Identifier::new([0xAA; 32]),
        })
    }

    fn make_delete_transition(nonce: u64) -> DocumentTransition {
        DocumentTransition::Delete(DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
            base: make_base_transition(nonce),
        }))
    }

    fn make_purchase_transition(nonce: u64, price: u64) -> DocumentTransition {
        DocumentTransition::Purchase(DocumentPurchaseTransition::V0(
            DocumentPurchaseTransitionV0 {
                base: make_base_transition(nonce),
                revision: 1,
                price,
            },
        ))
    }

    fn make_create_transition_with_prefunded(
        nonce: u64,
        prefunded: Option<(String, u64)>,
    ) -> DocumentTransition {
        DocumentTransition::Create(DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
            base: make_base_transition(nonce),
            entropy: [nonce as u8; 32],
            data: BTreeMap::new(),
            prefunded_voting_balance: prefunded,
        }))
    }

    fn make_token_base(nonce: u64) -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: nonce,
            token_contract_position: 0,
            data_contract_id: Identifier::new([0xBB; 32]),
            token_id: Identifier::new([0xCC; 32]),
            using_group_info: None,
        })
    }

    fn make_token_burn_transition(nonce: u64, amount: u64) -> TokenTransition {
        TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
            base: make_token_base(nonce),
            burn_amount: amount,
            public_note: None,
        }))
    }

    fn make_token_transfer_transition(nonce: u64) -> TokenTransition {
        TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_token_base(nonce),
            recipient_id: Identifier::new([0xDD; 32]),
            amount: 1000,
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        }))
    }

    fn make_token_claim_transition(nonce: u64) -> TokenTransition {
        TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: make_token_base(nonce),
            distribution_type: TokenDistributionType::PreProgrammed,
            public_note: None,
        }))
    }

    fn make_batch_v0(transitions: Vec<DocumentTransition>) -> BatchTransitionV0 {
        BatchTransitionV0 {
            owner_id: Identifier::new([0x01; 32]),
            transitions,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }

    fn make_batch_v1(transitions: Vec<BatchedTransition>) -> BatchTransitionV1 {
        BatchTransitionV1 {
            owner_id: Identifier::new([0x02; 32]),
            transitions,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: Accessors (DocumentsBatchTransitionAccessorsV0)
    // -----------------------------------------------------------------------

    #[test]
    fn v0_transitions_iter_yields_document_refs() {
        let batch = make_batch_v0(vec![make_delete_transition(1), make_delete_transition(2)]);
        let refs: Vec<_> = batch.transitions_iter().collect();
        assert_eq!(refs.len(), 2);
        for r in &refs {
            assert!(matches!(r, BatchedTransitionRef::Document(_)));
        }
    }

    #[test]
    fn v0_transitions_len_returns_count() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        assert_eq!(batch.transitions_len(), 1);
    }

    #[test]
    fn v0_transitions_are_empty_when_no_transitions() {
        let batch = make_batch_v0(vec![]);
        assert!(batch.transitions_are_empty());
    }

    #[test]
    fn v0_transitions_are_not_empty_with_transitions() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        assert!(!batch.transitions_are_empty());
    }

    #[test]
    fn v0_first_transition_returns_some() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        assert!(batch.first_transition().is_some());
    }

    #[test]
    fn v0_first_transition_returns_none_when_empty() {
        let batch = make_batch_v0(vec![]);
        assert!(batch.first_transition().is_none());
    }

    #[test]
    fn v0_first_transition_mut_returns_some() {
        let mut batch = make_batch_v0(vec![make_delete_transition(1)]);
        let first_mut = batch.first_transition_mut();
        assert!(first_mut.is_some());
        assert!(matches!(
            first_mut.unwrap(),
            BatchedTransitionMutRef::Document(_)
        ));
    }

    #[test]
    fn v0_first_transition_mut_returns_none_when_empty() {
        let mut batch = make_batch_v0(vec![]);
        assert!(batch.first_transition_mut().is_none());
    }

    #[test]
    fn v0_contains_document_transition_always_true() {
        let batch = make_batch_v0(vec![]);
        assert!(batch.contains_document_transition());
    }

    #[test]
    fn v0_contains_token_transition_always_false() {
        let batch = make_batch_v0(vec![]);
        assert!(!batch.contains_token_transition());
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: Methods (DocumentsBatchTransitionMethodsV0)
    // -----------------------------------------------------------------------

    #[test]
    fn v0_set_transitions_filters_out_token_transitions() {
        let mut batch = make_batch_v0(vec![make_delete_transition(1)]);
        let mixed = vec![
            BatchedTransition::Document(make_delete_transition(2)),
            BatchedTransition::Token(make_token_burn_transition(3, 100)),
        ];
        batch.set_transitions(mixed);
        // Token transitions should be filtered out, only document kept
        assert_eq!(batch.transitions.len(), 1);
    }

    #[test]
    fn v0_set_identity_contract_nonce_updates_all_transitions() {
        let mut batch = make_batch_v0(vec![make_delete_transition(1), make_delete_transition(2)]);
        use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
        batch.set_identity_contract_nonce(42);
        for t in &batch.transitions {
            assert_eq!(t.identity_contract_nonce(), 42);
        }
    }

    #[test]
    fn v0_all_document_purchases_amount_no_purchases() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        let result = batch.all_document_purchases_amount().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn v0_all_document_purchases_amount_single_purchase() {
        let batch = make_batch_v0(vec![make_purchase_transition(1, 5000)]);
        let result = batch.all_document_purchases_amount().unwrap();
        assert_eq!(result, Some(5000));
    }

    #[test]
    fn v0_all_document_purchases_amount_multiple_purchases() {
        let batch = make_batch_v0(vec![
            make_purchase_transition(1, 3000),
            make_purchase_transition(2, 2000),
        ]);
        let result = batch.all_document_purchases_amount().unwrap();
        assert_eq!(result, Some(5000));
    }

    #[test]
    fn v0_all_document_purchases_amount_overflow() {
        let batch = make_batch_v0(vec![
            make_purchase_transition(1, u64::MAX),
            make_purchase_transition(2, 1),
        ]);
        let result = batch.all_document_purchases_amount();
        assert!(result.is_err());
    }

    #[test]
    fn v0_all_conflicting_index_collateral_voting_funds_none() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        let result = batch
            .all_conflicting_index_collateral_voting_funds()
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn v0_all_conflicting_index_collateral_voting_funds_single() {
        let batch = make_batch_v0(vec![make_create_transition_with_prefunded(
            1,
            Some(("index1".to_string(), 10000)),
        )]);
        let result = batch
            .all_conflicting_index_collateral_voting_funds()
            .unwrap();
        assert_eq!(result, Some(10000));
    }

    #[test]
    fn v0_all_conflicting_index_collateral_voting_funds_multiple() {
        let batch = make_batch_v0(vec![
            make_create_transition_with_prefunded(1, Some(("idx1".to_string(), 3000))),
            make_create_transition_with_prefunded(2, Some(("idx2".to_string(), 7000))),
        ]);
        let result = batch
            .all_conflicting_index_collateral_voting_funds()
            .unwrap();
        assert_eq!(result, Some(10000));
    }

    #[test]
    fn v0_all_conflicting_index_collateral_voting_funds_overflow() {
        let batch = make_batch_v0(vec![
            make_create_transition_with_prefunded(1, Some(("idx1".to_string(), u64::MAX))),
            make_create_transition_with_prefunded(2, Some(("idx2".to_string(), 1))),
        ]);
        let result = batch.all_conflicting_index_collateral_voting_funds();
        assert!(result.is_err());
    }

    #[test]
    fn v0_all_conflicting_index_collateral_voting_funds_no_prefunded() {
        let batch = make_batch_v0(vec![make_create_transition_with_prefunded(1, None)]);
        let result = batch
            .all_conflicting_index_collateral_voting_funds()
            .unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: StateTransitionLike
    // -----------------------------------------------------------------------

    #[test]
    fn v0_modified_data_ids_returns_transition_ids() {
        let batch = make_batch_v0(vec![make_delete_transition(1), make_delete_transition(2)]);
        let ids = batch.modified_data_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], Identifier::new([1u8; 32]));
        assert_eq!(ids[1], Identifier::new([2u8; 32]));
    }

    #[test]
    fn v0_state_transition_protocol_version() {
        let batch = make_batch_v0(vec![]);
        assert_eq!(batch.state_transition_protocol_version(), 0);
    }

    #[test]
    fn v0_state_transition_type_is_batch() {
        let batch = make_batch_v0(vec![]);
        assert_eq!(batch.state_transition_type(), StateTransitionType::Batch);
    }

    #[test]
    fn v0_unique_identifiers_format() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        let ids = batch.unique_identifiers();
        assert_eq!(ids.len(), 1);
        // Should contain base64-encoded owner_id and data_contract_id and hex nonce
        assert!(ids[0].contains('-'));
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: StateTransitionHasUserFeeIncrease
    // -----------------------------------------------------------------------

    #[test]
    fn v0_user_fee_increase_get_and_set() {
        let mut batch = make_batch_v0(vec![]);
        assert_eq!(batch.user_fee_increase(), 0);
        batch.set_user_fee_increase(42);
        assert_eq!(batch.user_fee_increase(), 42);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: StateTransitionSingleSigned
    // -----------------------------------------------------------------------

    #[test]
    fn v0_signature_get_and_set() {
        let mut batch = make_batch_v0(vec![]);
        assert!(batch.signature().is_empty());
        let sig = BinaryData::new(vec![1, 2, 3, 4]);
        batch.set_signature(sig.clone());
        assert_eq!(*batch.signature(), sig);
    }

    #[test]
    fn v0_set_signature_bytes() {
        let mut batch = make_batch_v0(vec![]);
        batch.set_signature_bytes(vec![5, 6, 7]);
        assert_eq!(batch.signature().as_slice(), &[5, 6, 7]);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: StateTransitionOwned
    // -----------------------------------------------------------------------

    #[test]
    fn v0_owner_id() {
        let batch = make_batch_v0(vec![]);
        assert_eq!(batch.owner_id(), Identifier::new([0x01; 32]));
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: StateTransitionIdentitySigned
    // -----------------------------------------------------------------------

    #[test]
    fn v0_signature_public_key_id_get_and_set() {
        let mut batch = make_batch_v0(vec![]);
        assert_eq!(batch.signature_public_key_id(), 0);
        batch.set_signature_public_key_id(7);
        assert_eq!(batch.signature_public_key_id(), 7);
    }

    #[test]
    fn v0_security_level_requirement_returns_critical_high_medium() {
        use crate::identity::Purpose;
        let batch = make_batch_v0(vec![]);
        let levels = batch.security_level_requirement(Purpose::AUTHENTICATION);
        assert!(levels.contains(&SecurityLevel::CRITICAL));
        assert!(levels.contains(&SecurityLevel::HIGH));
        assert!(levels.contains(&SecurityLevel::MEDIUM));
        assert_eq!(levels.len(), 3);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV0: FeatureVersioned
    // -----------------------------------------------------------------------

    #[test]
    fn v0_feature_version_is_zero() {
        let batch = make_batch_v0(vec![]);
        assert_eq!(batch.feature_version(), 0);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: Accessors (DocumentsBatchTransitionAccessorsV0)
    // -----------------------------------------------------------------------

    #[test]
    fn v1_transitions_iter_yields_both_document_and_token_refs() {
        let batch = make_batch_v1(vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ]);
        let refs: Vec<_> = batch.transitions_iter().collect();
        assert_eq!(refs.len(), 2);
        assert!(matches!(refs[0], BatchedTransitionRef::Document(_)));
        assert!(matches!(refs[1], BatchedTransitionRef::Token(_)));
    }

    #[test]
    fn v1_transitions_len() {
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        assert_eq!(batch.transitions_len(), 1);
    }

    #[test]
    fn v1_transitions_are_empty() {
        let batch = make_batch_v1(vec![]);
        assert!(batch.transitions_are_empty());
    }

    #[test]
    fn v1_first_transition_document() {
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        let first = batch.first_transition();
        assert!(first.is_some());
        assert!(matches!(first.unwrap(), BatchedTransitionRef::Document(_)));
    }

    #[test]
    fn v1_first_transition_token() {
        let batch = make_batch_v1(vec![BatchedTransition::Token(make_token_burn_transition(
            1, 100,
        ))]);
        let first = batch.first_transition();
        assert!(first.is_some());
        assert!(matches!(first.unwrap(), BatchedTransitionRef::Token(_)));
    }

    #[test]
    fn v1_first_transition_mut_document() {
        let mut batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        let first = batch.first_transition_mut();
        assert!(first.is_some());
        assert!(matches!(
            first.unwrap(),
            BatchedTransitionMutRef::Document(_)
        ));
    }

    #[test]
    fn v1_first_transition_mut_token() {
        let mut batch = make_batch_v1(vec![BatchedTransition::Token(make_token_burn_transition(
            1, 100,
        ))]);
        let first = batch.first_transition_mut();
        assert!(first.is_some());
        assert!(matches!(first.unwrap(), BatchedTransitionMutRef::Token(_)));
    }

    #[test]
    fn v1_first_transition_none_when_empty() {
        let batch = make_batch_v1(vec![]);
        assert!(batch.first_transition().is_none());
    }

    #[test]
    fn v1_first_transition_mut_none_when_empty() {
        let mut batch = make_batch_v1(vec![]);
        assert!(batch.first_transition_mut().is_none());
    }

    #[test]
    fn v1_contains_document_transition_true() {
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        assert!(batch.contains_document_transition());
    }

    #[test]
    fn v1_contains_document_transition_false() {
        let batch = make_batch_v1(vec![BatchedTransition::Token(make_token_burn_transition(
            1, 100,
        ))]);
        assert!(!batch.contains_document_transition());
    }

    #[test]
    fn v1_contains_token_transition_true() {
        let batch = make_batch_v1(vec![BatchedTransition::Token(make_token_burn_transition(
            1, 100,
        ))]);
        assert!(batch.contains_token_transition());
    }

    #[test]
    fn v1_contains_token_transition_false() {
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        assert!(!batch.contains_token_transition());
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: Methods (DocumentsBatchTransitionMethodsV0)
    // -----------------------------------------------------------------------

    #[test]
    fn v1_set_transitions_preserves_all_types() {
        let mut batch = make_batch_v1(vec![]);
        let transitions = vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ];
        batch.set_transitions(transitions);
        assert_eq!(batch.transitions.len(), 2);
    }

    #[test]
    fn v1_set_identity_contract_nonce_updates_both_types() {
        let mut batch = make_batch_v1(vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ]);
        batch.set_identity_contract_nonce(99);
        for t in &batch.transitions {
            match t {
                BatchedTransition::Document(doc) => {
                    use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
                    assert_eq!(doc.identity_contract_nonce(), 99);
                }
                BatchedTransition::Token(tok) => {
                    use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
                    assert_eq!(tok.identity_contract_nonce(), 99);
                }
            }
        }
    }

    #[test]
    fn v1_all_document_purchases_amount_no_purchases() {
        let batch = make_batch_v1(vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ]);
        let result = batch.all_document_purchases_amount().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn v1_all_document_purchases_amount_with_purchase() {
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_purchase_transition(
            1, 7777,
        ))]);
        let result = batch.all_document_purchases_amount().unwrap();
        assert_eq!(result, Some(7777));
    }

    #[test]
    fn v1_all_document_purchases_amount_overflow() {
        let batch = make_batch_v1(vec![
            BatchedTransition::Document(make_purchase_transition(1, u64::MAX)),
            BatchedTransition::Document(make_purchase_transition(2, 1)),
        ]);
        let result = batch.all_document_purchases_amount();
        assert!(result.is_err());
    }

    #[test]
    fn v1_all_conflicting_index_collateral_voting_funds_with_value() {
        let batch = make_batch_v1(vec![BatchedTransition::Document(
            make_create_transition_with_prefunded(1, Some(("idx".to_string(), 5000))),
        )]);
        let result = batch
            .all_conflicting_index_collateral_voting_funds()
            .unwrap();
        assert_eq!(result, Some(5000));
    }

    #[test]
    fn v1_all_conflicting_index_collateral_voting_funds_overflow() {
        let batch = make_batch_v1(vec![
            BatchedTransition::Document(make_create_transition_with_prefunded(
                1,
                Some(("idx1".to_string(), u64::MAX)),
            )),
            BatchedTransition::Document(make_create_transition_with_prefunded(
                2,
                Some(("idx2".to_string(), 1)),
            )),
        ]);
        let result = batch.all_conflicting_index_collateral_voting_funds();
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: StateTransitionLike
    // -----------------------------------------------------------------------

    #[test]
    fn v1_modified_data_ids_only_includes_documents() {
        let batch = make_batch_v1(vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
            BatchedTransition::Document(make_delete_transition(3)),
        ]);
        let ids = batch.modified_data_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], Identifier::new([1u8; 32]));
        assert_eq!(ids[1], Identifier::new([3u8; 32]));
    }

    #[test]
    fn v1_state_transition_protocol_version() {
        let batch = make_batch_v1(vec![]);
        assert_eq!(batch.state_transition_protocol_version(), 1);
    }

    #[test]
    fn v1_state_transition_type_is_batch() {
        let batch = make_batch_v1(vec![]);
        assert_eq!(batch.state_transition_type(), StateTransitionType::Batch);
    }

    #[test]
    fn v1_unique_identifiers_for_mixed_transitions() {
        let batch = make_batch_v1(vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ]);
        let ids = batch.unique_identifiers();
        assert_eq!(ids.len(), 2);
        // Both should contain formatted strings with '-' separators
        for id in &ids {
            assert!(id.contains('-'));
        }
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: StateTransitionHasUserFeeIncrease
    // -----------------------------------------------------------------------

    #[test]
    fn v1_user_fee_increase_get_and_set() {
        let mut batch = make_batch_v1(vec![]);
        assert_eq!(batch.user_fee_increase(), 0);
        batch.set_user_fee_increase(55);
        assert_eq!(batch.user_fee_increase(), 55);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: StateTransitionSingleSigned
    // -----------------------------------------------------------------------

    #[test]
    fn v1_signature_get_and_set() {
        let mut batch = make_batch_v1(vec![]);
        assert!(batch.signature().is_empty());
        let sig = BinaryData::new(vec![10, 20, 30]);
        batch.set_signature(sig.clone());
        assert_eq!(*batch.signature(), sig);
    }

    #[test]
    fn v1_set_signature_bytes() {
        let mut batch = make_batch_v1(vec![]);
        batch.set_signature_bytes(vec![11, 22, 33]);
        assert_eq!(batch.signature().as_slice(), &[11, 22, 33]);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: StateTransitionOwned
    // -----------------------------------------------------------------------

    #[test]
    fn v1_owner_id() {
        let batch = make_batch_v1(vec![]);
        assert_eq!(batch.owner_id(), Identifier::new([0x02; 32]));
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: StateTransitionIdentitySigned
    // -----------------------------------------------------------------------

    #[test]
    fn v1_signature_public_key_id_get_and_set() {
        let mut batch = make_batch_v1(vec![]);
        assert_eq!(batch.signature_public_key_id(), 0);
        batch.set_signature_public_key_id(11);
        assert_eq!(batch.signature_public_key_id(), 11);
    }

    #[test]
    fn v1_security_level_requirement_authentication() {
        use crate::identity::Purpose;
        let batch = make_batch_v1(vec![]);
        let levels = batch.security_level_requirement(Purpose::AUTHENTICATION);
        assert!(levels.contains(&SecurityLevel::CRITICAL));
        assert!(levels.contains(&SecurityLevel::HIGH));
        assert!(levels.contains(&SecurityLevel::MEDIUM));
    }

    #[test]
    fn v1_security_level_requirement_transfer() {
        use crate::identity::Purpose;
        let batch = make_batch_v1(vec![]);
        let levels = batch.security_level_requirement(Purpose::TRANSFER);
        assert_eq!(levels, vec![SecurityLevel::CRITICAL]);
    }

    #[test]
    fn v1_purpose_requirement_default() {
        use crate::identity::Purpose;
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        let purposes = batch.purpose_requirement();
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION]);
    }

    #[test]
    fn v1_purpose_requirement_single_token_transfer() {
        use crate::identity::Purpose;
        let batch = make_batch_v1(vec![BatchedTransition::Token(
            make_token_transfer_transition(1),
        )]);
        let purposes = batch.purpose_requirement();
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION, Purpose::TRANSFER]);
    }

    #[test]
    fn v1_purpose_requirement_single_token_claim() {
        use crate::identity::Purpose;
        let batch = make_batch_v1(vec![BatchedTransition::Token(make_token_claim_transition(
            1,
        ))]);
        let purposes = batch.purpose_requirement();
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION, Purpose::TRANSFER]);
    }

    #[test]
    fn v1_purpose_requirement_multiple_transitions_no_transfer() {
        use crate::identity::Purpose;
        let batch = make_batch_v1(vec![
            BatchedTransition::Token(make_token_transfer_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ]);
        let purposes = batch.purpose_requirement();
        // With more than 1 transition, purpose_requirement returns AUTHENTICATION only
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION]);
    }

    #[test]
    fn v1_purpose_requirement_empty_transitions() {
        use crate::identity::Purpose;
        let batch = make_batch_v1(vec![]);
        let purposes = batch.purpose_requirement();
        // When transitions_len() == 0, we reach the default path
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION]);
    }

    // -----------------------------------------------------------------------
    // BatchTransitionV1: FeatureVersioned
    // -----------------------------------------------------------------------

    #[test]
    fn v1_feature_version_is_one() {
        let batch = make_batch_v1(vec![]);
        assert_eq!(batch.feature_version(), 1);
    }

    // -----------------------------------------------------------------------
    // Top-level BatchTransition enum: dispatch tests
    // -----------------------------------------------------------------------

    #[test]
    fn batch_enum_dispatches_modified_data_ids_v0() {
        let inner = make_batch_v0(vec![make_delete_transition(1)]);
        let batch = BatchTransition::V0(inner);
        let ids = batch.modified_data_ids();
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn batch_enum_dispatches_modified_data_ids_v1() {
        let inner = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        let batch = BatchTransition::V1(inner);
        let ids = batch.modified_data_ids();
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn batch_enum_state_transition_protocol_version() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![]));
        assert_eq!(v0.state_transition_protocol_version(), 0);
        assert_eq!(v1.state_transition_protocol_version(), 1);
    }

    #[test]
    fn batch_enum_state_transition_type() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![]));
        assert_eq!(v0.state_transition_type(), StateTransitionType::Batch);
        assert_eq!(v1.state_transition_type(), StateTransitionType::Batch);
    }

    #[test]
    fn batch_enum_unique_identifiers() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![make_delete_transition(1)]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_delete_transition(1),
        )]));
        assert_eq!(v0.unique_identifiers().len(), 1);
        assert_eq!(v1.unique_identifiers().len(), 1);
    }

    #[test]
    fn batch_enum_user_fee_increase_dispatch() {
        let mut v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let mut v1 = BatchTransition::V1(make_batch_v1(vec![]));
        v0.set_user_fee_increase(10);
        v1.set_user_fee_increase(20);
        assert_eq!(v0.user_fee_increase(), 10);
        assert_eq!(v1.user_fee_increase(), 20);
    }

    #[test]
    fn batch_enum_signature_dispatch() {
        let mut v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let mut v1 = BatchTransition::V1(make_batch_v1(vec![]));
        let sig0 = BinaryData::new(vec![1, 2]);
        let sig1 = BinaryData::new(vec![3, 4]);
        v0.set_signature(sig0.clone());
        v1.set_signature(sig1.clone());
        assert_eq!(*v0.signature(), sig0);
        assert_eq!(*v1.signature(), sig1);
    }

    #[test]
    fn batch_enum_set_signature_bytes_dispatch() {
        let mut v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let mut v1 = BatchTransition::V1(make_batch_v1(vec![]));
        v0.set_signature_bytes(vec![10]);
        v1.set_signature_bytes(vec![20]);
        assert_eq!(v0.signature().as_slice(), &[10]);
        assert_eq!(v1.signature().as_slice(), &[20]);
    }

    #[test]
    fn batch_enum_owner_id_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![]));
        assert_eq!(v0.owner_id(), Identifier::new([0x01; 32]));
        assert_eq!(v1.owner_id(), Identifier::new([0x02; 32]));
    }

    #[test]
    fn batch_enum_signature_public_key_id_dispatch() {
        let mut v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let mut v1 = BatchTransition::V1(make_batch_v1(vec![]));
        v0.set_signature_public_key_id(5);
        v1.set_signature_public_key_id(6);
        assert_eq!(v0.signature_public_key_id(), 5);
        assert_eq!(v1.signature_public_key_id(), 6);
    }

    #[test]
    fn batch_enum_security_level_requirement_dispatch() {
        use crate::identity::Purpose;
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![]));
        let l0 = v0.security_level_requirement(Purpose::AUTHENTICATION);
        let l1 = v1.security_level_requirement(Purpose::AUTHENTICATION);
        assert_eq!(l0.len(), 3);
        assert_eq!(l1.len(), 3);
    }

    #[test]
    fn batch_enum_purpose_requirement_dispatch() {
        use crate::identity::Purpose;
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![]));
        let p0 = v0.purpose_requirement();
        let p1 = v1.purpose_requirement();
        // V0 returns default [AUTHENTICATION], V1 with empty also returns [AUTHENTICATION]
        assert_eq!(p0, vec![Purpose::AUTHENTICATION]);
        assert_eq!(p1, vec![Purpose::AUTHENTICATION]);
    }

    #[test]
    fn batch_enum_feature_version_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![]));
        assert_eq!(v0.feature_version(), 0);
        assert_eq!(v1.feature_version(), 1);
    }

    // -----------------------------------------------------------------------
    // Top-level BatchTransition: accessor dispatches
    // -----------------------------------------------------------------------

    #[test]
    fn batch_enum_transitions_iter_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![make_delete_transition(1)]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_delete_transition(1),
        )]));
        assert_eq!(v0.transitions_iter().count(), 1);
        assert_eq!(v1.transitions_iter().count(), 1);
    }

    #[test]
    fn batch_enum_transitions_len_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![
            make_delete_transition(1),
            make_delete_transition(2),
        ]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_delete_transition(1),
        )]));
        assert_eq!(v0.transitions_len(), 2);
        assert_eq!(v1.transitions_len(), 1);
    }

    #[test]
    fn batch_enum_transitions_are_empty_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![]));
        assert!(v0.transitions_are_empty());
        assert!(v1.transitions_are_empty());
    }

    #[test]
    fn batch_enum_first_transition_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![make_delete_transition(1)]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Token(
            make_token_burn_transition(1, 100),
        )]));
        assert!(matches!(
            v0.first_transition().unwrap(),
            BatchedTransitionRef::Document(_)
        ));
        assert!(matches!(
            v1.first_transition().unwrap(),
            BatchedTransitionRef::Token(_)
        ));
    }

    #[test]
    fn batch_enum_first_transition_mut_dispatch() {
        let mut v0 = BatchTransition::V0(make_batch_v0(vec![make_delete_transition(1)]));
        let mut v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Token(
            make_token_burn_transition(1, 100),
        )]));
        assert!(v0.first_transition_mut().is_some());
        assert!(v1.first_transition_mut().is_some());
    }

    #[test]
    fn batch_enum_contains_document_transition_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1_doc = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_delete_transition(1),
        )]));
        let v1_tok = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Token(
            make_token_burn_transition(1, 100),
        )]));
        assert!(v0.contains_document_transition()); // V0 always true
        assert!(v1_doc.contains_document_transition());
        assert!(!v1_tok.contains_document_transition());
    }

    #[test]
    fn batch_enum_contains_token_transition_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let v1_tok = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Token(
            make_token_burn_transition(1, 100),
        )]));
        let v1_doc = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_delete_transition(1),
        )]));
        assert!(!v0.contains_token_transition()); // V0 always false
        assert!(v1_tok.contains_token_transition());
        assert!(!v1_doc.contains_token_transition());
    }

    // -----------------------------------------------------------------------
    // Top-level BatchTransition: methods dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn batch_enum_all_document_purchases_amount_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![make_purchase_transition(1, 100)]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_purchase_transition(1, 200),
        )]));
        assert_eq!(v0.all_document_purchases_amount().unwrap(), Some(100));
        assert_eq!(v1.all_document_purchases_amount().unwrap(), Some(200));
    }

    #[test]
    fn batch_enum_all_conflicting_index_collateral_voting_funds_dispatch() {
        let v0 = BatchTransition::V0(make_batch_v0(vec![make_create_transition_with_prefunded(
            1,
            Some(("idx".to_string(), 500)),
        )]));
        let v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_create_transition_with_prefunded(1, Some(("idx".to_string(), 600))),
        )]));
        assert_eq!(
            v0.all_conflicting_index_collateral_voting_funds().unwrap(),
            Some(500)
        );
        assert_eq!(
            v1.all_conflicting_index_collateral_voting_funds().unwrap(),
            Some(600)
        );
    }

    #[test]
    fn batch_enum_set_transitions_dispatch() {
        let mut v0 = BatchTransition::V0(make_batch_v0(vec![]));
        let mut v1 = BatchTransition::V1(make_batch_v1(vec![]));
        v0.set_transitions(vec![BatchedTransition::Document(make_delete_transition(1))]);
        v1.set_transitions(vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ]);
        assert_eq!(v0.transitions_len(), 1); // V0 filters out tokens
        assert_eq!(v1.transitions_len(), 2); // V1 keeps both
    }

    #[test]
    fn batch_enum_set_identity_contract_nonce_dispatch() {
        let mut v0 = BatchTransition::V0(make_batch_v0(vec![make_delete_transition(1)]));
        let mut v1 = BatchTransition::V1(make_batch_v1(vec![BatchedTransition::Document(
            make_delete_transition(1),
        )]));
        v0.set_identity_contract_nonce(77);
        v1.set_identity_contract_nonce(88);
        // Just verify no panic; the nonce was set internally
    }

    // -----------------------------------------------------------------------
    // get_security_level_requirement function
    // -----------------------------------------------------------------------

    #[test]
    fn get_security_level_requirement_returns_default_when_none() {
        use crate::state_transition::batch_transition::get_security_level_requirement;
        use platform_value::Value;

        let val = Value::Map(vec![]);
        let result = get_security_level_requirement(&val, SecurityLevel::HIGH);
        assert_eq!(result, SecurityLevel::HIGH);
    }

    #[test]
    fn get_security_level_requirement_returns_specified_level() {
        use crate::state_transition::batch_transition::get_security_level_requirement;
        use platform_value::Value;

        // SecurityLevel::MASTER is 0, CRITICAL is 1, HIGH is 2, MEDIUM is 3
        let val = Value::Map(vec![(
            Value::Text("signatureSecurityLevelRequirement".to_string()),
            Value::U64(1), // CRITICAL
        )]);
        let result = get_security_level_requirement(&val, SecurityLevel::HIGH);
        assert_eq!(result, SecurityLevel::CRITICAL);
    }

    #[test]
    fn get_security_level_requirement_invalid_level_returns_default() {
        use crate::state_transition::batch_transition::get_security_level_requirement;
        use platform_value::Value;

        let val = Value::Map(vec![(
            Value::Text("signatureSecurityLevelRequirement".to_string()),
            Value::U64(255), // Invalid level
        )]);
        let result = get_security_level_requirement(&val, SecurityLevel::HIGH);
        assert_eq!(result, SecurityLevel::HIGH);
    }

    // -----------------------------------------------------------------------
    // StateTransitionFieldTypes
    // -----------------------------------------------------------------------

    #[test]
    fn batch_transition_field_types_binary_paths() {
        use crate::state_transition::StateTransitionFieldTypes;
        let paths = BatchTransition::binary_property_paths();
        assert!(!paths.is_empty());
        assert!(paths.contains(&"signature"));
    }

    #[test]
    fn batch_transition_field_types_identifier_paths() {
        use crate::state_transition::StateTransitionFieldTypes;
        let paths = BatchTransition::identifiers_property_paths();
        assert!(paths.contains(&"ownerId"));
    }

    #[test]
    fn batch_transition_field_types_signature_paths() {
        use crate::state_transition::StateTransitionFieldTypes;
        let paths = BatchTransition::signature_property_paths();
        assert!(paths.contains(&"signature"));
        assert!(paths.contains(&"signaturePublicKeyId"));
    }

    // -----------------------------------------------------------------------
    // Fields constants
    // -----------------------------------------------------------------------

    #[test]
    fn fields_constants_are_correct() {
        assert_eq!(fields::property_names::OWNER_ID, "ownerId");
        assert_eq!(fields::property_names::TRANSITIONS, "transitions");
        assert_eq!(fields::property_names::DOCUMENT_TYPE, "$type");
        assert_eq!(fields::property_names::DATA_CONTRACT_ID, "$dataContractId");
        assert_eq!(
            fields::property_names::SECURITY_LEVEL_REQUIREMENT,
            "signatureSecurityLevelRequirement"
        );
        assert_eq!(fields::DEFAULT_SECURITY_LEVEL, SecurityLevel::HIGH);
    }

    #[test]
    fn identifier_fields_contain_expected() {
        assert_eq!(fields::IDENTIFIER_FIELDS.len(), 3);
    }

    #[test]
    fn u16_fields_contain_expected() {
        assert_eq!(fields::U16_FIELDS.len(), 1);
    }

    // -----------------------------------------------------------------------
    // DocumentTransitionActionType
    // -----------------------------------------------------------------------

    #[test]
    fn document_action_type_getter_create() {
        let dt = DocumentTransition::Create(DocumentCreateTransition::default());
        assert_eq!(dt.action_type(), DocumentTransitionActionType::Create);
    }

    #[test]
    fn document_action_type_getter_delete() {
        let dt = make_delete_transition(1);
        assert_eq!(dt.action_type(), DocumentTransitionActionType::Delete);
    }

    #[test]
    fn document_action_type_getter_purchase() {
        let dt = make_purchase_transition(1, 100);
        assert_eq!(dt.action_type(), DocumentTransitionActionType::Purchase);
    }

    #[test]
    fn document_action_type_from_str_valid() {
        assert_eq!(
            DocumentTransitionActionType::try_from("create").unwrap(),
            DocumentTransitionActionType::Create
        );
        assert_eq!(
            DocumentTransitionActionType::try_from("replace").unwrap(),
            DocumentTransitionActionType::Replace
        );
        assert_eq!(
            DocumentTransitionActionType::try_from("delete").unwrap(),
            DocumentTransitionActionType::Delete
        );
        assert_eq!(
            DocumentTransitionActionType::try_from("transfer").unwrap(),
            DocumentTransitionActionType::Transfer
        );
        assert_eq!(
            DocumentTransitionActionType::try_from("updatePrice").unwrap(),
            DocumentTransitionActionType::UpdatePrice
        );
        assert_eq!(
            DocumentTransitionActionType::try_from("update_price").unwrap(),
            DocumentTransitionActionType::UpdatePrice
        );
        assert_eq!(
            DocumentTransitionActionType::try_from("purchase").unwrap(),
            DocumentTransitionActionType::Purchase
        );
    }

    #[test]
    fn document_action_type_from_str_invalid() {
        let result = DocumentTransitionActionType::try_from("nonexistent");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // TokenTransitionActionType
    // -----------------------------------------------------------------------

    #[test]
    fn token_action_type_getter() {
        let burn = make_token_burn_transition(1, 100);
        assert_eq!(burn.action_type(), TokenTransitionActionType::Burn);

        let transfer = make_token_transfer_transition(1);
        assert_eq!(transfer.action_type(), TokenTransitionActionType::Transfer);

        let claim = make_token_claim_transition(1);
        assert_eq!(claim.action_type(), TokenTransitionActionType::Claim);
    }

    #[test]
    fn token_action_type_display() {
        assert_eq!(format!("{}", TokenTransitionActionType::Burn), "Burn");
        assert_eq!(format!("{}", TokenTransitionActionType::Mint), "Mint");
        assert_eq!(
            format!("{}", TokenTransitionActionType::Transfer),
            "Transfer"
        );
        assert_eq!(format!("{}", TokenTransitionActionType::Freeze), "Freeze");
        assert_eq!(
            format!("{}", TokenTransitionActionType::Unfreeze),
            "Unfreeze"
        );
        assert_eq!(
            format!("{}", TokenTransitionActionType::DestroyFrozenFunds),
            "DestroyFrozenFunds"
        );
        assert_eq!(format!("{}", TokenTransitionActionType::Claim), "Claim");
        assert_eq!(
            format!("{}", TokenTransitionActionType::EmergencyAction),
            "EmergencyAction"
        );
        assert_eq!(
            format!("{}", TokenTransitionActionType::ConfigUpdate),
            "ConfigUpdate"
        );
        assert_eq!(
            format!("{}", TokenTransitionActionType::DirectPurchase),
            "DirectPurchase"
        );
        assert_eq!(
            format!("{}", TokenTransitionActionType::SetPriceForDirectPurchase),
            "SetPriceForDirectPurchase"
        );
    }

    #[test]
    fn token_action_type_from_str_valid() {
        assert_eq!(
            TokenTransitionActionType::try_from("burn").unwrap(),
            TokenTransitionActionType::Burn
        );
        assert_eq!(
            TokenTransitionActionType::try_from("issuance").unwrap(),
            TokenTransitionActionType::Mint
        );
        assert_eq!(
            TokenTransitionActionType::try_from("transfer").unwrap(),
            TokenTransitionActionType::Transfer
        );
        assert_eq!(
            TokenTransitionActionType::try_from("freeze").unwrap(),
            TokenTransitionActionType::Freeze
        );
        assert_eq!(
            TokenTransitionActionType::try_from("unfreeze").unwrap(),
            TokenTransitionActionType::Unfreeze
        );
        assert_eq!(
            TokenTransitionActionType::try_from("claim").unwrap(),
            TokenTransitionActionType::Claim
        );
        assert_eq!(
            TokenTransitionActionType::try_from("destroy_frozen_funds").unwrap(),
            TokenTransitionActionType::DestroyFrozenFunds
        );
        assert_eq!(
            TokenTransitionActionType::try_from("destroyFrozenFunds").unwrap(),
            TokenTransitionActionType::DestroyFrozenFunds
        );
        assert_eq!(
            TokenTransitionActionType::try_from("emergency_action").unwrap(),
            TokenTransitionActionType::EmergencyAction
        );
        assert_eq!(
            TokenTransitionActionType::try_from("emergencyAction").unwrap(),
            TokenTransitionActionType::EmergencyAction
        );
        assert_eq!(
            TokenTransitionActionType::try_from("config_update").unwrap(),
            TokenTransitionActionType::ConfigUpdate
        );
        assert_eq!(
            TokenTransitionActionType::try_from("configUpdate").unwrap(),
            TokenTransitionActionType::ConfigUpdate
        );
        assert_eq!(
            TokenTransitionActionType::try_from("direct_purchase").unwrap(),
            TokenTransitionActionType::DirectPurchase
        );
        assert_eq!(
            TokenTransitionActionType::try_from("directPurchase").unwrap(),
            TokenTransitionActionType::DirectPurchase
        );
        assert_eq!(
            TokenTransitionActionType::try_from("set_price_for_direct_purchase").unwrap(),
            TokenTransitionActionType::SetPriceForDirectPurchase
        );
        assert_eq!(
            TokenTransitionActionType::try_from("setPriceForDirectPurchase").unwrap(),
            TokenTransitionActionType::SetPriceForDirectPurchase
        );
    }

    #[test]
    fn token_action_type_from_str_invalid() {
        let result = TokenTransitionActionType::try_from("nonexistent");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // BatchedTransition: borrow_as_ref, borrow_as_mut, set_identity_contract_nonce
    // -----------------------------------------------------------------------

    #[test]
    fn batched_transition_borrow_as_ref_document() {
        let bt = BatchedTransition::Document(make_delete_transition(1));
        let r = bt.borrow_as_ref();
        assert!(matches!(r, BatchedTransitionRef::Document(_)));
    }

    #[test]
    fn batched_transition_borrow_as_ref_token() {
        let bt = BatchedTransition::Token(make_token_burn_transition(1, 100));
        let r = bt.borrow_as_ref();
        assert!(matches!(r, BatchedTransitionRef::Token(_)));
    }

    #[test]
    fn batched_transition_borrow_as_mut_document() {
        let mut bt = BatchedTransition::Document(make_delete_transition(1));
        let r = bt.borrow_as_mut();
        assert!(matches!(r, BatchedTransitionMutRef::Document(_)));
    }

    #[test]
    fn batched_transition_borrow_as_mut_token() {
        let mut bt = BatchedTransition::Token(make_token_burn_transition(1, 100));
        let r = bt.borrow_as_mut();
        assert!(matches!(r, BatchedTransitionMutRef::Token(_)));
    }

    #[test]
    fn batched_transition_set_identity_contract_nonce_document() {
        let mut bt = BatchedTransition::Document(make_delete_transition(1));
        bt.set_identity_contract_nonce(123);
        use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
        if let BatchedTransition::Document(doc) = &bt {
            assert_eq!(doc.identity_contract_nonce(), 123);
        }
    }

    #[test]
    fn batched_transition_set_identity_contract_nonce_token() {
        let mut bt = BatchedTransition::Token(make_token_burn_transition(1, 100));
        bt.set_identity_contract_nonce(456);
        use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
        if let BatchedTransition::Token(tok) = &bt {
            assert_eq!(tok.identity_contract_nonce(), 456);
        }
    }

    // -----------------------------------------------------------------------
    // BatchedTransitionRef: to_owned_transition, identity_contract_nonce, data_contract_id
    // -----------------------------------------------------------------------

    #[test]
    fn batched_transition_ref_to_owned_document() {
        let doc = make_delete_transition(1);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        let owned = bt_ref.to_owned_transition();
        assert!(matches!(owned, BatchedTransition::Document(_)));
    }

    #[test]
    fn batched_transition_ref_to_owned_token() {
        let tok = make_token_burn_transition(1, 100);
        let bt_ref = BatchedTransitionRef::Token(&tok);
        let owned = bt_ref.to_owned_transition();
        assert!(matches!(owned, BatchedTransition::Token(_)));
    }

    #[test]
    fn batched_transition_ref_identity_contract_nonce() {
        let doc = make_delete_transition(42);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        assert_eq!(bt_ref.identity_contract_nonce(), 42);
    }

    #[test]
    fn batched_transition_ref_data_contract_id_document() {
        let doc = make_delete_transition(1);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        assert_eq!(bt_ref.data_contract_id(), Identifier::new([0xAA; 32]));
    }

    #[test]
    fn batched_transition_ref_data_contract_id_token() {
        let tok = make_token_burn_transition(1, 100);
        let bt_ref = BatchedTransitionRef::Token(&tok);
        assert_eq!(bt_ref.data_contract_id(), Identifier::new([0xBB; 32]));
    }

    // -----------------------------------------------------------------------
    // Resolvers on BatchedTransition
    // -----------------------------------------------------------------------

    #[test]
    fn batched_transition_resolvers_document_create() {
        let create = make_create_transition_with_prefunded(1, None);
        let bt = BatchedTransition::Document(create);
        assert!(bt.as_transition_create().is_some());
        assert!(bt.as_transition_replace().is_none());
        assert!(bt.as_transition_delete().is_none());
        assert!(bt.as_transition_purchase().is_none());
        assert!(bt.as_transition_token_burn().is_none());
        assert!(bt.as_transition_token_mint().is_none());
    }

    #[test]
    fn batched_transition_resolvers_document_delete() {
        let bt = BatchedTransition::Document(make_delete_transition(1));
        assert!(bt.as_transition_delete().is_some());
        assert!(bt.as_transition_create().is_none());
    }

    #[test]
    fn batched_transition_resolvers_document_purchase() {
        let bt = BatchedTransition::Document(make_purchase_transition(1, 100));
        assert!(bt.as_transition_purchase().is_some());
        assert!(bt.as_transition_create().is_none());
    }

    #[test]
    fn batched_transition_resolvers_token_returns_none_for_document_methods() {
        let bt = BatchedTransition::Token(make_token_burn_transition(1, 100));
        assert!(bt.as_transition_create().is_none());
        assert!(bt.as_transition_replace().is_none());
        assert!(bt.as_transition_delete().is_none());
        assert!(bt.as_transition_purchase().is_none());
        assert!(bt.as_transition_transfer().is_none());
    }

    #[test]
    fn batched_transition_resolvers_token_burn() {
        let bt = BatchedTransition::Token(make_token_burn_transition(1, 100));
        assert!(bt.as_transition_token_burn().is_some());
        assert!(bt.as_transition_token_mint().is_none());
        assert!(bt.as_transition_token_transfer().is_none());
    }

    #[test]
    fn batched_transition_resolvers_token_transfer() {
        let bt = BatchedTransition::Token(make_token_transfer_transition(1));
        assert!(bt.as_transition_token_transfer().is_some());
        assert!(bt.as_transition_token_burn().is_none());
    }

    #[test]
    fn batched_transition_resolvers_token_claim() {
        let bt = BatchedTransition::Token(make_token_claim_transition(1));
        assert!(bt.as_transition_token_claim().is_some());
        assert!(bt.as_transition_token_burn().is_none());
    }

    // -----------------------------------------------------------------------
    // Resolvers on BatchedTransitionRef
    // -----------------------------------------------------------------------

    #[test]
    fn batched_transition_ref_resolvers_document_delete() {
        let doc = make_delete_transition(1);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        assert!(bt_ref.as_transition_delete().is_some());
        assert!(bt_ref.as_transition_create().is_none());
        assert!(bt_ref.as_transition_token_burn().is_none());
    }

    #[test]
    fn batched_transition_ref_resolvers_token_burn() {
        let tok = make_token_burn_transition(1, 100);
        let bt_ref = BatchedTransitionRef::Token(&tok);
        assert!(bt_ref.as_transition_token_burn().is_some());
        assert!(bt_ref.as_transition_create().is_none());
        assert!(bt_ref.as_transition_delete().is_none());
    }

    #[test]
    fn batched_transition_ref_resolvers_token_transfer() {
        let tok = make_token_transfer_transition(1);
        let bt_ref = BatchedTransitionRef::Token(&tok);
        assert!(bt_ref.as_transition_token_transfer().is_some());
        assert!(bt_ref.as_transition_token_burn().is_none());
    }

    #[test]
    fn batched_transition_ref_resolvers_token_claim() {
        let tok = make_token_claim_transition(1);
        let bt_ref = BatchedTransitionRef::Token(&tok);
        assert!(bt_ref.as_transition_token_claim().is_some());
    }

    #[test]
    fn batched_transition_ref_resolvers_document_returns_none_for_token_methods() {
        let doc = make_delete_transition(1);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        assert!(bt_ref.as_transition_token_burn().is_none());
        assert!(bt_ref.as_transition_token_mint().is_none());
        assert!(bt_ref.as_transition_token_transfer().is_none());
        assert!(bt_ref.as_transition_token_freeze().is_none());
        assert!(bt_ref.as_transition_token_unfreeze().is_none());
        assert!(bt_ref.as_transition_token_destroy_frozen_funds().is_none());
        assert!(bt_ref.as_transition_token_claim().is_none());
        assert!(bt_ref.as_transition_token_emergency_action().is_none());
        assert!(bt_ref.as_transition_token_config_update().is_none());
        assert!(bt_ref.as_transition_token_direct_purchase().is_none());
        assert!(bt_ref
            .as_transition_token_set_price_for_direct_purchase()
            .is_none());
    }

    // -----------------------------------------------------------------------
    // From conversions (BatchTransitionV0 -> StateTransition)
    // -----------------------------------------------------------------------

    #[test]
    fn batch_transition_v0_into_state_transition() {
        use crate::state_transition::StateTransition;
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        let st: StateTransition = batch.into();
        assert_eq!(st.state_transition_type(), StateTransitionType::Batch);
    }

    #[test]
    fn batch_transition_v1_into_state_transition() {
        use crate::state_transition::StateTransition;
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        let st: StateTransition = batch.into();
        assert_eq!(st.state_transition_type(), StateTransitionType::Batch);
    }

    // -----------------------------------------------------------------------
    // Default implementations
    // -----------------------------------------------------------------------

    #[test]
    fn batch_transition_v0_default() {
        let batch = BatchTransitionV0::default();
        assert_eq!(batch.transitions.len(), 0);
        assert_eq!(batch.user_fee_increase, 0);
        assert_eq!(batch.signature_public_key_id, 0);
    }

    #[test]
    fn batch_transition_v1_default() {
        let batch = BatchTransitionV1::default();
        assert_eq!(batch.transitions.len(), 0);
        assert_eq!(batch.user_fee_increase, 0);
        assert_eq!(batch.signature_public_key_id, 0);
    }

    // -----------------------------------------------------------------------
    // Accessors: DocumentBatchIterator / DocumentBatchV1Iterator
    // -----------------------------------------------------------------------

    #[test]
    fn document_batch_iterator_v0_yields_document_refs() {
        use crate::state_transition::batch_transition::accessors::DocumentBatchIterator;

        let transitions = vec![make_delete_transition(1), make_delete_transition(2)];
        let mut iter = DocumentBatchIterator::V0(transitions.iter());
        let first = iter.next().unwrap();
        assert!(matches!(first, BatchedTransitionRef::Document(_)));
        let second = iter.next().unwrap();
        assert!(matches!(second, BatchedTransitionRef::Document(_)));
        assert!(iter.next().is_none());
    }

    #[test]
    fn document_batch_v1_iterator_yields_mixed_refs() {
        use crate::state_transition::batch_transition::accessors::{
            DocumentBatchIterator, DocumentBatchV1Iterator,
        };

        let transitions = vec![
            BatchedTransition::Document(make_delete_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ];
        #[allow(clippy::needless_borrows_for_generic_args)]
        let mut iter = DocumentBatchIterator::V1(DocumentBatchV1Iterator {
            inner: transitions.iter(),
        });
        let first = iter.next().unwrap();
        assert!(matches!(first, BatchedTransitionRef::Document(_)));
        let second = iter.next().unwrap();
        assert!(matches!(second, BatchedTransitionRef::Token(_)));
        assert!(iter.next().is_none());
    }

    // -----------------------------------------------------------------------
    // OptionallyAssetLockProved
    // -----------------------------------------------------------------------

    #[test]
    fn batch_transition_optionally_asset_lock_proved() {
        use crate::identity::state_transition::OptionallyAssetLockProved;
        let batch = BatchTransition::V0(make_batch_v0(vec![]));
        // OptionallyAssetLockProved has a default impl returning None
        assert!(batch.optional_asset_lock_proof().is_none());
    }
}
