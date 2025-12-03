#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
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
    use dpp::dashcore::blockdata::opcodes::all::*;
    use dpp::dashcore::blockdata::script::ScriptBuf;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::secp256k1::{
        PublicKey as RawPublicKey, Secp256k1, SecretKey as RawSecretKey,
    };
    use dpp::dashcore::PublicKey;
    use dpp::identity::core_script::CoreScript;
    use dpp::identity::signer::Signer;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::AddressNonce;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
    use dpp::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
    use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
    use dpp::state_transition::StateTransition;
    use dpp::withdrawal::Pooling;
    use dpp::ProtocolError;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use std::collections::{BTreeMap, HashMap};

    // ==========================================
    // Test Infrastructure - Signer
    // ==========================================

    /// A P2PKH key entry containing the secret key only
    /// (public key is recovered from signature during verification)
    #[derive(Debug, Clone)]
    struct P2pkhKeyEntry {
        secret_key: RawSecretKey,
    }

    /// A P2SH multisig entry containing multiple secret keys and the redeem script
    /// (public keys are embedded in the redeem script)
    #[derive(Debug, Clone)]
    struct P2shMultisigEntry {
        /// The threshold (M in M-of-N)
        threshold: u8,
        /// Secret keys for all participants
        secret_keys: Vec<RawSecretKey>,
        /// The redeem script (contains the public keys)
        redeem_script: Vec<u8>,
    }

    /// A test signer that can sign for P2PKH and P2SH multisig addresses
    #[derive(Debug, Default)]
    struct TestAddressSigner {
        p2pkh_keys: HashMap<[u8; 20], P2pkhKeyEntry>,
        p2sh_entries: HashMap<[u8; 20], P2shMultisigEntry>,
    }

    impl TestAddressSigner {
        fn new() -> Self {
            Self::default()
        }

        /// Creates a keypair from a 32-byte seed
        fn create_keypair(seed: [u8; 32]) -> (RawSecretKey, PublicKey) {
            let secp = Secp256k1::new();
            let secret_key = RawSecretKey::from_byte_array(&seed).expect("valid secret key");
            let raw_public_key = RawPublicKey::from_secret_key(&secp, &secret_key);
            let public_key = PublicKey::new(raw_public_key);
            (secret_key, public_key)
        }

        /// Signs data with a secret key
        fn sign_data(data: &[u8], secret_key: &RawSecretKey) -> Vec<u8> {
            dpp::dashcore::signer::sign(data, secret_key.as_ref())
                .expect("signing should succeed")
                .to_vec()
        }

        /// Creates a standard multisig redeem script
        fn create_multisig_script(threshold: u8, pubkeys: &[PublicKey]) -> Vec<u8> {
            let mut script = Vec::new();
            script.push(OP_PUSHNUM_1.to_u8() + threshold - 1);
            for pubkey in pubkeys {
                let bytes = pubkey.to_bytes();
                script.push(bytes.len() as u8);
                script.extend_from_slice(&bytes);
            }
            script.push(OP_PUSHNUM_1.to_u8() + pubkeys.len() as u8 - 1);
            script.push(OP_CHECKMULTISIG.to_u8());
            script
        }

        /// Adds a P2PKH address with the given seed, returns the address
        fn add_p2pkh(&mut self, seed: [u8; 32]) -> PlatformAddress {
            let (secret_key, public_key) = Self::create_keypair(seed);
            let pubkey_hash = *public_key.pubkey_hash().as_byte_array();
            self.p2pkh_keys
                .insert(pubkey_hash, P2pkhKeyEntry { secret_key });
            PlatformAddress::P2pkh(pubkey_hash)
        }

        /// Adds a P2SH multisig address with the given seeds, returns the address
        fn add_p2sh_multisig(&mut self, threshold: u8, seeds: &[[u8; 32]]) -> PlatformAddress {
            let keypairs: Vec<_> = seeds.iter().map(|s| Self::create_keypair(*s)).collect();
            let secret_keys: Vec<_> = keypairs.iter().map(|(sk, _)| *sk).collect();
            let public_keys: Vec<_> = keypairs.iter().map(|(_, pk)| *pk).collect();

            let redeem_script = Self::create_multisig_script(threshold, &public_keys);
            let script_buf = ScriptBuf::from_bytes(redeem_script.clone());
            let script_hash = *script_buf.script_hash().as_byte_array();

            self.p2sh_entries.insert(
                script_hash,
                P2shMultisigEntry {
                    threshold,
                    secret_keys,
                    redeem_script,
                },
            );

            PlatformAddress::P2sh(script_hash)
        }
    }

    impl Signer<PlatformAddress> for TestAddressSigner {
        fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
            match key {
                PlatformAddress::P2pkh(hash) => {
                    let entry = self.p2pkh_keys.get(hash).ok_or_else(|| {
                        ProtocolError::Generic(format!(
                            "No P2PKH key found for address hash {}",
                            hex::encode(hash)
                        ))
                    })?;
                    let signature = Self::sign_data(data, &entry.secret_key);
                    Ok(BinaryData::new(signature))
                }
                PlatformAddress::P2sh(hash) => {
                    let entry = self.p2sh_entries.get(hash).ok_or_else(|| {
                        ProtocolError::Generic(format!(
                            "No P2SH entry found for script hash {}",
                            hex::encode(hash)
                        ))
                    })?;
                    // Return concatenated signatures for multisig
                    let mut all_sigs = Vec::new();
                    for sk in &entry.secret_keys[..entry.threshold as usize] {
                        all_sigs.extend(Self::sign_data(data, sk));
                    }
                    Ok(BinaryData::new(all_sigs))
                }
            }
        }

        fn sign_create_witness(
            &self,
            key: &PlatformAddress,
            data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            match key {
                PlatformAddress::P2pkh(hash) => {
                    let entry = self.p2pkh_keys.get(hash).ok_or_else(|| {
                        ProtocolError::Generic(format!(
                            "No P2PKH key found for address hash {}",
                            hex::encode(hash)
                        ))
                    })?;
                    let signature = Self::sign_data(data, &entry.secret_key);
                    // P2PKH witness only needs the signature - the public key is recovered
                    // during verification, saving 33 bytes per witness
                    Ok(AddressWitness::P2pkh {
                        signature: BinaryData::new(signature),
                    })
                }
                PlatformAddress::P2sh(hash) => {
                    let entry = self.p2sh_entries.get(hash).ok_or_else(|| {
                        ProtocolError::Generic(format!(
                            "No P2SH entry found for script hash {}",
                            hex::encode(hash)
                        ))
                    })?;
                    // Sign with threshold number of keys (first M keys)
                    let signatures: Vec<BinaryData> = entry
                        .secret_keys
                        .iter()
                        .take(entry.threshold as usize)
                        .map(|sk| BinaryData::new(Self::sign_data(data, sk)))
                        .collect();

                    Ok(AddressWitness::P2sh {
                        signatures,
                        redeem_script: BinaryData::new(entry.redeem_script.clone()),
                    })
                }
            }
        }

        fn can_sign_with(&self, key: &PlatformAddress) -> bool {
            match key {
                PlatformAddress::P2pkh(hash) => self.p2pkh_keys.contains_key(hash),
                PlatformAddress::P2sh(hash) => self.p2sh_entries.contains_key(hash),
            }
        }
    }

    // ==========================================
    // Helper Functions
    // ==========================================

    /// Helper function to create a platform address from a seed (for output addresses that don't need signing)
    fn create_platform_address(seed: u8) -> PlatformAddress {
        let mut hash = [0u8; 20];
        hash[0] = seed;
        hash[19] = seed;
        PlatformAddress::P2pkh(hash)
    }

    /// Helper function to create a dummy P2PKH witness for testing structure validation
    /// (used for tests that should fail before witness validation)
    fn create_dummy_witness() -> AddressWitness {
        // P2PKH witness only needs the signature - public key is recovered during verification
        AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]), // dummy signature
        }
    }

    /// Create a random CoreScript for withdrawal output
    fn create_random_output_script(rng: &mut StdRng) -> CoreScript {
        use rand::Rng;
        CoreScript::random_p2pkh(rng)
    }

    /// Helper function to set up an address with balance and nonce in the drive
    /// Also adds the balance to system credits since withdrawals remove from system credits
    fn setup_address_with_balance(
        platform: &mut crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let mut drive_operations = Vec::new();

        // Add to system credits first (withdrawals remove from system credits)
        platform
            .drive
            .add_to_system_credits(balance, None, platform_version)
            .expect("expected to add to system credits");

        platform
            .drive
            .set_balance_to_address(
                address,
                nonce,
                balance,
                &mut None,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to set balance to address");

        platform
            .drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                drive_operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to apply drive operations");
    }

    /// Create a raw AddressCreditWithdrawalTransitionV0 with dummy witnesses for structure validation tests
    fn create_raw_withdrawal_transition_with_dummy_witnesses(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        output: Option<(PlatformAddress, u64)>,
        fee_strategy: AddressFundsFeeStrategy,
        output_script: CoreScript,
        input_witnesses_count: usize,
    ) -> StateTransition {
        let witnesses: Vec<AddressWitness> = (0..input_witnesses_count)
            .map(|_| create_dummy_witness())
            .collect();
        AddressCreditWithdrawalTransition::V0(AddressCreditWithdrawalTransitionV0 {
            inputs,
            output,
            fee_strategy,
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script,
            user_fee_increase: 0,
            input_witnesses: witnesses,
        })
        .into()
    }

    /// Create a signed AddressCreditWithdrawalTransition
    fn create_signed_address_credit_withdrawal_transition(
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        output: Option<(PlatformAddress, u64)>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        output_script: CoreScript,
    ) -> StateTransition {
        create_signed_address_credit_withdrawal_transition_with_fee_increase(
            signer,
            inputs,
            output,
            fee_strategy,
            output_script,
            0,
        )
    }

    fn create_signed_address_credit_withdrawal_transition_with_fee_increase(
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        output: Option<(PlatformAddress, u64)>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        output_script: CoreScript,
        user_fee_increase: u16,
    ) -> StateTransition {
        AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
            inputs,
            output,
            AddressFundsFeeStrategy::from(fee_strategy),
            1, // core_fee_per_byte
            Pooling::Never,
            output_script,
            signer,
            user_fee_increase,
            PlatformVersion::latest(),
        )
        .expect("should create signed transition")
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS
    // These test basic structure validation (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_no_inputs_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            // No inputs case - should fail validation
            let inputs = BTreeMap::new();

            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
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
                    ConsensusError::BasicError(BasicError::TransitionNoInputsError(_))
                )]
            );
        }

        #[test]
        fn test_too_many_inputs_returns_error() {
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();
            let max_inputs = platform_version.dpp.state_transitions.max_address_inputs;

            let mut rng = StdRng::seed_from_u64(567);

            // Create max_inputs + 1 inputs (17 inputs, max is 16)
            let input_count = max_inputs as usize + 1;
            let mut inputs = BTreeMap::new();
            for i in 0..input_count {
                inputs.insert(
                    create_platform_address(i as u8),
                    (1 as AddressNonce, dash_to_credits!(0.1)),
                );
            }

            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
                input_count,
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
        fn test_input_witness_count_mismatch_returns_error() {
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(0.1)),
            );
            inputs.insert(
                create_platform_address(2),
                (1 as AddressNonce, dash_to_credits!(0.1)),
            );

            // Create transition with 2 inputs but only 1 witness
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
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
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let same_address = create_platform_address(1);

            let mut inputs = BTreeMap::new();
            inputs.insert(same_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            // Output to same address as input
            let output = Some((same_address, dash_to_credits!(0.5)));

            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                output,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
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
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Empty fee strategy
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::FeeStrategyEmptyError(_))
                ),
                "Expected FeeStrategyEmptyError, got {:?}",
                error
            );
        }

        #[test]
        fn test_fee_strategy_index_out_of_bounds_for_input_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // DeductFromInput(5) but only 1 input exists
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(5),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
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
        fn test_fee_strategy_index_out_of_bounds_for_output_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // ReduceOutput(0) but no output exists
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None, // No output
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
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
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(create_platform_address(1), (1 as AddressNonce, 100)); // Very small input

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
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

        #[test]
        fn test_output_below_minimum_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Very small output amount
            let output = Some((create_platform_address(2), 100));

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::OutputBelowMinimumError(_))
                ),
                "Expected OutputBelowMinimumError, got {:?}",
                error
            );
        }

        #[test]
        fn test_fee_strategy_duplicate_steps_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Duplicate fee strategy steps
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0), // Duplicate
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::FeeStrategyDuplicateError(_))
                ),
                "Expected FeeStrategyDuplicateError, got {:?}",
                error
            );
        }
    }

    // ==========================================
    // SUCCESSFUL TRANSITION TESTS
    // ==========================================

    mod successful_transitions {
        use super::*;

        #[test]
        fn test_simple_withdrawal_from_single_address() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.9)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None, // No change output
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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
        fn test_withdrawal_with_change_output() {
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
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.5)));

            // Change output to a different address
            let output = Some((create_platform_address(2), dash_to_credits!(0.3)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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
        fn test_withdrawal_from_multiple_inputs() {
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
            let input_address1 = signer.add_p2pkh([1u8; 32]);
            let input_address2 = signer.add_p2pkh([2u8; 32]);
            let input_address3 = signer.add_p2pkh([3u8; 32]);
            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address3, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address3, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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
        fn test_withdrawal_with_fee_deducted_from_output() {
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
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.5)));

            // Deduct fee from output instead of input
            let output = Some((create_platform_address(2), dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                create_random_output_script(&mut rng),
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
        fn test_withdrawal_with_user_fee_increase() {
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
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            // Use fee increase
            let transition = create_signed_address_credit_withdrawal_transition_with_fee_increase(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                100, // 1% fee increase
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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

            let mut inputs = BTreeMap::new();
            // Try to spend 0.8 DASH when only 0.5 available
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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

            let mut inputs = BTreeMap::new();
            // Use wrong nonce (5 instead of expected 1)
            inputs.insert(input_address, (5 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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

            let mut inputs = BTreeMap::new();
            inputs.insert(real_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Sign with wrong signer
            let transition = create_signed_address_credit_withdrawal_transition(
                &wrong_signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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
        fn test_withdrawal_with_p2sh_multisig_input() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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
        fn test_withdrawal_with_mixed_p2pkh_and_p2sh_inputs() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(p2pkh_address, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
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

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            // Create transition manually with only 1 signature instead of required 2
            let mut transition = AddressCreditWithdrawalTransitionV0 {
                inputs: inputs.clone(),
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
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
    // CONCURRENT INPUT USAGE TESTS
    // ==========================================

    mod concurrent_input_usage {
        use super::*;

        #[test]
        fn test_two_transitions_same_input_address_sequential_nonces() {
            // Test that two withdrawals from the same address succeed with sequential nonces
            // when processed in separate blocks
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
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(5.0));

            let mut rng = StdRng::seed_from_u64(567);

            // First transition: spend 0.5 DASH with nonce 1
            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition1 = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs1,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result1 = transition1.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process first transition
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

            // First should succeed
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        #[test]
        fn test_second_transition_exceeds_remaining_balance() {
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
            // Only 1 DASH available
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            // First transition: spend 0.6 DASH
            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.6)));

            let transition1 = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs1,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            // Second transition: try to spend 0.6 DASH again (but only ~0.4 remains after first)
            let mut inputs2 = BTreeMap::new();
            inputs2.insert(input_address, (2 as AddressNonce, dash_to_credits!(0.6)));

            let transition2 = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs2,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result1 = transition1.serialize_to_bytes().expect("should serialize");
            let result2 = transition2.serialize_to_bytes().expect("should serialize");

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

            // First should succeed, second should fail due to insufficient balance
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [
                    StateTransitionExecutionResult::SuccessfulExecution(_, _),
                    StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::StateError(StateError::AddressNotEnoughFundsError(_))
                    )
                ]
            );
        }
    }
}
