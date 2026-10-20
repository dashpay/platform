#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;
    use crate::strategy::NetworkStrategy;
    use dash_platform_macros::stack_size;
    use dpp::dash_to_credits;
    use dpp::dash_to_duffs;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Txid;
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        AssetUnlockStatus, AssetUnlockStatusResult, QuorumType,
    };
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::document_type::v0::random_document_type::{
        FieldMinMaxBounds, FieldTypeWeights, RandomDocumentTypeParameters,
    };
    use dpp::data_contract::DataContract;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::tokens::token_event::TokenEvent;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::DocumentAction::DocumentActionReplaceRandom;
    use strategy_tests::operations::{
        DocumentAction, DocumentOp, IdentityUpdateOp, Operation, OperationType, TokenOp,
    };
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    /// The one chain simulation that runs on every pull request.
    ///
    /// The rest of the strategy suite is excluded from the PR path by the
    /// nextest filter in `.github/workflows/tests-rs-workspace.yml` (it
    /// selects this test by name — keep `comprehensive_mixed_operations` in
    /// the name if renaming) and runs on push and nightly instead. This test
    /// therefore packs as many subsystems as compose deterministically into
    /// one run: identity creation per block, top-ups, key additions, credit
    /// withdrawals, credit transfers, document create/replace/delete on
    /// dashpay, random contract creation, token minting, address funding
    /// from core asset locks, address-to-address transfers, identity top-ups
    /// from address balances, random core height increases with validator
    /// quorum rotation, and two epoch changes with masternode payouts —
    /// with per-transition proof verification and sum-tree verification on.
    ///
    /// Not covered here (nightly/push strategy tests still cover them):
    /// shielded operations (separate CI phase), contested-resource voting,
    /// protocol upgrades, failure injection, and masternode list mutations.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_comprehensive_mixed_operations_with_epoch_change_and_quorum_rotation() {
        let platform_version = PlatformVersion::latest();

        // A single hard-coded start identity: contracts deploy at block 2,
        // when the only identities present are the start identities, so both
        // contracts below deterministically get this identity as owner. That
        // is what lets the token contract's final id (and therefore the
        // token id the mint operation targets) be precomputed.
        let mut rng = StdRng::seed_from_u64(792);
        let mut simple_signer = SimpleSigner::default();
        let (mut identity, keys) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(3, &mut rng, platform_version)
        .expect("expected a random identity");
        simple_signer.add_identity_public_keys(keys);

        // The generated main keys are all authentication-purpose; withdrawal
        // and credit-transfer operations sign with a critical TRANSFER key,
        // so add one explicitly (inserted identities get theirs via
        // `extra_keys` below).
        let (transfer_key, transfer_private_key) =
            IdentityPublicKey::random_key_with_known_attributes(
                3,
                &mut rng,
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
                KeyType::ECDSA_SECP256K1,
                None,
                platform_version,
            )
            .expect("expected a transfer key");
        identity.add_public_key(transfer_key.clone());
        simple_signer.add_identity_public_key(transfer_key, transfer_private_key);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity],
                &(dash_to_duffs!(10)..=dash_to_duffs!(10)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dashpay_created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get dashpay contract from a json document");
        let dashpay_contract = dashpay_created_contract.data_contract().clone();

        let mut token_created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/basic-token/basic-token.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get token contract from a json document");

        let token_contract = token_created_contract.data_contract_mut();
        token_contract
            .token_configuration_mut(0)
            .expect("expected to get token configuration")
            .distribution_rules_mut()
            .set_minting_allow_choosing_destination(true);
        token_contract.set_owner_id(identity.id());
        // Contracts deploy in `start_contracts` order, each bumping the
        // owner's identity nonce: the dashpay contract takes nonce 1, this
        // one nonce 2. With a single start identity the id set here is
        // exactly the id deployment will assign, so the token op below stays
        // bound to the deployed token.
        token_contract.set_id(DataContract::generate_data_contract_id_v0(identity.id(), 2));
        let token_id = token_contract
            .token_id(0)
            .expect("expected to get token id");
        let token_op_contract = token_contract.clone();

        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .expect("expected a contactRequest document type")
            .to_owned_document_type();

        let operations = vec![
            Operation {
                op_type: OperationType::Document(DocumentOp {
                    contract: dashpay_contract.clone(),
                    action: DocumentAction::DocumentActionInsertRandom(
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: contact_request_document_type.clone(),
                }),
                frequency: Frequency {
                    times_per_block_range: 1..10,
                    chance_per_block: None,
                },
            },
            Operation {
                op_type: OperationType::Document(DocumentOp {
                    contract: dashpay_contract.clone(),
                    action: DocumentActionReplaceRandom,
                    document_type: contact_request_document_type.clone(),
                }),
                frequency: Frequency {
                    times_per_block_range: 1..4,
                    chance_per_block: Some(0.7),
                },
            },
            Operation {
                op_type: OperationType::Document(DocumentOp {
                    contract: dashpay_contract.clone(),
                    action: DocumentAction::DocumentActionDelete,
                    document_type: contact_request_document_type,
                }),
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.7),
                },
            },
            Operation {
                op_type: OperationType::IdentityTopUp(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: None,
                },
            },
            Operation {
                op_type: OperationType::IdentityUpdate(IdentityUpdateOp::IdentityUpdateAddKeys(2)),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.5),
                },
            },
            Operation {
                op_type: OperationType::IdentityWithdrawal(
                    dash_to_credits!(0.1)..=dash_to_credits!(0.1),
                ),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.5),
                },
            },
            Operation {
                op_type: OperationType::IdentityTransfer(None),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.5),
                },
            },
            Operation {
                op_type: OperationType::Token(TokenOp {
                    contract: token_op_contract,
                    token_id,
                    token_pos: 0,
                    use_identity_with_id: Some(identity.id()),
                    action: TokenEvent::Mint(1000, identity.id(), None),
                }),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            },
            Operation {
                op_type: OperationType::ContractCreate(
                    RandomDocumentTypeParameters {
                        new_fields_optional_count_range: 1..5,
                        new_fields_required_count_range: 1..5,
                        new_indexes_count_range: 1..3,
                        field_weights: FieldTypeWeights {
                            string_weight: 50,
                            float_weight: 50,
                            integer_weight: 50,
                            date_weight: 50,
                            boolean_weight: 20,
                            byte_array_weight: 70,
                        },
                        field_bounds: FieldMinMaxBounds {
                            string_min_len: 1..10,
                            string_has_min_len_chance: 0.5,
                            string_max_len: 10..63,
                            string_has_max_len_chance: 0.5,
                            integer_min: 1..10,
                            integer_has_min_chance: 0.5,
                            integer_max: 10..10000,
                            integer_has_max_chance: 0.5,
                            float_min: 0.1..10.0,
                            float_has_min_chance: 0.5,
                            float_max: 10.0..1000.0,
                            float_has_max_chance: 0.5,
                            date_min: 0,
                            date_max: 0,
                            byte_array_min_len: 1..10,
                            byte_array_has_min_len_chance: 0.0,
                            byte_array_max_len: 10..255,
                            byte_array_has_max_len_chance: 0.0,
                        },
                        keep_history_chance: 0.5,
                        documents_mutable_chance: 0.5,
                        documents_can_be_deleted_chance: 0.5,
                    },
                    1..3,
                ),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.3),
                },
            },
            Operation {
                op_type: OperationType::AddressFundingFromCoreAssetLock(
                    dash_to_credits!(20)..=dash_to_credits!(20),
                ),
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: None,
                },
            },
            Operation {
                op_type: OperationType::AddressTransfer(
                    dash_to_credits!(5)..=dash_to_credits!(5),
                    1..=4,
                    Some(0.2),
                    None,
                ),
                frequency: Frequency {
                    times_per_block_range: 0..2,
                    chance_per_block: Some(0.5),
                },
            },
            Operation {
                op_type: OperationType::IdentityTopUpFromAddresses(
                    dash_to_credits!(1)..=dash_to_credits!(3),
                ),
                frequency: Frequency {
                    times_per_block_range: 0..2,
                    chance_per_block: Some(0.4),
                },
            },
        ];

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![
                    (dashpay_created_contract, None),
                    (token_created_contract, None),
                ],
                operations,
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..6,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: [(
                        Purpose::TRANSFER,
                        [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                    )]
                    .into(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },
                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
            },
            total_hpmns: 120,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            sign_instant_locks: true,
            ..Default::default()
        };

        let two_days_in_ms = 1000 * 60 * 60 * 24 * 2;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: two_days_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 30;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        // The withdrawal queue broadcasts asset-unlock transactions to core
        // and later polls their status. Accept every broadcast and report
        // every status as still unknown — that keeps the queue, broadcast,
        // and rebroadcast paths exercised without simulating core-side
        // confirmation (the withdrawal strategy tests cover the full
        // lifecycle).
        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));
        platform
            .core_rpc
            .expect_get_asset_unlock_statuses()
            .returning(move |indices, _| {
                Ok(indices
                    .iter()
                    .map(|index| AssetUnlockStatusResult {
                        index: *index,
                        status: AssetUnlockStatus::Unknown,
                    })
                    .collect())
            });

        let outcome = run_chain_for_strategy(
            &mut platform,
            block_count,
            strategy,
            config,
            40,
            &mut None,
            &mut None,
        )
        .await;

        // The exact counts are deterministic for the fixed seeds above; a
        // change here means block execution behavior changed and should be
        // understood, not just re-pinned.
        assert!(
            outcome.identities.len() > 20,
            "expected the simulation to create identities, got {}",
            outcome.identities.len()
        );
        assert_eq!(outcome.masternode_identity_balances.len(), 120);
        let paid_masternodes = outcome
            .masternode_identity_balances
            .iter()
            .filter(|(_, balance)| **balance != 0)
            .count();
        assert!(
            paid_masternodes > 0,
            "expected epoch changes to pay proposers"
        );

        let issues = outcome
            .abci_app
            .platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to be able to verify grovedb");
        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}
