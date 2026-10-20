//! An identity create carries its identity id outside the bytes the asset lock key signs. From
//! protocol version 14 a block refuses, unpaid, one whose identity id is not the one its asset
//! lock derives, so a validator rejects a proposal holding it rather than letting it consume the
//! asset lock.
#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{Network, PrivateKey};
    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
    use dpp::native_bls::NativeBlsModule;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
    use tenderdash_abci::proto::abci::RequestProcessProposal;
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::version::Consensus;
    use tenderdash_abci::Application;

    const BLOCK_SPACING_MS: u64 = 3000;

    fn strategy() -> NetworkStrategy {
        NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 10,
            chain_lock_quorum_count: 10,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        }
    }

    fn config() -> PlatformConfig {
        PlatformConfig {
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: BLOCK_SPACING_MS,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        }
    }

    fn timestamp(time_ms: u64) -> Timestamp {
        Timestamp {
            seconds: (time_ms / 1000) as i64,
            nanos: ((time_ms % 1000) * 1_000_000) as i32,
        }
    }

    /// A valid identity create funded by an instant asset lock, with its identity id changed
    /// when `change_identity_id` is set. The identity id is not part of the signable bytes, so
    /// the asset lock signature still verifies either way.
    async fn identity_create(change_identity_id: bool) -> Vec<u8> {
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();
        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");
        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(999),
            platform_version,
        )
        .expect("expected to get key pair");
        signer.add_identity_public_key(key.clone(), private_key);

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .expect("expected to get key pair");
        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).expect("expected a key")),
            None,
        );

        let identity: Identity = IdentityV0 {
            id: asset_lock_proof
                .create_identifier()
                .expect("expected an identifier"),
            public_keys: BTreeMap::from([(0, master_key), (1, key)]),
            balance: 0,
            revision: 0,
        }
        .into();

        let mut transition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        if change_identity_id {
            let StateTransition::IdentityCreate(IdentityCreateTransition::V0(v0)) = &mut transition
            else {
                panic!("expected an identity create v0 transition");
            };
            let mut identity_id = v0.identity_id.to_buffer();
            identity_id[0] ^= 0x01;
            v0.identity_id = Identifier::new(identity_id);
        }

        transition
            .serialize_to_bytes()
            .expect("expected to serialize")
    }

    /// The next block, from a proposer other than this node, holding `txs`.
    fn next_block_from_another_proposer(
        outcome: &ChainExecutionOutcome,
        txs: Vec<Vec<u8>>,
    ) -> RequestProcessProposal {
        let platform_state = outcome.abci_app.platform.state.load();
        let protocol_version = PlatformVersion::latest().protocol_version as u64;
        RequestProcessProposal {
            txs,
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: [0x42u8; 32].to_vec(),
            height: (platform_state.last_committed_block_height() + 1) as i64,
            time: Some(timestamp(outcome.end_time_ms + BLOCK_SPACING_MS)),
            next_validators_hash: [0u8; 32].to_vec(),
            round: 0,
            core_chain_locked_height: platform_state.last_committed_core_height(),
            core_chain_lock_update: None,
            proposer_pro_tx_hash: outcome.proposers[1].pro_tx_hash().to_byte_array().to_vec(),
            proposed_app_version: protocol_version,
            version: Some(Consensus {
                block: 0,
                app: protocol_version,
            }),
            quorum_hash: outcome
                .current_quorum()
                .quorum_hash
                .to_byte_array()
                .to_vec(),
        }
    }

    /// Runs a fresh test chain and has it process a proposal holding only `transition`.
    async fn process_proposal_holding(transition: Vec<u8>) -> i32 {
        let config = config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let outcome = run_chain_for_strategy(
            &mut platform,
            5,
            strategy(),
            config,
            7,
            &mut None,
            &mut None,
        )
        .await;

        outcome
            .abci_app
            .process_proposal(next_block_from_another_proposer(&outcome, vec![transition]))
            .expect("expected to process the proposal")
            .status
    }

    #[tokio::test]
    async fn should_reject_a_proposal_holding_an_identity_create_whose_identity_id_does_not_match_its_asset_lock(
    ) {
        // The same proposal with the identity id the asset lock derives is accepted, so the
        // rejection comes from the identity id alone
        assert_eq!(
            process_proposal_holding(identity_create(false).await).await,
            ProposalStatus::Accept as i32
        );

        assert_eq!(
            process_proposal_holding(identity_create(true).await).await,
            ProposalStatus::Reject as i32
        );
    }
}
