//! The token market contract's orders (`ask` and `bid`, protocol version 15) through block
//! processing. Only the token marketplace transitions write an order, so a document
//! transition replacing or deleting a stored order is refused by the reject data trigger,
//! even when its owner signs it, and the order stays as stored.

use super::*;

mod token_market_orders_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::create_token_contract_with_owner_identity;
    use dpp::consensus::state::data_trigger::DataTriggerError;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Setters;
    use dpp::document::{Document, DocumentV0};
    use dpp::prelude::DataContract;
    use dpp::state_transition::StateTransition;
    use dpp::system_data_contracts::token_market_contract;
    use dpp::system_data_contracts::token_market_contract::v1::document_types::{ask, bid};
    use drive::util::object_size_info::{
        DocumentAndContractInfo, DocumentInfo::DocumentRefInfo, OwnedDocumentInfo,
    };
    use std::collections::BTreeMap;

    /// An order of `document_type_name` owned by `owner_id`, for 10 units of `token_id` (a
    /// token of 6 decimals) at a price of 5, as the marketplace stores it.
    fn order(
        document_type_name: &str,
        id: u8,
        owner_id: Identifier,
        token_id: Identifier,
    ) -> Document {
        let mut properties = BTreeMap::from([
            (
                "tokenId".to_string(),
                Value::Identifier(token_id.to_buffer()),
            ),
            ("price".to_string(), Value::U64(5)),
            ("amount".to_string(), Value::U64(10)),
            ("remaining".to_string(), Value::U64(10)),
            ("decimals".to_string(), Value::U8(6)),
        ]);
        if document_type_name == bid::NAME {
            properties.insert("escrow".to_string(), Value::U64(50));
        }
        Document::V0(DocumentV0 {
            id: Identifier::from([id; 32]),
            owner_id,
            properties,
            revision: Some(1),
            created_at: Some(0),
            updated_at: Some(0),
            ..Default::default()
        })
    }

    fn stored_orders(
        platform: &TempPlatform<MockCoreRPCLike>,
        market: &DataContract,
        document_type_name: &str,
    ) -> Vec<Document> {
        let query = DriveDocumentQuery::from_sql_expr(
            &format!("select * from {document_type_name}"),
            market,
            Some(&platform.config.drive),
            PlatformVersion::latest(),
        )
        .expect("expected an order query");
        platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected the stored orders")
            .documents()
            .to_vec()
    }

    fn process(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
    ) -> StateTransitionExecutionResult {
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the transition to serialize");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
                &platform.state.load(),
                &BlockInfo::default(),
                &transaction,
                PlatformVersion::latest(),
                false,
                None,
            )
            .expect("expected to process the state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the transaction");
        processing_result.into_execution_results().remove(0)
    }

    /// The owner of a stored ask and a stored bid signs a replace and a delete of each: block
    /// processing refuses every one with the reject trigger's `DataTriggerConditionError` and
    /// leaves both orders as stored.
    #[tokio::test]
    async fn should_refuse_a_signed_replace_or_delete_of_a_stored_order() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (owner, signer, key) = setup_identity(&mut platform, 961, dash_to_credits!(0.5));
        let (issuer, _, _) = setup_identity(&mut platform, 962, dash_to_credits!(0.5));
        let (_, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            issuer.id(),
            Some(|configuration: &mut TokenConfiguration| {
                configuration.conventions_mut().set_decimals(6)
            }),
            None,
            None,
            None,
            platform_version,
        );
        let market = platform
            .drive
            .get_contract_with_fetch_info(
                token_market_contract::ID.to_buffer(),
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the token market contract")
            .expect("expected the token market contract at genesis")
            .contract
            .clone();

        let mut nonce = 1;
        for (id, name) in [(1, ask::NAME), (2, bid::NAME)] {
            let document_type = market
                .document_type_for_name(name)
                .expect("expected the order type");
            let stored = order(name, id, owner.id(), token_id);
            platform
                .drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&stored, None)),
                            owner_id: None,
                        },
                        contract: &market,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to store the order");
            let before = stored_orders(&platform, &market, name);
            assert_eq!(
                before
                    .iter()
                    .map(|document| document.id())
                    .collect::<Vec<_>>(),
                vec![stored.id()]
            );

            let mut replacement = stored.clone();
            replacement.set("remaining", Value::U64(4));
            replacement
                .increment_revision()
                .expect("expected the revision to increment");
            let replace = BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                document_type,
                &key,
                nonce,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            let delete = BatchTransition::new_document_deletion_transition_from_document(
                stored,
                document_type,
                &key,
                nonce + 1,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the delete transition");
            nonce += 2;

            for (action, transition) in [("replace", replace), ("delete", delete)] {
                let result = process(&platform, &transition);
                assert_matches!(
                    result,
                    PaidConsensusError {
                        error: ConsensusError::StateError(StateError::DataTriggerError(
                            DataTriggerError::DataTriggerConditionError(_)
                        )),
                        ..
                    },
                    "{name} {action}: {result:?}"
                );
            }
            assert_eq!(stored_orders(&platform, &market, name), before, "{name}");
        }
    }
}
