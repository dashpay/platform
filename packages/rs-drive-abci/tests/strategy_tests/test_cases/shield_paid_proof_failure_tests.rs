#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::dash_to_credits;
    use dpp::dashcore::hashes::Hash;
    use dpp::identity::signer::Signer;
    use dpp::serialization::Signable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shield_transition::v1::ShieldTransitionV1;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::StateTransition;
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::mimic::MimicExecuteBlockOptions;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    #[tokio::test]
    async fn should_commit_a_paid_shield_proof_failure_after_independent_proposal_validation() {
        let pv = PlatformVersion::latest();
        let config = PlatformConfig {
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::AddressFundingFromCoreAssetLock(
                        dash_to_credits!(1.0)..=dash_to_credits!(1.0),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            validator_quorum_count: 10,
            chain_lock_quorum_count: 10,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let outcome =
            run_chain_for_strategy(&mut platform, 5, strategy, config, 7, &mut None, &mut None)
                .await;
        let (&address, _) = outcome
            .addresses_with_balance
            .addresses_with_balance
            .first_key_value()
            .expect("funded address");
        let (nonce, balance) = outcome
            .abci_app
            .platform
            .drive
            .fetch_balance_and_nonce(&address, None, pv)
            .expect("funded address balance")
            .expect("persisted funded address");
        let mut transition = StateTransition::Shield(ShieldTransition::V1(ShieldTransitionV1 {
            inputs: BTreeMap::from([(address, (nonce + 1, balance))]),
            actions: vec![SerializedAction {
                nullifier: [1; 32],
                rk: [2; 32],
                cmx: [3; 32],
                encrypted_note: vec![4; 216],
                cv_net: [5; 32],
                spend_auth_sig: [6; 64],
            }],
            amount: 1000,
            anchor: [42; 32],
            proof: vec![0; 100],
            binding_signature: [0; 64],
            fee_strategy: AddressFundsFeeStrategy::from(vec![
                AddressFundsFeeStrategyStep::DeductFromInput(0),
            ]),
            user_fee_increase: 0,
            input_witnesses: vec![],
        }));
        let witness = outcome
            .signer
            .sign_create_witness(&address, &transition.signable_bytes().expect("signable"))
            .await
            .expect("witness");
        let StateTransition::Shield(ShieldTransition::V1(ref mut shield)) = transition else {
            unreachable!();
        };
        shield.input_witnesses = vec![witness];
        let state = outcome.abci_app.platform.state.load();
        let block_info = BlockInfo {
            height: state.last_committed_block_height() + 1,
            core_height: state.last_committed_core_height(),
            time_ms: outcome.end_time_ms + 3000,
            epoch: Epoch::new(outcome.end_epoch_index).expect("epoch"),
        };
        drop(state);
        let block = outcome
            .abci_app
            .mimic_execute_block(
                outcome.proposers[0].pro_tx_hash().to_byte_array(),
                outcome.current_quorum(),
                pv.protocol_version,
                block_info,
                0,
                &[40902],
                false,
                vec![transition],
                MimicExecuteBlockOptions {
                    dont_finalize_block: false,
                    rounds_before_finalization: None,
                    max_tx_bytes_per_block: 40000,
                    independent_process_proposal_verification: true,
                },
            )
            .expect("paid failure must survive preparation and independent validation");
        assert_eq!(
            block.state_transaction_results.len(),
            1,
            "failure stays in the block"
        );
        let result = &block.state_transaction_results[0].1;
        assert_eq!(result.code, 40902);
        let (committed_nonce, committed_balance) = outcome
            .abci_app
            .platform
            .drive
            .fetch_balance_and_nonce(&address, None, pv)
            .expect("address")
            .expect("persisted address");
        assert_eq!(committed_nonce, nonce + 1);
        assert!(committed_balance < balance);
        assert!(
            balance - committed_balance
                >= pv
                    .drive_abci
                    .validation_and_processing
                    .penalties
                    .shielded_proof_verification_failure
        );
        assert_eq!(
            outcome
                .abci_app
                .platform
                .drive
                .read_shielded_pool_total_balance(None, &mut vec![], pv)
                .expect("pool"),
            0
        );
        assert_eq!(
            outcome
                .abci_app
                .platform
                .state
                .load()
                .last_committed_block_height(),
            block_info.height
        );
    }
}
