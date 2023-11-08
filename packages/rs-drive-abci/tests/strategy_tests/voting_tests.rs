

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{NetworkStrategy, Strategy};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use tenderdash_abci::proto::types::CoreChainLock;
    use dpp::data_contract::document_type::random_document::{DocumentFieldFillSize, DocumentFieldFillType};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::platform_value::Value;
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::transitions::create_state_transitions_for_identities;

    #[test]
    fn run_chain_block_two_state_transitions_conflicting_unique_index() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the dpns contract and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            quorum_size: 100,
            execution: ExecutionConfig {
                //we disable document triggers because we are using dpns and dpns needs a preorder
                use_document_triggers: false,
                validator_set_quorum_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let platform_version = PlatformVersion::latest();

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (identity1, keys) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
                .unwrap();

        simple_signer.add_keys(keys);

        let (identity2, keys) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
                .unwrap();

        simple_signer.add_keys(keys);

        let start_identities = create_state_transitions_for_identities(
            vec![identity1, identity2],
            &mut simple_signer,
            &mut rng,
            platform_version,
        );

        let document_op_1 = DocumentOp {
            contract: platform.drive.system_contracts.dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([(
                            "dashUniqueIdentityId",
                            Value::from(start_identities.first().unwrap().0.id()),
                        )])
                            .into(),
                    ),
                ]),
                Some(start_identities.first().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: platform
                .drive
                .system_contracts
                .dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_op_2 = DocumentOp {
            contract: platform.drive.system_contracts.dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([(
                            "dashUniqueIdentityId",
                            Value::from(start_identities.last().unwrap().0.id()),
                        )])
                            .into(),
                    ),
                ]),
                Some(start_identities.last().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: platform
                .drive
                .system_contracts
                .dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_op_1),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,

                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_op_2),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities,
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
                signer: Some(simple_signer),
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,

        };

        let mut core_block_heights = vec![10, 11];

        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                let core_block_height = if core_block_heights.len() == 1 {
                    *core_block_heights.first().unwrap()
                } else {
                    core_block_heights.remove(0)
                };
                Ok(CoreChainLock {
                    core_block_height,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        // On the first block we only have identities and contracts
        let outcome =
            run_chain_for_strategy(&mut platform, 2, strategy.clone(), config.clone(), 15);

        let state_transitions_block_2 = &outcome
            .state_transition_results_per_block
            .get(&2)
            .expect("expected to get block 2");

        let first_document_insert_result = &state_transitions_block_2
            .first()
            .as_ref()
            .expect("expected a document insert")
            .1;
        assert_eq!(first_document_insert_result.code, 0);

        let second_document_insert_result = &state_transitions_block_2
            .get(1)
            .as_ref()
            .expect("expected a document insert")
            .1;

        assert_eq!(second_document_insert_result.code, 4009); // we expect an error
    }
}