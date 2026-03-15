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
    use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
    use crate::state_transition::batch_transition::{
        BatchTransitionV0, BatchTransitionV1,
    };
    use crate::state_transition::StateTransitionLike;
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
    fn v0_transitions_are_empty_when_no_transitions() {
        let batch = make_batch_v0(vec![]);
        assert!(batch.transitions_are_empty());
        assert_eq!(batch.transitions_len(), 0);
    }

    #[test]
    fn v0_first_transition_returns_some_and_none() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        assert!(batch.first_transition().is_some());

        let empty = make_batch_v0(vec![]);
        assert!(empty.first_transition().is_none());
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
    fn v0_unique_identifiers_format() {
        let batch = make_batch_v0(vec![make_delete_transition(1)]);
        let ids = batch.unique_identifiers();
        assert_eq!(ids.len(), 1);
        // Should contain base64-encoded owner_id and data_contract_id and hex nonce
        assert!(ids[0].contains('-'));
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
    fn v1_transitions_are_empty() {
        let batch = make_batch_v1(vec![]);
        assert!(batch.transitions_are_empty());
        assert_eq!(batch.transitions_len(), 0);
    }

    #[test]
    fn v1_first_transition_document_and_token() {
        let batch_doc = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        assert!(matches!(
            batch_doc.first_transition().unwrap(),
            BatchedTransitionRef::Document(_)
        ));

        let batch_tok = make_batch_v1(vec![BatchedTransition::Token(make_token_burn_transition(
            1, 100,
        ))]);
        assert!(matches!(
            batch_tok.first_transition().unwrap(),
            BatchedTransitionRef::Token(_)
        ));
    }

    #[test]
    fn v1_first_transition_mut_variants() {
        let mut batch_doc =
            make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        assert!(matches!(
            batch_doc.first_transition_mut().unwrap(),
            BatchedTransitionMutRef::Document(_)
        ));

        let mut batch_tok = make_batch_v1(vec![BatchedTransition::Token(
            make_token_burn_transition(1, 100),
        )]);
        assert!(matches!(
            batch_tok.first_transition_mut().unwrap(),
            BatchedTransitionMutRef::Token(_)
        ));

        let mut empty = make_batch_v1(vec![]);
        assert!(empty.first_transition_mut().is_none());
    }

    #[test]
    fn v1_contains_document_and_token_transition() {
        let doc_batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        assert!(doc_batch.contains_document_transition());
        assert!(!doc_batch.contains_token_transition());

        let tok_batch = make_batch_v1(vec![BatchedTransition::Token(make_token_burn_transition(
            1, 100,
        ))]);
        assert!(!tok_batch.contains_document_transition());
        assert!(tok_batch.contains_token_transition());
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
    // BatchTransitionV1: StateTransitionIdentitySigned (purpose_requirement)
    // -----------------------------------------------------------------------

    #[test]
    fn v1_security_level_requirement_transfer() {
        use crate::identity::Purpose;
        use crate::state_transition::StateTransitionIdentitySigned;
        let batch = make_batch_v1(vec![]);
        let levels = batch.security_level_requirement(Purpose::TRANSFER);
        assert_eq!(levels, vec![SecurityLevel::CRITICAL]);
    }

    #[test]
    fn v1_security_level_requirement_authentication() {
        use crate::identity::Purpose;
        use crate::state_transition::StateTransitionIdentitySigned;
        let batch = make_batch_v1(vec![]);
        let levels = batch.security_level_requirement(Purpose::AUTHENTICATION);
        assert!(levels.contains(&SecurityLevel::CRITICAL));
        assert!(levels.contains(&SecurityLevel::HIGH));
        assert!(levels.contains(&SecurityLevel::MEDIUM));
    }

    #[test]
    fn v1_purpose_requirement_single_token_transfer() {
        use crate::identity::Purpose;
        use crate::state_transition::StateTransitionIdentitySigned;
        let batch = make_batch_v1(vec![BatchedTransition::Token(
            make_token_transfer_transition(1),
        )]);
        let purposes = batch.purpose_requirement();
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION, Purpose::TRANSFER]);
    }

    #[test]
    fn v1_purpose_requirement_single_token_claim() {
        use crate::identity::Purpose;
        use crate::state_transition::StateTransitionIdentitySigned;
        let batch = make_batch_v1(vec![BatchedTransition::Token(make_token_claim_transition(
            1,
        ))]);
        let purposes = batch.purpose_requirement();
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION, Purpose::TRANSFER]);
    }

    #[test]
    fn v1_purpose_requirement_multiple_transitions_no_transfer() {
        use crate::identity::Purpose;
        use crate::state_transition::StateTransitionIdentitySigned;
        let batch = make_batch_v1(vec![
            BatchedTransition::Token(make_token_transfer_transition(1)),
            BatchedTransition::Token(make_token_burn_transition(2, 100)),
        ]);
        let purposes = batch.purpose_requirement();
        // With more than 1 transition, purpose_requirement returns AUTHENTICATION only
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION]);
    }

    #[test]
    fn v1_purpose_requirement_default_document() {
        use crate::identity::Purpose;
        use crate::state_transition::StateTransitionIdentitySigned;
        let batch = make_batch_v1(vec![BatchedTransition::Document(make_delete_transition(1))]);
        let purposes = batch.purpose_requirement();
        assert_eq!(purposes, vec![Purpose::AUTHENTICATION]);
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
    fn batch_transition_field_types_binary_and_identifier_paths() {
        use crate::state_transition::batch_transition::BatchTransition;
        use crate::state_transition::StateTransitionFieldTypes;
        let paths = BatchTransition::binary_property_paths();
        assert!(!paths.is_empty());
        assert!(paths.contains(&"signature"));

        let id_paths = BatchTransition::identifiers_property_paths();
        assert!(id_paths.contains(&"ownerId"));

        let sig_paths = BatchTransition::signature_property_paths();
        assert!(sig_paths.contains(&"signature"));
        assert!(sig_paths.contains(&"signaturePublicKeyId"));
    }

    // -----------------------------------------------------------------------
    // DocumentTransitionActionType
    // -----------------------------------------------------------------------

    #[test]
    fn document_action_type_getter() {
        let create = DocumentTransition::Create(DocumentCreateTransition::default());
        assert_eq!(create.action_type(), DocumentTransitionActionType::Create);

        let delete = make_delete_transition(1);
        assert_eq!(delete.action_type(), DocumentTransitionActionType::Delete);

        let purchase = make_purchase_transition(1, 100);
        assert_eq!(
            purchase.action_type(),
            DocumentTransitionActionType::Purchase
        );
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
    fn batched_transition_borrow_as_ref() {
        let doc = BatchedTransition::Document(make_delete_transition(1));
        assert!(matches!(
            doc.borrow_as_ref(),
            BatchedTransitionRef::Document(_)
        ));

        let tok = BatchedTransition::Token(make_token_burn_transition(1, 100));
        assert!(matches!(
            tok.borrow_as_ref(),
            BatchedTransitionRef::Token(_)
        ));
    }

    #[test]
    fn batched_transition_borrow_as_mut() {
        let mut doc = BatchedTransition::Document(make_delete_transition(1));
        assert!(matches!(
            doc.borrow_as_mut(),
            BatchedTransitionMutRef::Document(_)
        ));

        let mut tok = BatchedTransition::Token(make_token_burn_transition(1, 100));
        assert!(matches!(
            tok.borrow_as_mut(),
            BatchedTransitionMutRef::Token(_)
        ));
    }

    #[test]
    fn batched_transition_set_identity_contract_nonce() {
        let mut doc = BatchedTransition::Document(make_delete_transition(1));
        doc.set_identity_contract_nonce(123);
        use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
        if let BatchedTransition::Document(d) = &doc {
            assert_eq!(d.identity_contract_nonce(), 123);
        }

        let mut tok = BatchedTransition::Token(make_token_burn_transition(1, 100));
        tok.set_identity_contract_nonce(456);
        use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
        if let BatchedTransition::Token(t) = &tok {
            assert_eq!(t.identity_contract_nonce(), 456);
        }
    }

    // -----------------------------------------------------------------------
    // BatchedTransitionRef: to_owned_transition, identity_contract_nonce, data_contract_id
    // -----------------------------------------------------------------------

    #[test]
    fn batched_transition_ref_to_owned() {
        let doc = make_delete_transition(1);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        assert!(matches!(
            bt_ref.to_owned_transition(),
            BatchedTransition::Document(_)
        ));

        let tok = make_token_burn_transition(1, 100);
        let bt_ref = BatchedTransitionRef::Token(&tok);
        assert!(matches!(
            bt_ref.to_owned_transition(),
            BatchedTransition::Token(_)
        ));
    }

    #[test]
    fn batched_transition_ref_identity_contract_nonce() {
        let doc = make_delete_transition(42);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        assert_eq!(bt_ref.identity_contract_nonce(), 42);
    }

    #[test]
    fn batched_transition_ref_data_contract_id() {
        let doc = make_delete_transition(1);
        let bt_ref = BatchedTransitionRef::Document(&doc);
        assert_eq!(bt_ref.data_contract_id(), Identifier::new([0xAA; 32]));

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
    fn batched_transition_resolvers_token_variants() {
        let burn = BatchedTransition::Token(make_token_burn_transition(1, 100));
        assert!(burn.as_transition_token_burn().is_some());
        assert!(burn.as_transition_token_mint().is_none());
        assert!(burn.as_transition_token_transfer().is_none());

        let transfer = BatchedTransition::Token(make_token_transfer_transition(1));
        assert!(transfer.as_transition_token_transfer().is_some());
        assert!(transfer.as_transition_token_burn().is_none());

        let claim = BatchedTransition::Token(make_token_claim_transition(1));
        assert!(claim.as_transition_token_claim().is_some());
        assert!(claim.as_transition_token_burn().is_none());
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
    fn batched_transition_ref_resolvers_token_variants() {
        let burn = make_token_burn_transition(1, 100);
        let bt_ref = BatchedTransitionRef::Token(&burn);
        assert!(bt_ref.as_transition_token_burn().is_some());
        assert!(bt_ref.as_transition_create().is_none());

        let transfer = make_token_transfer_transition(1);
        let bt_ref = BatchedTransitionRef::Token(&transfer);
        assert!(bt_ref.as_transition_token_transfer().is_some());

        let claim = make_token_claim_transition(1);
        let bt_ref = BatchedTransitionRef::Token(&claim);
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
}
