//! Multi-block strategy coverage for `requiredSince`: a scheduled contract
//! update adds a new required property mid-run while random document inserts
//! keep firing every block. Documents inserted before the update land stamped
//! with contract version 1 and survive in state; inserts generated from the
//! pre-update schema after it are consensus-rejected for missing the new
//! property. Every block's transitions are proof-verified and the chain's
//! app hashes stay deterministic — the cross-block wiring no scripted test
//! exercises.

#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{FailureStrategy, NetworkStrategy};
    use dash_platform_macros::stack_size;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::document::DocumentV0Getters;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::version::PlatformVersion;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::query::DriveDocumentQuery;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use std::collections::{BTreeMap, HashMap};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_contract_update_adds_required_field_mid_run() {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let mut contract_update = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-add-required-since-field.json",
            2,
            true,
            platform_version,
        )
        .expect("expected to get the updated contract from a json document");
        contract_update.data_contract_mut().set_version(2);

        let contract = created_contract.data_contract();

        // Inserts are generated from the document type captured here — the
        // pre-update schema — every block. Before the update they are valid
        // and get stamped with contract version 1; after it they lack the
        // newly required `country` and must be consensus-rejected.
        let document_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a contactRequest document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(
                    created_contract,
                    Some(BTreeMap::from([(4, contract_update)])),
                )],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: Some(FailureStrategy {
                deterministic_start_seed: None,
                dont_finalize_block: false,
                expect_every_block_errors_with_codes: vec![],
                rounds_before_successful_block: None,
                // From block 5 on, the pre-update generator's documents miss
                // the newly required `country`: JSON-schema rejection (10101)
                expect_specific_block_errors_with_codes: HashMap::from([
                    (5, vec![10101]),
                    (6, vec![10101]),
                    (7, vec![10101]),
                    (8, vec![10101]),
                ]),
            }),
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 8, strategy, config, 15, &mut None, &mut None)
                .await;

        // The runner re-derives the contract id at deployment, so read it
        // back from the outcome's strategy
        let contract_id = outcome
            .strategy
            .strategy
            .start_contracts
            .first()
            .expect("expected the start contract")
            .0
            .data_contract()
            .id();

        // The scheduled update landed: the stored contract is at version 2
        let fetched_contract = outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(contract_id.to_buffer(), None, None, None, platform_version)
            .unwrap()
            .expect("expected to fetch the contract")
            .expect("expected the contract to exist");
        assert_eq!(
            fetched_contract.contract.version(),
            2,
            "the scheduled requiredSince update must have been applied"
        );

        // Every surviving contactRequest document predates the update: stamped with
        // contract version 1 and stored without the new required property —
        // grandfathered rows living under the version-2 schema
        let query = DriveDocumentQuery::from_sql_expr(
            "select * from contactRequest",
            &fetched_contract.contract,
            None,
            platform_version,
        )
        .expect("expected a document query");
        let documents = outcome
            .abci_app
            .platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected to query documents")
            .documents()
            .to_vec();

        assert!(
            !documents.is_empty(),
            "documents inserted before the update must survive it"
        );
        for document in &documents {
            assert_eq!(
                document.contract_version(),
                Some(1),
                "every surviving document predates the update and must be stamped 1"
            );
            assert!(
                !document.properties().contains_key("country"),
                "grandfathered documents must not carry the new property"
            );
        }
    }
}
