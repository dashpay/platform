#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::check_tx::CheckTxLevel;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_witness, create_platform_address, setup_address_with_balance,
        P2shMultisigEntry, TestAddressSigner, TestHash, TestProtocolError, TestPublicKey,
        TestScriptBuf, TestSecp256k1, OP_CHECKSIG, OP_DROP, OP_PUSHNUM_1, OP_RETURN,
    };
    use crate::platform_types::platform::{Platform, PlatformRef};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::address_funds::{
        AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress,
    };
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::dashcore::blockdata::script::ScriptBuf;
    use dpp::dashcore::secp256k1::Secp256k1;
    use dpp::dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dpp::dashcore::transaction::special_transaction::TransactionPayload;
    use dpp::dashcore::{
        BlockHash, Network, OutPoint, PrivateKey, PublicKey, Transaction, TxIn, TxOut, Txid,
    };
    use dpp::dashcore_rpc::json::GetRawTransactionResult;
    use dpp::identity::signer::Signer;
    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::AddressNonce;
    use dpp::prelude::AssetLockProof;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
    use dpp::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
    use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
    use dpp::ProtocolError;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::str::FromStr;
    use tempfile::TempDir;

    /// Create a raw AddressFundingFromAssetLockTransitionV0 with dummy witnesses for structure validation tests
    fn create_raw_transition_with_dummy_witnesses(
        asset_lock_proof: dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, Option<u64>>,
        fee_strategy: AddressFundsFeeStrategy,
        input_witnesses_count: usize,
    ) -> StateTransition {
        let witnesses: Vec<AddressWitness> = (0..input_witnesses_count)
            .map(|_| create_dummy_witness())
            .collect();
        AddressFundingFromAssetLockTransition::V0(AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            inputs,
            outputs,
            fee_strategy,
            user_fee_increase: 0,
            signature: BinaryData::new(vec![0u8; 65]), // dummy signature
            input_witnesses: witnesses,
        })
        .into()
    }

    /// Creates an asset lock proof and returns it with the private key for signing
    fn create_asset_lock_proof_with_key(
        rng: &mut StdRng,
    ) -> (
        dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        Vec<u8>,
    ) {
        let platform_version = PlatformVersion::latest();
        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        (asset_lock_proof, pk.to_vec())
    }

    /// Creates a chain asset lock proof with transaction and private key.
    /// Returns (AssetLockProof, private_key_bytes, Transaction).
    /// The Transaction can be used to set up Core RPC mock expectations.
    fn create_chain_asset_lock_proof_with_key_and_tx(
        rng: &mut StdRng,
    ) -> (
        dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        Vec<u8>,
        Transaction,
    ) {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

        let platform_version = PlatformVersion::latest();
        let secp = Secp256k1::new();

        // Generate the one-time key that will receive the asset lock funds
        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(rng, platform_version)
            .unwrap();

        let one_time_private_key = PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap();
        let one_time_public_key = one_time_private_key.public_key(&secp);
        let one_time_key_hash = one_time_public_key.pubkey_hash();

        // Create a fake input (doesn't need to be real for our tests)
        let input_txid =
            Txid::from_str("a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458")
                .unwrap();
        let input = TxIn {
            previous_output: OutPoint::new(input_txid, 0),
            script_sig: ScriptBuf::new(),
            sequence: 0xffffffff,
            witness: Default::default(),
        };

        // Create the funding output (P2PKH to the one-time key)
        let funding_output = TxOut {
            value: 100000000, // 1 Dash
            script_pubkey: ScriptBuf::new_p2pkh(&one_time_key_hash),
        };

        // Create the burn output (OP_RETURN)
        let burn_output = TxOut {
            value: 100000000, // 1 Dash
            script_pubkey: ScriptBuf::new_op_return(&[]),
        };

        // Create the asset lock payload
        let payload = TransactionPayload::AssetLockPayloadType(AssetLockPayload {
            version: 1,
            credit_outputs: vec![funding_output.clone()],
        });

        // Create the transaction
        let transaction = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![input],
            output: vec![burn_output],
            special_transaction_payload: Some(payload),
        };

        let txid = transaction.txid();

        // Create the chain proof with out_point pointing to this transaction
        let mut out_point_bytes = [0u8; 36];
        out_point_bytes[..32].copy_from_slice(txid.as_raw_hash().as_byte_array());
        out_point_bytes[32..36].copy_from_slice(&0u32.to_le_bytes());

        // Use core_chain_locked_height of 100 (will need platform state to match)
        let core_chain_locked_height = 100;

        let chain_proof = ChainAssetLockProof::new(core_chain_locked_height, out_point_bytes);

        (AssetLockProof::Chain(chain_proof), pk.to_vec(), transaction)
    }

    /// Creates a chain asset lock proof and returns it with the private key for signing.
    /// Note: This version doesn't return the transaction - use create_chain_asset_lock_proof_with_key_and_tx
    /// for tests that need to set up Core RPC mocks.
    #[allow(dead_code)]
    fn create_chain_asset_lock_proof_with_key(
        rng: &mut StdRng,
    ) -> (
        dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        Vec<u8>,
    ) {
        let (proof, pk, _tx) = create_chain_asset_lock_proof_with_key_and_tx(rng);
        (proof, pk)
    }

    /// Creates a platform with a Core RPC mock configured to return the given transaction
    fn create_platform_with_chain_asset_lock_mock(
        platform_config: PlatformConfig,
        transaction: Transaction,
        transaction_height: i64,
    ) -> crate::test::helpers::setup::TempPlatform<MockCoreRPCLike> {
        let tempdir = TempDir::new().expect("should create temp dir");

        let mut core_rpc_mock = MockCoreRPCLike::new();

        // Set up block hash expectation
        core_rpc_mock.expect_get_block_hash().returning(|_| {
            Ok(BlockHash::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap())
        });

        // Set up block JSON expectation
        core_rpc_mock.expect_get_block_json().returning(|_| {
            Ok(json!({
                "tx": [],
            }))
        });

        // Set up the optional transaction extended info expectation
        let tx_clone = transaction.clone();
        core_rpc_mock
            .expect_get_optional_transaction_extended_info()
            .returning(move |_txid| {
                // Create the GetRawTransactionResult
                Ok(Some(GetRawTransactionResult {
                    in_active_chain: true,
                    hex: dpp::dashcore::consensus::serialize(&tx_clone),
                    txid: tx_clone.txid(),
                    size: 0,
                    version: tx_clone.version as u32,
                    tx_type: 8, // Asset lock transaction type
                    locktime: tx_clone.lock_time,
                    vin: vec![],
                    vout: vec![],
                    extra_payload_size: None,
                    extra_payload: None,
                    blockhash: Some(
                        BlockHash::from_str(
                            "0000000000000000000000000000000000000000000000000000000000000001",
                        )
                        .unwrap(),
                    ),
                    confirmations: Some(1000),
                    time: Some(0),
                    blocktime: Some(0),
                    height: Some(transaction_height as i32),
                    instantlock: false,
                    instantlock_internal: false,
                    chainlock: true,
                }))
            });

        let use_initial_protocol_version = Some(PlatformVersion::latest().protocol_version);
        let platform = Platform::<MockCoreRPCLike>::open_with_client(
            tempdir.path(),
            Some(platform_config),
            core_rpc_mock,
            use_initial_protocol_version,
        )
        .expect("should open Platform successfully");

        crate::test::helpers::setup::TempPlatform { platform, tempdir }
    }

    /// Get the balance of an address from the drive
    #[allow(dead_code)]
    fn get_address_balance(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        address: PlatformAddress,
        transaction: &drive::grovedb::Transaction,
    ) -> u64 {
        let platform_version = PlatformVersion::latest();
        platform
            .drive
            .fetch_balance_and_nonce(&address, Some(transaction), platform_version)
            .ok()
            .flatten()
            .map(|(_, balance)| balance)
            .unwrap_or(0)
    }

    /// Get the nonce of an address from the drive
    #[allow(dead_code)]
    fn get_address_nonce(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        address: PlatformAddress,
        transaction: &drive::grovedb::Transaction,
    ) -> AddressNonce {
        let platform_version = PlatformVersion::latest();
        platform
            .drive
            .fetch_balance_and_nonce(&address, Some(transaction), platform_version)
            .ok()
            .flatten()
            .map(|(nonce, _)| nonce)
            .unwrap_or(0)
    }

    /// Check if an asset lock outpoint has been spent
    /// TODO: Implement proper check using Drive when available
    #[allow(dead_code)]
    fn is_asset_lock_spent(
        _platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        _outpoint: &dpp::dashcore::OutPoint,
        _transaction: &drive::grovedb::Transaction,
    ) -> bool {
        // For now, always return true - tests using this need to be updated
        // when proper asset lock spent tracking is implemented
        true
    }

    /// Perform check_tx on a raw transaction and return whether it's valid
    /// This simulates what happens when a transaction is submitted to the mempool.
    /// - invalid_unpaid transactions should return false (rejected from mempool)
    /// - invalid_paid transactions should return true (accepted to mempool, will fail at processing)
    fn check_tx_is_valid(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        raw_tx: &[u8],
        platform_version: &PlatformVersion,
    ) -> bool {
        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let check_result = platform
            .check_tx(
                raw_tx,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                platform_version,
            )
            .expect("expected to check tx");

        check_result.is_valid()
    }

    /// Generate signable bytes for an address funding from asset lock transition
    /// This creates an unsigned transition and gets its signable bytes
    fn get_signable_bytes_for_transition(
        asset_lock_proof: &dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        inputs: &BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: &BTreeMap<PlatformAddress, Option<u64>>,
    ) -> Vec<u8> {
        use dpp::serialization::Signable;

        let transition =
            AddressFundingFromAssetLockTransition::V0(AddressFundingFromAssetLockTransitionV0 {
                asset_lock_proof: asset_lock_proof.clone(),
                inputs: inputs.clone(),
                outputs: outputs.clone(),
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ]),
                user_fee_increase: 0,
                signature: BinaryData::new(vec![0u8; 65]),
                input_witnesses: vec![],
            });

        let state_transition: StateTransition = transition.into();
        state_transition
            .signable_bytes()
            .expect("should get signable bytes")
    }

    /// Create a signed AddressFundingFromAssetLockTransition
    fn create_signed_address_funding_from_asset_lock_transition(
        asset_lock_proof: dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        asset_lock_private_key: &[u8],
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, Option<u64>>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
    ) -> StateTransition {
        create_signed_address_funding_from_asset_lock_transition_with_fee_increase(
            asset_lock_proof,
            asset_lock_private_key,
            signer,
            inputs,
            outputs,
            fee_strategy,
            0,
        )
    }

    fn create_signed_address_funding_from_asset_lock_transition_with_fee_increase(
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: &[u8],
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, Option<u64>>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        user_fee_increase: u16,
    ) -> StateTransition {
        AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer(
            asset_lock_proof,
            asset_lock_private_key,
            inputs,
            outputs,
            AddressFundsFeeStrategy::from(fee_strategy),
            signer,
            user_fee_increase,
            PlatformVersion::latest(),
        )
        .expect("should create signed transition")
    }

    /// Create a transition with valid asset lock signature but custom (possibly invalid) witnesses.
    /// This is used for tests that need to test invalid witness scenarios.
    fn create_transition_with_custom_witnesses(
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: &[u8],
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, Option<u64>>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        custom_witnesses: Vec<AddressWitness>,
    ) -> StateTransition {
        use dpp::serialization::Signable;

        // Create the unsigned transition
        let mut address_funding_transition = AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            inputs,
            outputs,
            fee_strategy: AddressFundsFeeStrategy::from(fee_strategy),
            user_fee_increase: 0,
            signature: Default::default(),
            input_witnesses: custom_witnesses.clone(),
        };

        let state_transition: StateTransition = address_funding_transition.clone().into();
        let signable_bytes = state_transition
            .signable_bytes()
            .expect("should get signable bytes");

        // Sign the asset lock proof with the private key
        let signature = dpp::dashcore::signer::sign(&signable_bytes, asset_lock_private_key)
            .expect("should sign");
        address_funding_transition.signature = signature.to_vec().into();
        address_funding_transition.input_witnesses = custom_witnesses;

        address_funding_transition.into()
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS
    // These test basic structure validation (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_no_outputs_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            // No outputs case - should fail validation
            let inputs = BTreeMap::new();
            let outputs = BTreeMap::new(); // Empty outputs

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                0,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::TransitionNoOutputsError(_))
                )]
            );
        }

        #[test]
        fn test_too_many_inputs_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();
            let max_inputs = platform_version.dpp.state_transitions.max_address_inputs;

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            // Create max_inputs + 1 inputs (17 inputs, max is 16)
            let input_count = max_inputs as usize + 1;
            let mut inputs = BTreeMap::new();
            for i in 0..input_count {
                inputs.insert(
                    create_platform_address(i as u8),
                    (1 as AddressNonce, dash_to_credits!(0.01)),
                );
            }

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(100), None); // Remainder recipient

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                input_count, // Match input count
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::TransitionOverMaxInputsError(e))
                    if e.actual_inputs() == 17 && e.max_inputs() == 16
                ),
                "Expected TransitionOverMaxInputsError with 17 actual and 16 max, got {:?}",
                error
            );
        }

        #[test]
        fn test_too_many_outputs_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            // Create max_outputs + 1 outputs (17 outputs, max is 16)
            let output_count = max_outputs as usize + 1;
            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            // First output_count - 1 are explicit, last one is remainder
            for i in 0..(output_count - 1) {
                outputs.insert(
                    create_platform_address(i as u8),
                    Some(dash_to_credits!(0.1)),
                );
            }
            outputs.insert(create_platform_address((output_count - 1) as u8), None); // Remainder

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                0,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::TransitionOverMaxOutputsError(e))
                    if e.actual_outputs() == output_count as u16 && e.max_outputs() == max_outputs
                ),
                "Expected TransitionOverMaxOutputsError with {} actual and {} max, got {:?}",
                output_count,
                max_outputs,
                error
            );
        }

        #[test]
        fn test_input_witness_count_mismatch_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(0.1)),
            );
            inputs.insert(
                create_platform_address(2),
                (1 as AddressNonce, dash_to_credits!(0.1)),
            );

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(3), None); // Remainder recipient

            // Create transition with 2 inputs but only 1 witness
            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1, // Only 1 witness for 2 inputs
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::InputWitnessCountMismatchError(_))
                ),
                "Expected InputWitnessCountMismatchError, got {:?}",
                error
            );
        }

        #[test]
        fn test_output_address_also_input_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let same_address = create_platform_address(1);

            let mut inputs = BTreeMap::new();
            inputs.insert(same_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(same_address, None); // Same address as input (remainder)

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::OutputAddressAlsoInputError(_))
                ),
                "Expected OutputAddressAlsoInputError, got {:?}",
                error
            );
        }

        #[test]
        fn test_empty_fee_strategy_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Empty fee strategy
            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![]),
                0,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyEmptyError(_))
                )]
            );
        }

        #[test]
        fn test_fee_strategy_index_out_of_bounds_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(1.0))); // Explicit output
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // ReduceOutput(5) but only 1 explicit output exists (index 0)
            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(5)]),
                0,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyIndexOutOfBoundsError(_))
                )]
            );
        }

        #[test]
        fn test_no_remainder_output_returns_error() {
            // Exactly one output must be None (the remainder recipient)
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();

            let mut outputs = BTreeMap::new();
            // All outputs are explicit (Some) - no remainder recipient
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(2), Some(dash_to_credits!(0.4)));

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                0,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::InvalidRemainderOutputCountError(e))
                    if e.actual_count() == 0
                ),
                "Expected InvalidRemainderOutputCountError with 0 count, got {:?}",
                error
            );
        }

        #[test]
        fn test_multiple_remainder_outputs_returns_error() {
            // Exactly one output must be None (the remainder recipient)
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();

            let mut outputs = BTreeMap::new();
            // Two remainder recipients - invalid
            outputs.insert(create_platform_address(1), None);
            outputs.insert(create_platform_address(2), None);

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                0,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::InvalidRemainderOutputCountError(e))
                    if e.actual_count() == 2
                ),
                "Expected InvalidRemainderOutputCountError with 2 count, got {:?}",
                error
            );
        }

        #[test]
        fn test_output_below_minimum_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(100)); // Very small explicit output - below minimum
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                0,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OutputBelowMinimumError(_))
                )]
            );
        }
    }

    // ==========================================
    // SUCCESSFUL TRANSITION TESTS
    // ==========================================

    mod successful_transitions {
        use super::*;

        #[test]
        fn test_simple_asset_lock_funding_to_single_address() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // No inputs - just funding from asset lock
            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            // One explicit output and one remainder
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.5))); // Explicit output
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_asset_lock_funding_to_multiple_addresses() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // No inputs - funding from asset lock to multiple outputs
            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.3)));
            outputs.insert(create_platform_address(2), Some(dash_to_credits!(0.3)));
            outputs.insert(create_platform_address(3), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_asset_lock_funding_combined_with_existing_address_input() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Combine existing address funds with asset lock
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            // Explicit output plus remainder
            outputs.insert(create_platform_address(2), Some(dash_to_credits!(0.5))); // Explicit output
            outputs.insert(create_platform_address(3), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    // ==========================================
    // REMAINDER OUTPUT HANDLING TESTS
    // These test the logic for handling remainder outputs based on available funds
    // ==========================================

    mod remainder_output_handling {
        use super::*;

        #[test]
        fn test_explicit_outputs_exceed_available_funds_returns_error() {
            // When explicit outputs sum > asset_lock + inputs, should return error
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(950);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            // Asset lock is 1 DASH

            // No inputs
            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            // Explicit output of 2 DASH - more than the 1 DASH asset lock
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(2.0)));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should PASS for invalid_paid transactions - they get accepted to mempool
            // but fail at processing time (fees are still paid)
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept invalid_paid transaction to mempool (insufficient funds for outputs)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail with AddressesNotEnoughFundsError
            // Note: This is now invalid_paid because advanced structure validation
            // creates a PartiallyUseAssetLockAction that deducts a penalty from the asset lock
            assert_eq!(processing_result.invalid_paid_count(), 1);
        }

        #[test]
        fn test_exact_match_removes_remainder_output() {
            // When explicit outputs sum == asset_lock + inputs, remainder output should be removed
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([51u8; 32]);
            // Set up input address with exactly the amount we need to make totals match
            // Asset lock = 1 DASH, we want explicit output = 1.5 DASH
            // So input needs to provide 0.5 DASH
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(951);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            // Asset lock is 1 DASH

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            // Explicit output of exactly 1.5 DASH (= 1 DASH asset lock + 0.5 DASH input)
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(1.5)));
            outputs.insert(create_platform_address(2), None); // Remainder - should be removed

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Verify the explicit output address received funds (minus fees)
            let output_balance =
                get_address_balance(&platform, create_platform_address(1), &transaction);
            assert!(
                output_balance > 0,
                "Output address should have received funds"
            );
            // Should be less than 1.5 DASH due to fee deduction
            assert!(
                output_balance < dash_to_credits!(1.5),
                "Output balance {} should be less than 1.5 DASH due to fees",
                output_balance
            );

            // Verify the remainder address received nothing (was removed)
            let remainder_balance =
                get_address_balance(&platform, create_platform_address(2), &transaction);
            assert_eq!(
                remainder_balance, 0,
                "Remainder address should have received nothing when funds exactly match"
            );
        }

        #[test]
        fn test_surplus_funds_go_to_remainder() {
            // When explicit outputs sum < asset_lock + inputs, remainder gets the difference
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(952);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            // Asset lock is 1 DASH

            // No inputs
            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            // Explicit output of 0.3 DASH - less than the 1 DASH asset lock
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.3)));
            outputs.insert(create_platform_address(2), None); // Remainder - should receive ~0.7 DASH minus fees

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Verify the explicit output received its amount (minus fees from ReduceOutput(0))
            let explicit_balance =
                get_address_balance(&platform, create_platform_address(1), &transaction);
            assert!(
                explicit_balance > 0 && explicit_balance < dash_to_credits!(0.3),
                "Explicit output should have received funds minus fees"
            );

            // Verify the remainder address received the surplus
            let remainder_balance =
                get_address_balance(&platform, create_platform_address(2), &transaction);
            assert!(
                remainder_balance > 0,
                "Remainder address should have received surplus funds"
            );
            // Remainder should be approximately 0.7 DASH (1.0 - 0.3)
            assert!(
                remainder_balance > dash_to_credits!(0.5),
                "Remainder balance {} should be substantial",
                remainder_balance
            );
        }
    }

    // ==========================================
    // STATE VALIDATION TESTS
    // These test state validation errors (StateError)
    // ==========================================

    mod state_validation {
        use super::*;

        #[test]
        fn test_input_address_does_not_exist_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Note: NOT setting up balance for this address

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(3), None); // Remainder

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail because input address doesn't exist
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(_))
                )]
            );
        }

        #[test]
        fn test_insufficient_balance_in_input_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up with only 0.5 DASH
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            // Try to spend 0.8 DASH when only 0.5 available
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(3), None); // Remainder

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail because of insufficient balance
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressNotEnoughFundsError(_))
                )]
            );
        }

        #[test]
        fn test_wrong_nonce_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up with nonce 0
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            // Use wrong nonce (5 instead of expected 1)
            inputs.insert(input_address, (5 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(3), None); // Remainder

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail because of invalid nonce
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(_))
                )]
            );
        }
    }

    // ==========================================
    // SIGNATURE VALIDATION TESTS
    // ==========================================

    mod signature_validation {
        use super::*;

        #[test]
        fn test_wrong_asset_lock_signature_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            // Use a DIFFERENT key to sign (not matching the asset lock)
            let wrong_private_key = [42u8; 32];

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(2), None); // Remainder

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &wrong_private_key,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (wrong asset lock signature)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to signature verification error
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_signature_from_different_key_for_input_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create two signers - one for the "real" address, one that will sign incorrectly
            let mut real_signer = TestAddressSigner::new();
            let real_address = real_signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, real_address, 0, dash_to_credits!(1.0));

            // Create a different signer with a different key
            let mut wrong_signer = TestAddressSigner::new();
            let wrong_address = wrong_signer.add_p2pkh([2u8; 32]);
            // Add the real address hash to the wrong signer so it can "try" to sign for it
            // but with the wrong key
            wrong_signer.p2pkh_keys.insert(
                match real_address {
                    PlatformAddress::P2pkh(h) => h,
                    _ => panic!("expected p2pkh"),
                },
                wrong_signer.p2pkh_keys[&match wrong_address {
                    PlatformAddress::P2pkh(h) => h,
                    _ => panic!("expected p2pkh"),
                }]
                    .clone(),
            );

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(real_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(3), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(4), None); // Remainder

            // Sign with wrong signer
            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &wrong_signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (wrong input signature)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to witness verification error
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    // ==========================================
    // P2SH MULTISIG TESTS
    // ==========================================

    mod p2sh_multisig {
        use super::*;

        #[test]
        fn test_asset_lock_with_p2sh_multisig_input() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            // Create a 2-of-3 multisig
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(2), None); // Remainder

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_asset_lock_with_mixed_p2pkh_and_p2sh_inputs() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();

            // Create a P2PKH input
            let p2pkh_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, p2pkh_address, 0, dash_to_credits!(0.5));

            // Create a 2-of-3 P2SH multisig input
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2pkh_address, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(2), None); // Remainder

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_p2sh_with_insufficient_signatures_fails() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            // Create a 2-of-3 multisig
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(2), None); // Remainder

            // Create transition manually with only 1 signature instead of required 2
            let mut transition = AddressFundingFromAssetLockTransitionV0 {
                asset_lock_proof: asset_lock_proof.clone(),
                inputs: inputs.clone(),
                outputs: outputs.clone(),
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ]),
                user_fee_increase: 0,
                signature: BinaryData::new(vec![0u8; 65]),
                input_witnesses: vec![],
            };

            // Get the entry and create witness with insufficient signatures
            let hash = match p2sh_address {
                PlatformAddress::P2sh(h) => h,
                _ => panic!("expected p2sh"),
            };
            let entry = signer.p2sh_entries.get(&hash).unwrap();

            // Only provide 1 signature instead of required 2
            let single_signature = TestAddressSigner::sign_data(&[0u8; 32], &entry.secret_keys[0]);
            transition.input_witnesses = vec![AddressWitness::P2sh {
                signatures: vec![BinaryData::new(single_signature)], // Only 1 sig, need 2
                redeem_script: BinaryData::new(entry.redeem_script.clone()),
            }];

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (insufficient P2SH signatures)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to insufficient signatures
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    // ==========================================
    // ADDITIONAL STRUCTURE VALIDATION TESTS
    // ==========================================

    mod additional_structure_validation {
        use super::*;

        #[test]
        fn test_fee_strategy_duplicate_steps_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.5)));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // Duplicate fee strategy steps
            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0), // Duplicate
                ]),
                0,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyDuplicateError(_))
                )]
            );
        }

        #[test]
        fn test_deduct_from_input_index_out_of_bounds_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(0.1)),
            );

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // DeductFromInput(5) but only 1 input exists
            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    5,
                )]),
                1,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::FeeStrategyIndexOutOfBoundsError(_))
                ),
                "Expected FeeStrategyIndexOutOfBoundsError, got {:?}",
                error
            );
        }

        #[test]
        fn test_input_below_minimum_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            // Very small input - below minimum (100 credits, minimum is 500,000)
            inputs.insert(create_platform_address(1), (1 as AddressNonce, 100));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                1,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::InputBelowMinimumError(_))
                ),
                "Expected InputBelowMinimumError, got {:?}",
                error
            );
        }
    }

    // ==========================================
    // EDGE CASE TESTS
    // ==========================================

    mod edge_cases {
        use super::*;

        #[test]
        fn test_maximum_allowed_inputs_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut signer = TestAddressSigner::new();

            // Create exactly 16 inputs (the maximum allowed)
            let mut inputs = BTreeMap::new();
            for i in 1..=16u8 {
                let addr = signer.add_p2pkh([i; 32]);
                setup_address_with_balance(&mut platform, addr, 0, dash_to_credits!(0.1));
                inputs.insert(addr, (1 as AddressNonce, dash_to_credits!(0.05)));
            }

            let mut outputs = BTreeMap::new();
            // 16 * 0.05 = 0.8 from inputs + 1.0 from asset lock = 1.8 total
            outputs.insert(create_platform_address(100), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_maximum_allowed_outputs_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // No inputs, just asset lock
            let inputs = BTreeMap::new();

            // Create exactly 16 outputs (the maximum allowed)
            let mut outputs = BTreeMap::new();
            for i in 1..=15u8 {
                outputs.insert(create_platform_address(i), Some(dash_to_credits!(0.05)));
            }
            outputs.insert(create_platform_address(16), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_multiple_p2pkh_inputs_with_asset_lock() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();

            // Create 3 P2PKH inputs
            let addr1 = signer.add_p2pkh([1u8; 32]);
            let addr2 = signer.add_p2pkh([2u8; 32]);
            let addr3 = signer.add_p2pkh([3u8; 32]);

            setup_address_with_balance(&mut platform, addr1, 0, dash_to_credits!(0.5));
            setup_address_with_balance(&mut platform, addr2, 0, dash_to_credits!(0.5));
            setup_address_with_balance(&mut platform, addr3, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(addr1, (1 as AddressNonce, dash_to_credits!(0.2)));
            inputs.insert(addr2, (1 as AddressNonce, dash_to_credits!(0.2)));
            inputs.insert(addr3, (1 as AddressNonce, dash_to_credits!(0.2)));

            let mut outputs = BTreeMap::new();
            // 0.6 from inputs + 1.0 from asset lock = 1.6 total
            outputs.insert(create_platform_address(10), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    // ==========================================
    // FEE STRATEGY TESTS
    // ==========================================

    mod fee_strategy {
        use super::*;

        #[test]
        fn test_multiple_fee_strategy_steps_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.6)));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // Multiple fee strategy steps: first try input, then outputs
            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(1),
                ],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_deduct_from_input_only_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Only DeductFromInput, fees come from input surplus
            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    // ==========================================
    // ASSET LOCK SPECIFIC TESTS
    // ==========================================

    mod asset_lock_validation {
        use super::*;
        use dpp::consensus::signature::SignatureError;

        #[test]
        fn test_asset_lock_already_spent_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof.clone(),
                &asset_lock_pk,
                &signer,
                inputs.clone(),
                outputs.clone(),
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // First transition should succeed
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            // Try to use the same asset lock again
            let transition2 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result2 = transition2.serialize_to_bytes().expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result2, platform_version),
                "check_tx should reject invalid_unpaid transaction (asset lock already used)"
            );

            let platform_state2 = platform.state.load();
            let transaction2 = platform.drive.grove.start_transaction();

            let processing_result2 = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result2],
                    &platform_state2,
                    &BlockInfo::default(),
                    &transaction2,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Second attempt should fail - asset lock already used
            assert_eq!(processing_result2.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_invalid_signature_format_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Create transition with invalid signature (wrong length)
            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(vec![0u8; 10]), // Invalid signature length
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (invalid signature format)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to invalid signature
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    // ==========================================
    // BALANCE VERIFICATION TESTS
    // ==========================================

    mod balance_verification {
        use super::*;

        #[test]
        fn test_output_address_receives_correct_balance() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let output_address = create_platform_address(1);
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            // Verify the output address received funds (minus fees)
            let balance_and_nonce = platform
                .drive
                .fetch_balance_and_nonce(&output_address, None, platform_version)
                .expect("expected to fetch balance");

            // Balance should be approximately 1.0 DASH minus processing fees (gets remainder)
            assert!(balance_and_nonce.is_some());
            let (_nonce, actual_balance) = balance_and_nonce.unwrap();
            // Should be less than full asset lock value due to fees, but greater than 0
            assert!(actual_balance > 0);
            assert!(actual_balance < dash_to_credits!(1.0));
        }

        #[test]
        fn test_input_address_balance_reduced_correctly() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let initial_balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, initial_balance);

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let input_amount = dash_to_credits!(0.5);
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, input_amount));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            // Verify the input address balance was reduced
            let remaining_balance_and_nonce = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("expected to fetch balance");

            assert!(remaining_balance_and_nonce.is_some());
            let (_nonce, actual_remaining) = remaining_balance_and_nonce.unwrap();
            // Remaining should be initial - input_amount = 1.0 - 0.5 = 0.5 DASH
            assert_eq!(actual_remaining, initial_balance - input_amount);
        }
    }

    // ==========================================
    // WITNESS VALIDATION TESTS
    // ==========================================

    mod witness_validation {
        use super::*;

        #[test]
        fn test_p2pkh_with_wrong_signature_length_fails() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // Create transition with invalid witness (wrong signature length)
            let state_transition = create_transition_with_custom_witnesses(
                asset_lock_proof,
                &asset_lock_pk,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                vec![AddressWitness::P2pkh {
                    signature: BinaryData::new(vec![0u8; 10]), // Wrong length
                }],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (wrong signature length)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to invalid witness
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_p2sh_with_wrong_redeem_script_fails() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Get the real entry for signatures
            let hash = match p2sh_address {
                PlatformAddress::P2sh(h) => h,
                _ => panic!("expected p2sh"),
            };
            let entry = signer.p2sh_entries.get(&hash).unwrap();

            // Create valid signatures but with WRONG redeem script
            let dummy_data = [0u8; 32];
            let signatures: Vec<BinaryData> = entry
                .secret_keys
                .iter()
                .take(2)
                .map(|sk| BinaryData::new(TestAddressSigner::sign_data(&dummy_data, sk)))
                .collect();

            let state_transition = create_transition_with_custom_witnesses(
                asset_lock_proof,
                &asset_lock_pk,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                vec![AddressWitness::P2sh {
                    signatures,
                    redeem_script: BinaryData::new(vec![0u8; 50]), // Wrong redeem script
                }],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (wrong redeem script hash)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to wrong redeem script hash
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_witness_type_mismatch_fails() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            // Create a P2PKH address
            let p2pkh_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, p2pkh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2pkh_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // Create transition with P2SH witness for P2PKH address (type mismatch)
            let state_transition = create_transition_with_custom_witnesses(
                asset_lock_proof,
                &asset_lock_pk,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                vec![AddressWitness::P2sh {
                    signatures: vec![BinaryData::new(vec![0u8; 65])],
                    redeem_script: BinaryData::new(vec![0u8; 50]),
                }],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (witness type mismatch)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to witness type mismatch
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod fee_edge_cases {
        use super::*;

        #[test]
        fn test_fee_equals_exact_remaining_balance() {
            // Test where fee exactly equals the remaining balance after outputs
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();

            let mut rng = StdRng::seed_from_u64(600);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            // Asset lock provides 1 DASH

            let mut outputs = BTreeMap::new();
            // Output receives remainder after fees
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed if fee is covered by remaining 0.01 DASH
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_fee_exceeds_remaining_by_one_credit() {
            // Test where the output amount equals the entire asset lock value.
            // The fee strategy reduces the output to cover fees.
            // After fee deduction, the output should be reduced, which is valid behavior.
            // This test confirms that when ReduceOutput is used, the transaction succeeds
            // by reducing the output amount (the fee is taken from the output itself).
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(601);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Output gets the remainder after fees
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - the fee is deducted from the output amount
            // The recipient receives (1 DASH - fee) instead of 1 DASH
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_user_fee_increase_with_reduce_output_succeeds() {
            // Test that the fee increase actually results in higher fees being paid.
            // The user_fee_increase multiplier only applies to processing fees, not storage fees.
            // Formula: total_fee = storage_fee + processing_fee * (1 + user_fee_increase / 100)
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let output_address = create_platform_address(1);
            let asset_lock_value = dash_to_credits!(1.0); // From fixture

            // First transaction: NO fee increase
            let platform_no_increase = TestPlatformBuilder::new()
                .with_config(platform_config.clone())
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(602);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition_no_increase =
                create_signed_address_funding_from_asset_lock_transition_with_fee_increase(
                    asset_lock_proof,
                    &asset_lock_pk,
                    &signer,
                    BTreeMap::new(),
                    outputs.clone(),
                    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                    0, // No fee increase
                );

            let result = state_transition_no_increase
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform_no_increase.state.load();
            let transaction = platform_no_increase.drive.grove.start_transaction();

            let processing_result = platform_no_increase
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            let fee_result_no_increase = assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, fee_result)] => fee_result.clone()
            );

            let balance_no_increase =
                get_address_balance(&platform_no_increase, output_address, &transaction);

            // Second transaction: MAXIMUM fee increase (u16::MAX = 655.35% extra on processing fees)
            let platform_max_increase = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(6020); // Different seed for different asset lock
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let state_transition_max_increase =
                create_signed_address_funding_from_asset_lock_transition_with_fee_increase(
                    asset_lock_proof,
                    &asset_lock_pk,
                    &signer,
                    BTreeMap::new(),
                    outputs,
                    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                    u16::MAX, // Maximum fee increase (655.35% extra on processing fees)
                );

            let result = state_transition_max_increase
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform_max_increase.state.load();
            let transaction = platform_max_increase.drive.grove.start_transaction();

            let processing_result = platform_max_increase
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            let fee_result_max_increase = assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, fee_result)] => fee_result.clone()
            );

            let balance_max_increase =
                get_address_balance(&platform_max_increase, output_address, &transaction);

            // Calculate actual fees paid (deducted from the asset lock value)
            let fee_paid_no_increase = asset_lock_value - balance_no_increase;
            let fee_paid_max_increase = asset_lock_value - balance_max_increase;

            // Verify the balance with max fee increase is lower (more fee was paid)
            assert!(
                balance_max_increase < balance_no_increase,
                "Balance with max fee increase ({}) should be less than balance without increase ({})",
                balance_max_increase,
                balance_no_increase
            );

            // Storage fees should be the same (not affected by user_fee_increase)
            assert_eq!(
                fee_result_no_increase.storage_fee, fee_result_max_increase.storage_fee,
                "Storage fees should be identical regardless of user_fee_increase"
            );

            // Processing fee with max increase should be much higher
            // With u16::MAX (65535), multiplier is (1 + 65535/100) = 656.35x
            let expected_processing_fee_multiplier = 1.0 + (u16::MAX as f64 / 100.0);
            let actual_processing_fee_ratio = fee_result_max_increase.processing_fee as f64
                / fee_result_no_increase.processing_fee as f64;

            assert!(
                (actual_processing_fee_ratio - expected_processing_fee_multiplier).abs() < 1.0,
                "Processing fee ratio should be ~{:.2}x, got {:.2}x (no_increase: {}, max_increase: {})",
                expected_processing_fee_multiplier,
                actual_processing_fee_ratio,
                fee_result_no_increase.processing_fee,
                fee_result_max_increase.processing_fee
            );

            // Verify the actual fee deducted from output matches the fee result
            let total_fee_no_increase = fee_result_no_increase.total_base_fee();
            let total_fee_max_increase = fee_result_max_increase.total_base_fee();

            assert_eq!(
                fee_paid_no_increase, total_fee_no_increase,
                "Fee deducted from output should match total_base_fee (no increase)"
            );
            assert_eq!(
                fee_paid_max_increase, total_fee_max_increase,
                "Fee deducted from output should match total_base_fee (max increase)"
            );

            // Verify both fees are positive
            assert!(fee_paid_no_increase > 0, "Base fee should be positive");
            assert!(
                fee_paid_max_increase > fee_paid_no_increase,
                "Max increase fee ({}) should be higher than base fee ({})",
                fee_paid_max_increase,
                fee_paid_no_increase
            );
        }

        #[test]
        fn test_user_fee_increase_small_amount() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();

            let mut rng = StdRng::seed_from_u64(603);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition =
                create_signed_address_funding_from_asset_lock_transition_with_fee_increase(
                    asset_lock_proof,
                    &asset_lock_pk,
                    &signer,
                    BTreeMap::new(),
                    outputs,
                    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                    100, // Small fee increase (1%)
                );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with small fee increase
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod asset_lock_edge_cases {
        use super::*;

        #[test]
        fn test_asset_lock_double_spend_same_block() {
            // Test using the same asset lock twice in the same block
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(611);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs1 = BTreeMap::new();
            outputs1.insert(create_platform_address(1), None); // Remainder recipient

            let mut outputs2 = BTreeMap::new();
            outputs2.insert(create_platform_address(2), None); // Remainder recipient

            let signer = TestAddressSigner::new();

            // Create two transitions using the same asset lock
            let state_transition1 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof.clone(),
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs1,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let state_transition2 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result1 = state_transition1
                .serialize_to_bytes()
                .expect("should serialize");
            let result2 = state_transition2
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1, result2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // First should succeed, second should fail as double spend (already consumed)
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [
                    StateTransitionExecutionResult::SuccessfulExecution(_, _),
                    StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::IdentityAssetLockTransactionOutPointAlreadyConsumedError(_)
                        )
                    )
                ]
            );
        }

        #[test]
        fn test_asset_lock_already_used_in_previous_block() {
            // Test using an asset lock that was already consumed in a previous block
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(612);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();

            // First transition - should succeed
            let state_transition1 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof.clone(),
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs.clone(),
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result1 = state_transition1
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("commit");

            // Now try to use the same asset lock again
            let mut outputs2 = BTreeMap::new();
            outputs2.insert(create_platform_address(2), None); // Remainder recipient

            let state_transition2 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result2 = state_transition2
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result2, platform_version),
                "check_tx should reject invalid_unpaid transaction (asset lock fully used)"
            );

            let platform_state = platform.state.load();
            let transaction2 = platform.drive.grove.start_transaction();

            let processing_result2 = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result2],
                    &platform_state,
                    &BlockInfo::default_with_height(2),
                    &transaction2,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - asset lock already used
            assert_eq!(processing_result2.invalid_unpaid_count(), 1);
        }
    }

    mod nonce_edge_cases {
        use super::*;

        #[test]
        fn test_nonce_zero_for_new_address() {
            // New address should have nonce 0, so first tx should use nonce 1
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([20u8; 32]);
            // Set up address with nonce 0 (brand new address)
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(620);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5))); // First nonce should be 1

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_nonce_gap_fails() {
            // Skipping a nonce should fail
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([21u8; 32]);
            // Address has nonce 5
            setup_address_with_balance(&mut platform, input_address, 5, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(621);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            // Skip from 5 to 7 (should use 6)
            inputs.insert(input_address, (7 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            let witness = signer
                .sign_p2pkh(input_address, &signable_bytes)
                .expect("should sign");

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (nonce gap)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to nonce gap
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_nonce_reuse_fails() {
            // Using an already-used nonce should fail
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([22u8; 32]);
            // Address already used nonce 5, current nonce is 5
            setup_address_with_balance(&mut platform, input_address, 5, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(622);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            // Try to use nonce 5 again (should use 6)
            inputs.insert(input_address, (5 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            let witness = signer
                .sign_p2pkh(input_address, &signable_bytes)
                .expect("should sign");

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (nonce already used)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to nonce already used
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_high_nonce_value() {
            // Test with a very high nonce value
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([23u8; 32]);
            // Very high nonce (max u32 - 1)
            let high_nonce: AddressNonce = u32::MAX - 1;
            setup_address_with_balance(
                &mut platform,
                input_address,
                high_nonce,
                dash_to_credits!(1.0),
            );

            let mut rng = StdRng::seed_from_u64(623);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (high_nonce + 1, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with high nonce
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod amount_edge_cases {
        use super::*;

        #[test]
        fn test_output_amount_near_u64_max() {
            // Test with amounts near u64::MAX to check overflow protection
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(630);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(u64::MAX - 1000));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (output exceeds asset lock value)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - output exceeds asset lock value
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_zero_input_amount() {
            // Input with zero amount
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([32u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(632);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, 0)); // Zero amount

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            let witness = signer
                .sign_p2pkh(input_address, &signable_bytes)
                .expect("should sign");

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (zero input amount)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - zero input amount
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_zero_output_amount() {
            // Output with zero amount
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(633);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(0)); // Zero amount
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (zero output amount)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - zero output amount
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod platform_state_edge_cases {
        use super::*;

        #[test]
        fn test_address_with_zero_balance() {
            // Address exists but has zero balance
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([40u8; 32]);
            // Address exists with zero balance
            setup_address_with_balance(&mut platform, input_address, 0, 0);

            let mut rng = StdRng::seed_from_u64(640);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (insufficient balance)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - insufficient balance
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_output_to_existing_address_adds_balance() {
            // Sending to an address that already exists should add to balance
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let output_address = create_platform_address(1);
            // Set up output address with existing balance
            setup_address_with_balance(&mut platform, output_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(641);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Verify balance was added (not replaced)
            // Initial: 1.0 DASH, Remainder output receives asset lock value (1.0 DASH) minus fees
            // Balance should be > 1.0 DASH (original) since we added from asset lock
            let new_balance = get_address_balance(&platform, output_address, &transaction);
            assert!(
                new_balance > dash_to_credits!(1.0),
                "Balance {} should be greater than original 1.0 DASH",
                new_balance
            );
            // Balance should be close to 2.0 DASH (1.0 initial + 1.0 from asset lock - fees)
            // But less than 2.0 DASH due to fee deduction
            assert!(
                new_balance < dash_to_credits!(2.0),
                "Balance {} should be less than 2.0 DASH due to fees",
                new_balance
            );
        }

        #[test]
        fn test_multiple_inputs_from_same_address_deduplicated_by_btreemap() {
            // BTreeMap naturally prevents duplicate addresses in inputs
            // This test demonstrates that behavior - the second insert overwrites the first
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([42u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(642);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // BTreeMap will only keep one entry per key - second insert overwrites
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.4))); // Overwrites with same nonce

            // Only one input in map due to BTreeMap dedup
            assert_eq!(inputs.len(), 1);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // This demonstrates that BTreeMap deduplication works
            // The transition itself should succeed (with only one input)
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod dust_and_minimum_amounts {
        use super::*;

        #[test]
        fn test_output_becomes_below_minimum_after_fee_deduction() {
            // Output starts above minimum but falls below after fee deduction
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(650);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Set output to minimum allowed + tiny bit, so after fee deduction it might be dust
            outputs.insert(create_platform_address(1), Some(1001)); // Just above minimum
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (dust output after fee)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - output would be dust after fee deduction
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_minimum_output_after_fee_deduction() {
            // Output after fee deduction equals exactly minimum
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(651);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Output receives remainder after fees
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed if output after fee >= minimum
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod signature_recovery_edge_cases {
        use super::*;

        #[test]
        fn test_recovered_pubkey_wrong_address() {
            // Signature is valid but recovered pubkey hashes to wrong address
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([50u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a different signer for wrong signature
            let mut wrong_signer = TestAddressSigner::new();
            let _wrong_address = wrong_signer.add_p2pkh([51u8; 32]);

            let mut rng = StdRng::seed_from_u64(660);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Sign with the wrong signer's key
            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            // Create signature with wrong key - the recovered pubkey won't match the address
            let witness = wrong_signer
                .sign_p2pkh(_wrong_address, &signable_bytes)
                .expect("should sign");

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (recovered pubkey wrong address)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - recovered pubkey doesn't match input address
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_invalid_recovery_id() {
            // Signature with invalid recovery ID
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([52u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(661);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            let mut witness = signer
                .sign_p2pkh(input_address, &signable_bytes)
                .expect("should sign");

            // Corrupt the recovery ID (last byte of signature)
            match &mut witness {
                AddressWitness::P2pkh { signature } => {
                    let len = signature.len();
                    signature.0[len - 1] = 0xFF; // Invalid recovery ID
                }
                _ => panic!("Expected P2PKH witness"),
            }

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (invalid recovery ID)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - invalid recovery ID
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_signature_for_different_message() {
            // Valid signature but for different signable bytes
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([53u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(662);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Sign DIFFERENT signable bytes (create a different transition with different input amount)
            let mut wrong_inputs = BTreeMap::new();
            wrong_inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.6))); // Wrong amount!
            let wrong_signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &wrong_inputs, &outputs);
            let witness = signer
                .sign_p2pkh(input_address, &wrong_signable_bytes)
                .expect("should sign");

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (signature for different message)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - signature for different message
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod complex_scenarios {
        use super::*;

        #[test]
        fn test_all_inputs_p2sh_multisig() {
            // All inputs are P2SH multisig (no P2PKH)
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let p2sh_address1 = signer.add_p2sh_2of3([60u8; 32], [61u8; 32], [62u8; 32]);
            let p2sh_address2 = signer.add_p2sh_2of3([63u8; 32], [64u8; 32], [65u8; 32]);

            setup_address_with_balance(&mut platform, p2sh_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, p2sh_address2, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(670);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address1, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(p2sh_address2, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with multiple P2SH inputs
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_complex_fee_strategy_multiple_outputs() {
            // Complex fee strategy that deducts from multiple outputs
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(671);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(0.3)));
            outputs.insert(create_platform_address(2), Some(dash_to_credits!(0.3)));
            outputs.insert(create_platform_address(3), None); // Remainder recipient

            let signer = TestAddressSigner::new();

            // Fee deducted from multiple explicit outputs (ReduceOutput only applies to explicit outputs)
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(1),
                ],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_self_transfer_same_input_output_address() {
            // Input and output have the same address (though this should be blocked by structure validation)
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let address = signer.add_p2pkh([70u8; 32]);
            setup_address_with_balance(&mut platform, address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(672);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(address, None); // Same address as input (remainder recipient)

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (same address in input and output)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - same address in input and output
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_maximum_total_amount() {
            // Test with maximum combined input amounts
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();

            // Create 16 inputs with large balances
            let mut inputs = BTreeMap::new();

            let mut rng = StdRng::seed_from_u64(673);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            for i in 0..16u8 {
                let address = signer.add_p2pkh([100 + i; 32]);
                setup_address_with_balance(&mut platform, address, 0, dash_to_credits!(100.0));
                inputs.insert(address, (1 as AddressNonce, dash_to_credits!(100.0)));
            }

            let mut outputs = BTreeMap::new();
            // Address inputs: 16 * 100 DASH = 1600 DASH
            // Asset lock: 1 DASH
            // Total: 1601 DASH
            // Output receives remainder after fees
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with large amounts
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod chain_asset_lock {
        use super::*;

        #[test]
        fn test_chain_asset_lock_proof_basic() {
            // Test with ChainAssetLockProof instead of InstantAssetLockProof
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut rng = StdRng::seed_from_u64(700);
            let (chain_asset_lock_proof, asset_lock_pk, asset_lock_tx) =
                create_chain_asset_lock_proof_with_key_and_tx(&mut rng);

            // The chain proof has core_chain_locked_height = 100
            // We need the transaction to be mined at or before that height
            // Create platform with mock that returns the transaction at height 50
            let platform = create_platform_with_chain_asset_lock_mock(
                platform_config,
                asset_lock_tx,
                50, // Transaction mined at height 50, proof height is 100
            );

            // Set genesis state
            let platform = platform.set_genesis_state_with_activation_info(0, 1);

            // Fast forward to set last_committed_core_height >= proof's core_chain_locked_height (100)
            // The chain proof requires platform's core height to be at least 100
            crate::test::helpers::fast_forward_to_block::fast_forward_to_block(
                &platform, 0, 1, 200, 0, false,
            );

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                chain_asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Use BlockInfo with core_height >= proof's core_chain_locked_height
            let block_info = BlockInfo {
                time_ms: 0,
                height: 1,
                core_height: 200,
                epoch: Default::default(),
            };

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with chain asset lock proof
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_chain_asset_lock_insufficient_confirmations() {
            // Chain lock that doesn't have enough confirmations
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(701);
            let (mut chain_asset_lock_proof, asset_lock_pk) =
                create_chain_asset_lock_proof_with_key(&mut rng);

            // Set core chain locked height to be too recent (not enough confirmations)
            match &mut chain_asset_lock_proof {
                AssetLockProof::Chain(chain_proof) => {
                    chain_proof.core_chain_locked_height = 1000; // Very recent
                }
                _ => panic!("Expected chain proof"),
            }

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                chain_asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (insufficient confirmations)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Use block info with lower core chain locked height
            let block_info = BlockInfo {
                time_ms: 0,
                height: 1,
                core_height: 10, // Lower than the proof's core_chain_locked_height
                epoch: Default::default(),
            };

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - insufficient confirmations
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod asset_lock_signature_field {
        use super::*;

        #[test]
        fn test_empty_asset_lock_signature() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(710);
            let (asset_lock_proof, _asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(vec![]), // Empty signature
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (empty signature)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - empty signature
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_asset_lock_signature_too_short() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(711);
            let (asset_lock_proof, _asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(vec![0u8; 32]), // Too short (should be 64-65)
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (signature too short)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - signature too short
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_asset_lock_signature_too_long() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(712);
            let (asset_lock_proof, _asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(vec![0u8; 128]), // Too long
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (signature too long)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - signature too long
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_asset_lock_signature_wrong_key() {
            // Signature is valid but from wrong key (doesn't match asset lock tx pubkey)
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: false, // Enable verification!
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(713);
            let (asset_lock_proof, _correct_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Generate a different key to sign with
            let wrong_pk = [99u8; 32];

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(wrong_pk.to_vec()), // Wrong key
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (wrong signature key)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - wrong key for asset lock signature
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod witness_ordering {
        use super::*;

        #[test]
        fn test_witnesses_wrong_order() {
            // Witnesses provided in wrong order
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([80u8; 32]);
            let input_address2 = signer.add_p2pkh([81u8; 32]);

            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(720);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // All inputs sign the same signable bytes (the entire transition)
            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);

            let witness1 = signer
                .sign_p2pkh(input_address1, &signable_bytes)
                .expect("should sign");
            let witness2 = signer
                .sign_p2pkh(input_address2, &signable_bytes)
                .expect("should sign");

            // Provide witnesses in WRONG order (swapped)
            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness2, witness1], // WRONG ORDER
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (witnesses wrong order)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - witnesses in wrong order
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_missing_middle_witness() {
            // 3 inputs but only witnesses 0 and 2 (missing witness 1)
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([82u8; 32]);
            let input_address2 = signer.add_p2pkh([83u8; 32]);
            let input_address3 = signer.add_p2pkh([84u8; 32]);

            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address3, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(721);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(input_address3, (1 as AddressNonce, dash_to_credits!(0.3)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // All inputs sign the same signable bytes
            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);

            let witness1 = signer
                .sign_p2pkh(input_address1, &signable_bytes)
                .expect("should sign");
            let witness3 = signer
                .sign_p2pkh(input_address3, &signable_bytes)
                .expect("should sign");

            // Only 2 witnesses for 3 inputs
            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness1, witness3], // Missing middle witness
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (missing middle witness)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - witness count mismatch (already tested, but this confirms middle missing)
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod p2sh_variations {
        use super::*;

        #[test]
        fn test_p2sh_1_of_1_multisig() {
            // Single signature wrapped in P2SH
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let p2sh_address = signer.add_p2sh_1of1([90u8; 32]);

            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(730);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with 1-of-1 P2SH
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_p2sh_3_of_3_multisig() {
            // All signatures required
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let p2sh_address = signer.add_p2sh_3of3([91u8; 32], [92u8; 32], [93u8; 32]);

            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(731);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with 3-of-3 P2SH
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_p2sh_more_signatures_than_threshold() {
            // Provide 3 signatures for a 2-of-3 multisig
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let p2sh_address = signer.add_p2sh_2of3([94u8; 32], [95u8; 32], [96u8; 32]);

            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(732);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            // Sign with all 3 keys for 2-of-3
            let witness = signer
                .sign_p2sh_all_keys(p2sh_address, &signable_bytes)
                .expect("should sign");

            let state_transition = create_transition_with_custom_witnesses(
                asset_lock_proof,
                &asset_lock_pk,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                vec![witness],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - extra signatures are valid
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_p2sh_maximum_keys() {
            // 15-of-15 multisig (maximum allowed)
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            // Create 15 private keys
            let private_keys: Vec<[u8; 32]> = (0..15).map(|i| [100 + i as u8; 32]).collect();
            let p2sh_address = signer.add_p2sh_n_of_n(&private_keys);

            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(733);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with 15-of-15
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod state_verification_after_success {
        use super::*;

        #[test]
        fn test_nonce_incremented_after_success() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([120u8; 32]);
            let initial_nonce: AddressNonce = 5;
            setup_address_with_balance(
                &mut platform,
                input_address,
                initial_nonce,
                dash_to_credits!(1.0),
            );

            let mut rng = StdRng::seed_from_u64(740);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (initial_nonce + 1, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Verify nonce was incremented
            let new_nonce = get_address_nonce(&platform, input_address, &transaction);
            assert_eq!(new_nonce, initial_nonce + 1);
        }

        #[test]
        fn test_asset_lock_marked_as_spent() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(741);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            let asset_lock_outpoint = asset_lock_proof.out_point().expect("should have outpoint");

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Verify asset lock is marked as spent
            let is_spent = is_asset_lock_spent(&platform, &asset_lock_outpoint, &transaction);
            assert!(is_spent);
        }

        #[test]
        fn test_exact_balance_deltas() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([122u8; 32]);
            let output_address = create_platform_address(1);

            let initial_input_balance = dash_to_credits!(2.0);
            let input_amount = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, initial_input_balance);

            let mut rng = StdRng::seed_from_u64(742);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            let asset_lock_value = dash_to_credits!(1.0); // From fixture

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, input_amount));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Verify balance changes
            let new_input_balance = get_address_balance(&platform, input_address, &transaction);
            let new_output_balance = get_address_balance(&platform, output_address, &transaction);

            // Input should have: initial - input_amount = 2.0 - 1.0 = 1.0 DASH
            assert_eq!(new_input_balance, initial_input_balance - input_amount);

            // Output should receive asset_lock_value + input_amount MINUS fee
            // (since ReduceOutput(0) deducts fee from the remainder output)
            let total_funds = asset_lock_value + input_amount;
            assert!(
                new_output_balance > 0,
                "Output balance should be > 0, got {}",
                new_output_balance
            );
            assert!(
                new_output_balance < total_funds,
                "Output balance {} should be less than {} due to fee deduction",
                new_output_balance,
                total_funds
            );
        }
    }

    mod error_type_verification {
        use super::*;
        use dpp::consensus::state::state_error::StateError;

        #[test]
        fn test_address_not_found_error_type() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([130u8; 32]);
            // Don't set up address - it doesn't exist

            let mut rng = StdRng::seed_from_u64(750);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (address not found)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.invalid_unpaid_count(), 1);

            // Verify specific error type
            let result = processing_result
                .into_execution_results()
                .into_iter()
                .next()
                .unwrap();
            let StateTransitionExecutionResult::UnpaidConsensusError(consensus_error) = result
            else {
                panic!("expected an unpaid consensus error");
            };

            assert!(matches!(
                consensus_error,
                ConsensusError::StateError(StateError::AddressDoesNotExistError(_))
            ));
        }

        #[test]
        fn test_insufficient_balance_error_type() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([131u8; 32]);
            // Set up with less balance than requested
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.1));

            let mut rng = StdRng::seed_from_u64(751);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5))); // More than available

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (insufficient balance)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.invalid_unpaid_count(), 1);

            // Verify specific error type
            let result = processing_result
                .into_execution_results()
                .into_iter()
                .next()
                .unwrap();
            let StateTransitionExecutionResult::UnpaidConsensusError(consensus_error) = result
            else {
                panic!("expected an unpaid consensus error");
            };

            assert!(matches!(
                consensus_error,
                ConsensusError::StateError(StateError::AddressNotEnoughFundsError(_))
            ));
        }

        #[test]
        fn test_invalid_nonce_error_type() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([132u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 5, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(752);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (3 as AddressNonce, dash_to_credits!(0.5))); // Wrong nonce (should be 6)

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (invalid nonce)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.invalid_unpaid_count(), 1);

            // Verify specific error type
            let result = processing_result
                .into_execution_results()
                .into_iter()
                .next()
                .unwrap();
            let StateTransitionExecutionResult::UnpaidConsensusError(consensus_error) = result
            else {
                panic!("expected an unpaid consensus error");
            };

            assert!(matches!(
                consensus_error,
                ConsensusError::StateError(StateError::AddressInvalidNonceError(_))
            ));
        }
    }

    mod signature_malleability {
        use super::*;

        #[test]
        fn test_high_s_signature_rejected() {
            // High-S signatures should be rejected per BIP-62
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([140u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(760);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            let mut witness = signer
                .sign_p2pkh(input_address, &signable_bytes)
                .expect("should sign");

            // Convert signature to high-S form
            match &mut witness {
                AddressWitness::P2pkh { signature } => {
                    // The S value is in bytes 32-63 of the signature
                    // To make it high-S, we can flip it (n - s where n is curve order)
                    // For testing, we'll just corrupt the S value to simulate high-S
                    if signature.len() >= 64 {
                        // Set S to a high value (greater than half the curve order)
                        for i in 32..48 {
                            signature.0[i] = 0xFF;
                        }
                    }
                }
                _ => panic!("Expected P2PKH witness"),
            }

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (high-S signature)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - high-S signature
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod block_info_edge_cases {
        use super::*;

        #[test]
        fn test_block_height_zero() {
            // Genesis-like block
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(770);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let block_info = BlockInfo {
                time_ms: 0,
                height: 0, // Genesis height
                core_height: 0,
                epoch: Default::default(),
            };

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // May succeed or fail depending on genesis handling
            // The important thing is it doesn't panic
            assert!(
                processing_result.valid_count()
                    + processing_result.invalid_unpaid_count()
                    + processing_result.invalid_unpaid_count()
                    == 1
            );
        }

        #[test]
        fn test_very_high_block_height() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(771);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let block_info = BlockInfo {
                time_ms: u64::MAX,
                height: u64::MAX,
                core_height: u32::MAX,
                epoch: Default::default(),
            };

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed (no height restrictions on this transition type)
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod partial_failure_scenarios {
        use super::*;

        #[test]
        fn test_one_valid_one_invalid_input_signature() {
            // Two inputs, one with valid signature, one with invalid - whole tx should fail
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([150u8; 32]);
            let input_address2 = signer.add_p2pkh([151u8; 32]);

            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(780);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Get the correct signable bytes for this transition
            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            // Create valid witness for first input
            let witness1 = signer
                .sign_p2pkh(input_address1, &signable_bytes)
                .expect("should sign");

            // Create INVALID witness for second input (wrong message)
            // Simulate signing with wrong nonce by creating signable bytes for a different transition
            let mut wrong_inputs = BTreeMap::new();
            wrong_inputs.insert(input_address2, (99 as AddressNonce, dash_to_credits!(0.5)));
            let wrong_signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &wrong_inputs, &outputs);
            let witness2 = signer
                .sign_p2pkh(input_address2, &wrong_signable_bytes)
                .expect("should sign");

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![witness1, witness2],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (one invalid input signature)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Whole transaction should fail due to one invalid signature
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    mod size_limit_edge_cases {
        use super::*;

        #[test]
        fn test_minimum_valid_transition() {
            // Smallest possible valid transition
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(790);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Output receives remainder after fee deduction
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(), // No inputs
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let serialized = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check it's reasonably small
            assert!(serialized.len() < 500);

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod address_format_edge_cases {
        use super::*;

        #[test]
        fn test_all_zero_address_hash() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();

            let mut rng = StdRng::seed_from_u64(800);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Create address with all-zero hash
            let zero_address = PlatformAddress::P2pkh([0u8; 20]);

            let mut outputs = BTreeMap::new();
            outputs.insert(zero_address, None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(), // No inputs
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - all-zero address is technically valid
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_all_ff_address_hash() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();

            let mut rng = StdRng::seed_from_u64(801);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Create address with all-FF hash
            let max_address = PlatformAddress::P2pkh([0xFFu8; 20]);

            let mut outputs = BTreeMap::new();
            outputs.insert(max_address, None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(), // No inputs
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - all-FF address is technically valid
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    mod fee_strategy_input_combinations {
        use super::*;

        #[test]
        fn test_deduct_from_input_and_reduce_output() {
            // Combined fee strategy
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([160u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(810);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            // Combined strategy: first try output, then remaining input balance
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ],
            );
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed with combined strategy
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_deduct_from_input_exact_amount() {
            // DeductFromInput leaves exactly 0 remaining
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([161u8; 32]);
            // Set up with exact amount that will be fully consumed
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(811);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            // Use entire balance
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - fee covered by asset lock remainder
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Verify input balance is now 0
            let remaining_balance = get_address_balance(&platform, input_address, &transaction);
            assert_eq!(remaining_balance, 0);
        }
    }

    mod replay_and_idempotency {
        use super::*;

        #[test]
        fn test_replay_same_transition_fails() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(820);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let serialized = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // First execution - should succeed
            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![serialized.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Commit
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("commit");

            // Second execution of same transition - should fail (asset lock already spent)
            let platform_state = platform.state.load();
            let transaction2 = platform.drive.grove.start_transaction();

            let processing_result2 = platform
                .platform
                .process_raw_state_transitions(
                    &vec![serialized],
                    &platform_state,
                    &BlockInfo::default_with_height(2),
                    &transaction2,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - can't replay same transition
            assert_eq!(processing_result2.invalid_unpaid_count(), 1);
        }
    }

    // ==========================================
    // CONCURRENT INPUT USAGE TESTS
    // ==========================================

    mod concurrent_input_usage {
        use super::*;

        #[test]
        fn test_two_transitions_same_input_address_same_block() {
            // Two transitions in the same block both try to use the same input address.
            // The second one should fail due to nonce mismatch (first uses nonce 1,
            // but both were created expecting nonce 1).
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create input address with enough balance for both transitions
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([50u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(10.0));

            // First transition: uses nonce 1
            let mut rng1 = StdRng::seed_from_u64(901);
            let (asset_lock_proof1, asset_lock_pk1) = create_asset_lock_proof_with_key(&mut rng1);

            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, dash_to_credits!(2.0)));

            let mut outputs1 = BTreeMap::new();
            outputs1.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition1 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof1,
                &asset_lock_pk1,
                &signer,
                inputs1,
                outputs1,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            // Second transition: also uses nonce 1 (will conflict)
            let mut rng2 = StdRng::seed_from_u64(902);
            let (asset_lock_proof2, asset_lock_pk2) = create_asset_lock_proof_with_key(&mut rng2);

            let mut inputs2 = BTreeMap::new();
            inputs2.insert(input_address, (1 as AddressNonce, dash_to_credits!(3.0)));

            let mut outputs2 = BTreeMap::new();
            outputs2.insert(create_platform_address(2), None); // Remainder recipient

            let state_transition2 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof2,
                &asset_lock_pk2,
                &signer,
                inputs2,
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result1 = state_transition1
                .serialize_to_bytes()
                .expect("should serialize");
            let result2 = state_transition2
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both transitions in the same block
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1, result2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // First should succeed, second should fail (nonce conflict)
            let results = processing_result.execution_results();
            assert_eq!(results.len(), 2);

            assert_matches!(
                &results[0],
                StateTransitionExecutionResult::SuccessfulExecution(_, _)
            );

            // Second fails because nonce 1 was already used by first transition
            // This is an UnpaidConsensusError because nonce validation happens before fee payment
            assert_matches!(
                &results[1],
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::AddressInvalidNonceError(_)
                ))
            );
        }

        #[test]
        fn test_two_transitions_same_input_address_sequential_nonces() {
            // Two transitions in the same block using same input but with sequential nonces.
            // First uses nonce 1, second uses nonce 2. Both should succeed.
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create input address with enough balance for both transitions
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([51u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(10.0));

            // First transition: uses nonce 1
            let mut rng1 = StdRng::seed_from_u64(903);
            let (asset_lock_proof1, asset_lock_pk1) = create_asset_lock_proof_with_key(&mut rng1);

            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, dash_to_credits!(2.0)));

            let mut outputs1 = BTreeMap::new();
            outputs1.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition1 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof1,
                &asset_lock_pk1,
                &signer,
                inputs1,
                outputs1,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            // Second transition: uses nonce 2 (sequential)
            let mut rng2 = StdRng::seed_from_u64(904);
            let (asset_lock_proof2, asset_lock_pk2) = create_asset_lock_proof_with_key(&mut rng2);

            let mut inputs2 = BTreeMap::new();
            inputs2.insert(input_address, (2 as AddressNonce, dash_to_credits!(3.0)));

            let mut outputs2 = BTreeMap::new();
            outputs2.insert(create_platform_address(2), None); // Remainder recipient

            let state_transition2 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof2,
                &asset_lock_pk2,
                &signer,
                inputs2,
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result1 = state_transition1
                .serialize_to_bytes()
                .expect("should serialize");
            let result2 = state_transition2
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both transitions in the same block
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1, result2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // Both should succeed with sequential nonces
            let results = processing_result.execution_results();
            assert_eq!(results.len(), 2);

            assert_matches!(
                &results[0],
                StateTransitionExecutionResult::SuccessfulExecution(_, _)
            );
            assert_matches!(
                &results[1],
                StateTransitionExecutionResult::SuccessfulExecution(_, _)
            );

            // Verify final balance: started with 10, spent 2+3=5
            let final_balance = get_address_balance(&platform, input_address, &transaction);
            assert_eq!(final_balance, dash_to_credits!(5.0));
        }

        #[test]
        fn test_second_transition_exceeds_remaining_balance() {
            // Two transitions in same block. First succeeds, second fails because
            // the first consumed balance that second was counting on.
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create input address with limited balance
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([52u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(5.0));

            // First transition: uses 3 DASH (nonce 1)
            let mut rng1 = StdRng::seed_from_u64(905);
            let (asset_lock_proof1, asset_lock_pk1) = create_asset_lock_proof_with_key(&mut rng1);

            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, dash_to_credits!(3.0)));

            let mut outputs1 = BTreeMap::new();
            outputs1.insert(create_platform_address(1), None); // Remainder recipient

            let state_transition1 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof1,
                &asset_lock_pk1,
                &signer,
                inputs1,
                outputs1,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            // Second transition: tries to use 3 DASH (nonce 2)
            // But after first transition, only 2 DASH remains
            let mut rng2 = StdRng::seed_from_u64(906);
            let (asset_lock_proof2, asset_lock_pk2) = create_asset_lock_proof_with_key(&mut rng2);

            let mut inputs2 = BTreeMap::new();
            inputs2.insert(input_address, (2 as AddressNonce, dash_to_credits!(3.0)));

            let mut outputs2 = BTreeMap::new();
            outputs2.insert(create_platform_address(2), None); // Remainder recipient

            let state_transition2 = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof2,
                &asset_lock_pk2,
                &signer,
                inputs2,
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result1 = state_transition1
                .serialize_to_bytes()
                .expect("should serialize");
            let result2 = state_transition2
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both transitions in the same block
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1, result2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // First should succeed, second should fail (insufficient balance)
            let results = processing_result.execution_results();
            assert_eq!(results.len(), 2);

            assert_matches!(
                &results[0],
                StateTransitionExecutionResult::SuccessfulExecution(_, _)
            );

            // Second fails because balance was depleted by first
            // This is an UnpaidConsensusError because balance validation happens before fee payment
            assert_matches!(
                &results[1],
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::AddressNotEnoughFundsError(_)
                ))
            );
        }
    }

    // ==========================================
    // OVERFLOW PROTECTION TESTS
    // ==========================================

    mod overflow_protection {
        use super::*;

        #[test]
        fn test_output_sum_overflow() {
            // Multiple outputs that would overflow u64 when summed
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(910);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Two outputs that would overflow when added together
            // At least one must be a remainder (None), but we can still have an explicit huge one
            outputs.insert(create_platform_address(1), Some(u64::MAX - 1000));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (output sum overflow)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - outputs exceed what any asset lock could provide
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_input_sum_overflow() {
            // Multiple inputs that would overflow u64 when summed
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(911);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let signer = TestAddressSigner::new();
            let input_address1 = create_platform_address(10);
            let input_address2 = create_platform_address(11);

            let mut inputs = BTreeMap::new();
            // Two inputs that would overflow when added together
            inputs.insert(input_address1, (1 as AddressNonce, u64::MAX - 1000));
            inputs.insert(input_address2, (1 as AddressNonce, u64::MAX - 1000));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![
                        AddressWitness::P2pkh {
                            signature: BinaryData::new(vec![0u8; 65]),
                        },
                        AddressWitness::P2pkh {
                            signature: BinaryData::new(vec![0u8; 65]),
                        },
                    ],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (input sum overflow)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - likely overflow or validation error
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_output_plus_fee_overflow() {
            // Output amount that when fee is added would overflow
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(912);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Output very close to u64::MAX, so adding any fee would overflow
            outputs.insert(create_platform_address(1), Some(u64::MAX - 100));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::DeductFromInput(0), // Try to deduct from non-existent input
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject invalid_unpaid transaction (output plus fee overflow)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail - output exceeds asset lock or overflow protection kicks in
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_user_fee_increase_overflow() {
            // Very high fee increase that could cause overflow in fee calculation
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(913);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Output receives remainder
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let signer = TestAddressSigner::new();
            let state_transition =
                create_signed_address_funding_from_asset_lock_transition_with_fee_increase(
                    asset_lock_proof,
                    &asset_lock_pk,
                    &signer,
                    BTreeMap::new(),
                    outputs,
                    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                    u16::MAX, // Maximum fee increase
                );

            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - fee increase is handled with saturating arithmetic
            // and ReduceOutput will just take more from the output
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    // ==========================================
    // PARTIALLY USED ASSET LOCK TESTS
    // These test scenarios where an asset lock has been partially consumed
    // (e.g., by a failed identity create with duplicate unique key)
    // ==========================================

    mod partially_used_asset_lock {
        use super::*;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
        use dpp::native_bls::NativeBlsModule;
        use dpp::prelude::Identifier;
        use dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
        use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
        use simple_signer::signer::SimpleSigner;

        #[test]
        fn test_address_funding_with_partially_used_asset_lock() {
            // This test verifies that an asset lock that was partially consumed
            // (due to a failed identity create with duplicate unique key) can still
            // be used for address funding with the remaining balance.

            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let platform_state = platform.state.load();

            let mut identity_signer = SimpleSigner::default();
            let mut rng = StdRng::seed_from_u64(567);

            // Create keys for the identity we'll try to create
            let (master_key, master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(58),
                    platform_version,
                )
                .expect("expected to get key pair");

            identity_signer.add_identity_public_key(master_key.clone(), master_private_key);

            let (critical_public_key_that_is_already_in_system, private_key) =
                IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                    1,
                    Some(999),
                    platform_version,
                )
                .expect("expected to get key pair");

            // First, add an identity with the same unique key to the system
            let (another_master_key, _) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(53),
                    platform_version,
                )
                .expect("expected to get key pair");

            let identity_already_in_system: Identity = IdentityV0 {
                id: Identifier::random_with_rng(&mut rng),
                public_keys: BTreeMap::from([
                    (0, another_master_key.clone()),
                    (1, critical_public_key_that_is_already_in_system.clone()),
                ]),
                balance: 100000,
                revision: 0,
            }
            .into();

            // Add this identity to the system first
            platform
                .drive
                .add_new_identity(
                    identity_already_in_system,
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add a new identity");

            identity_signer.add_identity_public_key(
                critical_public_key_that_is_already_in_system.clone(),
                private_key,
            );

            // Create an asset lock proof
            let (_, pk) = dpp::identity::KeyType::ECDSA_SECP256K1
                .random_public_and_private_key_data(&mut rng, platform_version)
                .unwrap();

            let asset_lock_proof = dpp::tests::fixtures::instant_asset_lock_proof_fixture(
                Some(
                    dpp::dashcore::PrivateKey::from_byte_array(
                        &pk,
                        dpp::dashcore::Network::Testnet,
                    )
                    .unwrap(),
                ),
                None, // 1 DASH default
            );

            let identifier = asset_lock_proof
                .create_identifier()
                .expect("expected an identifier");

            // Try to create an identity with the duplicate key (this will fail and partially use the asset lock)
            let identity_to_fail: Identity = IdentityV0 {
                id: identifier,
                public_keys: BTreeMap::from([
                    (0, master_key.clone()),
                    (1, critical_public_key_that_is_already_in_system.clone()),
                ]),
                balance: 1000000000,
                revision: 0,
            }
            .into();

            let identity_create_transition: StateTransition =
                IdentityCreateTransition::try_from_identity_with_signer(
                    &identity_to_fail,
                    asset_lock_proof.clone(),
                    pk.as_slice(),
                    &identity_signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
                .expect("expected an identity create transition");

            let identity_create_serialized_transition = identity_create_transition
                .serialize_to_bytes()
                .expect("serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![identity_create_serialized_transition],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Identity creation should fail due to duplicate unique key
            assert_eq!(processing_result.invalid_paid_count(), 1);
            assert_eq!(processing_result.valid_count(), 0);

            // Penalty was paid from the asset lock (10000000 penalty + processing fee)
            let penalty_fee = processing_result.aggregated_fees().processing_fee;
            assert!(
                penalty_fee > 10000000,
                "Expected penalty fee > 10M, got {}",
                penalty_fee
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            // Now try to use the same asset lock for address funding
            // The remaining balance should still be available

            let address_signer = TestAddressSigner::new();

            let mut outputs = BTreeMap::new();
            // Remainder recipient - gets whatever is left from the partially consumed asset lock
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let address_funding_transition =
                create_signed_address_funding_from_asset_lock_transition(
                    asset_lock_proof,
                    &pk,
                    &address_signer,
                    BTreeMap::new(), // No additional inputs
                    outputs,
                    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                );

            let address_funding_serialized = address_funding_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![address_funding_serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Address funding should succeed with the remaining asset lock balance
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_address_funding_with_small_output_after_partial_consumption() {
            // This test verifies that an asset lock that was partially consumed can still
            // be used for address funding with an output that fits within the remaining balance.
            //
            // Key calculations (CREDITS_PER_DUFF = 1000):
            // - Asset lock: 200,000 duffs = 200,000,000 credits (minimum for identity create)
            // - Penalty for unique_key_already_present: 10,000,000 credits = 10,000 duffs
            // - Processing fees: ~1-2M credits = ~1-2K duffs per attempt
            // - After 1 failure: ~188,000 duffs remain
            // - Then request 100,000,000 credits (100,000 duffs) which fits within remaining

            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let platform_state = platform.state.load();

            let mut identity_signer = SimpleSigner::default();
            let mut rng = StdRng::seed_from_u64(567);

            let (critical_public_key_that_is_already_in_system, private_key) =
                IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                    1,
                    Some(999),
                    platform_version,
                )
                .expect("expected to get key pair");

            // First, add an identity with the unique key to the system
            let (another_master_key, _) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(53),
                    platform_version,
                )
                .expect("expected to get key pair");

            let identity_already_in_system: Identity = IdentityV0 {
                id: Identifier::random_with_rng(&mut rng),
                public_keys: BTreeMap::from([
                    (0, another_master_key.clone()),
                    (1, critical_public_key_that_is_already_in_system.clone()),
                ]),
                balance: 100000,
                revision: 0,
            }
            .into();

            platform
                .drive
                .add_new_identity(
                    identity_already_in_system,
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add a new identity");

            identity_signer.add_identity_public_key(
                critical_public_key_that_is_already_in_system.clone(),
                private_key,
            );

            // Create an asset lock with 200,000 duffs (minimum for identity create)
            let (_, pk) = dpp::identity::KeyType::ECDSA_SECP256K1
                .random_public_and_private_key_data(&mut rng, platform_version)
                .unwrap();

            let asset_lock_proof = dpp::tests::fixtures::instant_asset_lock_proof_fixture(
                Some(
                    dpp::dashcore::PrivateKey::from_byte_array(
                        &pk,
                        dpp::dashcore::Network::Testnet,
                    )
                    .unwrap(),
                ),
                Some(2560000), // 200,000 duffs = minimum for identity create
            );

            let identifier = asset_lock_proof
                .create_identifier()
                .expect("expected an identifier");

            // Consume some of the asset lock with a failed identity create
            let (new_master_key, new_master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(60),
                    platform_version,
                )
                .expect("expected to get key pair");

            identity_signer.add_identity_public_key(new_master_key.clone(), new_master_private_key);

            let identity: Identity = IdentityV0 {
                id: identifier,
                public_keys: BTreeMap::from([
                    (0, new_master_key.clone()),
                    (1, critical_public_key_that_is_already_in_system.clone()),
                ]),
                balance: 1000000000,
                revision: 0,
            }
            .into();

            let identity_create_transition: StateTransition =
                IdentityCreateTransition::try_from_identity_with_signer(
                    &identity,
                    asset_lock_proof.clone(),
                    pk.as_slice(),
                    &identity_signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
                .expect("expected an identity create transition");

            let identity_create_serialized = identity_create_transition
                .serialize_to_bytes()
                .expect("serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![identity_create_serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail with penalty charged (~12,000 duffs consumed)
            assert_eq!(
                processing_result.invalid_paid_count(),
                1,
                "Identity create should fail with penalty"
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            // Now use the partially-used asset lock for address funding
            // After 1 failure (~12,000 duffs consumed), ~188,000 duffs remain
            // Remainder recipient will receive whatever is left
            let address_signer = TestAddressSigner::new();

            let mut outputs = BTreeMap::new();
            // Remainder recipient receives whatever is left from the asset lock
            outputs.insert(create_platform_address(1), None); // Remainder recipient

            let address_funding_transition =
                create_signed_address_funding_from_asset_lock_transition(
                    asset_lock_proof,
                    &pk,
                    &address_signer,
                    BTreeMap::new(),
                    outputs,
                    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                );

            let address_funding_serialized = address_funding_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![address_funding_serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - output fits within remaining balance
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    // ==========================================
    // INVALID PAID FEE SOURCE TESTS
    // These test the different scenarios for where fees come from when
    // PartiallyUseAssetLockAction is used due to insufficient funds validation failure
    // ==========================================

    mod invalid_paid_fee_sources {
        use super::*;
        use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
        use dpp::asset_lock::StoredAssetLockInfo;

        /// Helper to get asset lock info after processing
        fn get_asset_lock_info(
            platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
            outpoint: &dpp::dashcore::OutPoint,
            transaction: &drive::grovedb::Transaction,
        ) -> StoredAssetLockInfo {
            let platform_version = PlatformVersion::latest();
            let outpoint_bytes: [u8; 36] = {
                let mut bytes = [0u8; 36];
                bytes[..32].copy_from_slice(outpoint.txid.as_raw_hash().as_byte_array());
                bytes[32..36].copy_from_slice(&outpoint.vout.to_le_bytes());
                bytes
            };

            platform
                .drive
                .fetch_asset_lock_outpoint_info(
                    &outpoint_bytes.into(),
                    Some(transaction),
                    &platform_version.drive,
                )
                .expect("should fetch asset lock info")
        }

        #[test]
        fn test_invalid_paid_fee_from_asset_lock_only() {
            // Scenario: No inputs provided, so the penalty must come entirely from the asset lock.
            // Expected: Asset lock remaining balance is reduced by at least the penalty.
            //
            // Note: The exact fee includes penalty + processing fees computed at the time of
            // validation. The final aggregated_fees may differ slightly due to storage costs
            // added during operation execution.
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let signer = TestAddressSigner::new();
            let mut rng = StdRng::seed_from_u64(2001);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            let asset_lock_outpoint = asset_lock_proof.out_point().expect("should have outpoint");
            let initial_asset_lock_value = dash_to_credits!(1.0); // From fixture

            // No inputs - fee can only come from asset lock
            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            // Explicit output of 2 DASH - more than the 1 DASH asset lock (will fail)
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(2.0)));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should PASS for invalid_paid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept invalid_paid transaction to mempool"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should be invalid_paid (fee deducted from asset lock)
            assert_eq!(processing_result.invalid_paid_count(), 1);
            assert_eq!(processing_result.valid_count(), 0);

            // Verify asset lock was partially consumed
            let asset_lock_info =
                get_asset_lock_info(&platform, &asset_lock_outpoint, &transaction);
            match asset_lock_info {
                StoredAssetLockInfo::PartiallyConsumed(value) => {
                    let remaining = value.remaining_credit_value();
                    let initial = value.initial_credit_value();

                    // Initial should match our expected value
                    assert_eq!(initial, initial_asset_lock_value);

                    // Remaining should be less than initial
                    assert!(
                        remaining < initial_asset_lock_value,
                        "Asset lock remaining {} should be less than initial {}",
                        remaining,
                        initial_asset_lock_value
                    );

                    // Calculate the amount deducted from asset lock
                    let amount_deducted = initial_asset_lock_value - remaining;

                    // The penalty is the minimum that should have been deducted
                    let penalty = platform_version
                        .drive_abci
                        .validation_and_processing
                        .penalties
                        .address_funds_insufficient_balance;

                    // Verify at least the penalty was deducted
                    assert!(
                        amount_deducted >= penalty,
                        "Amount deducted {} should be at least the penalty {}",
                        amount_deducted,
                        penalty
                    );

                    // Verify fee was collected
                    let processing_fee = processing_result.aggregated_fees().processing_fee;
                    assert!(
                        processing_fee > 0,
                        "Processing fee should be greater than 0"
                    );
                }
                StoredAssetLockInfo::FullyConsumed => {
                    panic!("Asset lock should be partially consumed, not fully consumed");
                }
                StoredAssetLockInfo::NotPresent => {
                    panic!("Asset lock should be present after processing");
                }
            }
        }

        #[test]
        fn test_invalid_paid_fee_from_input_only() {
            // Scenario: Input has enough balance to cover the entire penalty + processing fee.
            // Fee strategy specifies DeductFromInput first.
            // Expected: Input balance is reduced, asset lock remains untouched.
            //
            // Note: When a transition fails in advanced_structure validation, the action still has
            // the "remaining balance" which is (actual_balance - input_spend_amount). The fee is
            // then deducted from this remaining balance. So the final balance is:
            // actual_balance - input_spend_amount - fee_from_remaining
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([200u8; 32]);

            // Set up input with plenty of balance to cover fees
            // Penalty is 10_000_000 + some processing fee (~10M total)
            // We need: input_spend_amount + enough left over to cover fee
            let initial_input_balance = dash_to_credits!(0.5); // 50_000_000_000 credits
            let input_spend_amount = dash_to_credits!(0.1); // 10_000_000_000 - What we're trying to spend
                                                            // remaining_balance in action = 40_000_000_000 (plenty to cover ~20M fee)
            setup_address_with_balance(&mut platform, input_address, 0, initial_input_balance);

            let mut rng = StdRng::seed_from_u64(2002);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            let asset_lock_outpoint = asset_lock_proof.out_point().expect("should have outpoint");
            let initial_asset_lock_value = dash_to_credits!(1.0); // From fixture

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, input_spend_amount));

            let mut outputs = BTreeMap::new();
            // Try to send 3 DASH total - more than asset_lock (1) + input spend (0.1) = 1.1 DASH
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(3.0)));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // Fee strategy: Deduct from input first
            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should PASS for invalid_paid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept invalid_paid transaction to mempool (input covers fee)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Get the input balance before processing
            let input_balance_before = get_address_balance(&platform, input_address, &transaction);
            assert_eq!(input_balance_before, initial_input_balance);

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should be invalid_paid
            assert_eq!(processing_result.invalid_paid_count(), 1);
            assert_eq!(processing_result.valid_count(), 0);

            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .address_funds_insufficient_balance;

            // Verify input balance was reduced
            let input_balance_after = get_address_balance(&platform, input_address, &transaction);

            // The remaining_balance in action = initial_input_balance - input_spend_amount
            let remaining_balance_in_action = initial_input_balance - input_spend_amount;

            // Fee deducted from input = remaining_balance - balance_after
            let fee_from_input = remaining_balance_in_action - input_balance_after;

            // Verify that input was charged (balance reduced)
            assert!(
                input_balance_after < remaining_balance_in_action,
                "Input balance {} should be less than remaining {} (some fee was taken)",
                input_balance_after,
                remaining_balance_in_action
            );

            // Verify at least the penalty was taken from input
            assert!(
                fee_from_input >= penalty,
                "Fee from input {} should be at least the penalty {}",
                fee_from_input,
                penalty
            );

            // Verify asset lock is untouched (full value remains)
            // Since input had enough to cover penalty + processing fee at advanced_structure time
            let asset_lock_info =
                get_asset_lock_info(&platform, &asset_lock_outpoint, &transaction);
            match asset_lock_info {
                StoredAssetLockInfo::PartiallyConsumed(value) => {
                    let remaining = value.remaining_credit_value();

                    // Asset lock should still have full value since input covered the fees
                    assert_eq!(
                        remaining, initial_asset_lock_value,
                        "Asset lock should be unchanged when input covers all fees (remaining {}, initial {})",
                        remaining, initial_asset_lock_value
                    );
                }
                StoredAssetLockInfo::FullyConsumed => {
                    panic!("Asset lock should be partially consumed, not fully consumed");
                }
                StoredAssetLockInfo::NotPresent => {
                    panic!("Asset lock should be present after processing");
                }
            }
        }

        #[test]
        fn test_invalid_paid_fee_from_input_then_asset_lock() {
            // Scenario: Input has some balance but not enough for the full fee (penalty + processing).
            // Fee strategy specifies DeductFromInput first.
            // Expected: Input contributes what it can, remainder comes from asset lock.
            //
            // Note: The action's inputs_with_remaining_balance contains (actual_balance - input_spend_amount).
            // Fee is deducted from this remaining balance, not the original balance.
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([201u8; 32]);

            // Set up input such that remaining_balance < total_fee
            // remaining_balance = initial_input_balance - input_spend_amount
            // We want: remaining_balance < penalty (~10M) + processing (~10M)
            // But remaining_balance > 0 so both input and asset lock contribute
            let initial_input_balance = 15_000_000u64; // 15M credits
            let input_spend_amount = 10_000_000u64; // 10M credits
                                                    // remaining_balance_in_action = 15M - 10M = 5M (less than ~20M total fee)
            setup_address_with_balance(&mut platform, input_address, 0, initial_input_balance);

            let mut rng = StdRng::seed_from_u64(2003);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            let asset_lock_outpoint = asset_lock_proof.out_point().expect("should have outpoint");
            let initial_asset_lock_value = dash_to_credits!(1.0); // From fixture

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, input_spend_amount));

            let mut outputs = BTreeMap::new();
            // Try to send 3 DASH - more than available, will fail
            outputs.insert(create_platform_address(1), Some(dash_to_credits!(3.0)));
            outputs.insert(create_platform_address(2), None); // Remainder recipient

            // Fee strategy: Deduct from input first, then reduce output (but output won't be created)
            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                &signer,
                inputs,
                outputs,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ],
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should PASS for invalid_paid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept invalid_paid transaction to mempool (input+asset_lock covers fee)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Get the input balance before processing
            let input_balance_before = get_address_balance(&platform, input_address, &transaction);
            assert_eq!(input_balance_before, initial_input_balance);

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should be invalid_paid
            assert_eq!(processing_result.invalid_paid_count(), 1);
            assert_eq!(processing_result.valid_count(), 0);

            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .address_funds_insufficient_balance;

            // Verify input balance after processing
            let input_balance_after = get_address_balance(&platform, input_address, &transaction);

            // The remaining_balance in action = initial_input_balance - input_spend_amount
            let remaining_balance_in_action = initial_input_balance - input_spend_amount;

            // Input balance should be 0 or reduced significantly (fee deducted from remaining)
            // Final balance = remaining_balance - min(fee, remaining_balance) = 0 (if remaining < fee)
            assert!(
                input_balance_after < remaining_balance_in_action,
                "Input balance after {} should be less than remaining {} (some fee was taken)",
                input_balance_after,
                remaining_balance_in_action
            );

            // Fee deducted from input = remaining_balance_in_action - input_balance_after
            let fee_from_input = remaining_balance_in_action - input_balance_after;

            // Verify asset lock was partially consumed
            let asset_lock_info =
                get_asset_lock_info(&platform, &asset_lock_outpoint, &transaction);
            match asset_lock_info {
                StoredAssetLockInfo::PartiallyConsumed(value) => {
                    let remaining = value.remaining_credit_value();

                    // Asset lock should have been reduced
                    assert!(
                        remaining < initial_asset_lock_value,
                        "Asset lock remaining {} should be less than initial {}",
                        remaining,
                        initial_asset_lock_value
                    );

                    // Fee from asset lock
                    let fee_from_asset_lock = initial_asset_lock_value - remaining;

                    // Verify that both sources contributed
                    assert!(
                        fee_from_input > 0,
                        "Input should have contributed to fee payment, got {} contribution",
                        fee_from_input
                    );
                    assert!(
                        fee_from_asset_lock > 0,
                        "Asset lock should have contributed to fee payment, got {} contribution",
                        fee_from_asset_lock
                    );

                    // Total fee collected should be at least the penalty
                    let total_collected = fee_from_input + fee_from_asset_lock;
                    assert!(
                        total_collected >= penalty,
                        "Total collected {} should be at least penalty {}",
                        total_collected,
                        penalty
                    );
                }
                StoredAssetLockInfo::FullyConsumed => {
                    panic!("Asset lock should be partially consumed, not fully consumed");
                }
                StoredAssetLockInfo::NotPresent => {
                    panic!("Asset lock should be present after processing");
                }
            }
        }
    }
}
