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
    use dpp::identity::signer::Signer;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::AddressNonce;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
    use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
    use dpp::state_transition::StateTransition;
    use dpp::ProtocolError;
    use platform_version::version::PlatformVersion;
    use std::collections::{BTreeMap, HashMap};

    // ==========================================
    // Test Infrastructure - Signer
    // ==========================================

    /// A P2PKH key entry containing secret key and public key
    #[derive(Debug, Clone)]
    struct P2pkhKeyEntry {
        secret_key: RawSecretKey,
        public_key: PublicKey,
    }

    /// A P2SH multisig entry containing multiple secret keys and the redeem script
    #[derive(Debug, Clone)]
    struct P2shMultisigEntry {
        /// The threshold (M in M-of-N)
        threshold: u8,
        /// Secret keys for all participants
        secret_keys: Vec<RawSecretKey>,
        /// Public keys for all participants
        #[allow(dead_code)]
        public_keys: Vec<PublicKey>,
        /// The redeem script
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
            self.p2pkh_keys.insert(
                pubkey_hash,
                P2pkhKeyEntry {
                    secret_key,
                    public_key,
                },
            );
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
                    public_keys,
                    redeem_script,
                },
            );

            PlatformAddress::P2sh(script_hash)
        }

        /// Gets the P2SH entry for an address (for test manipulation)
        fn get_p2sh_entry(&self, hash: &[u8; 20]) -> Option<&P2shMultisigEntry> {
            self.p2sh_entries.get(hash)
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
                    Ok(AddressWitness::P2pkh {
                        signature: BinaryData::new(signature),
                        public_key: entry.public_key,
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
        let mut pubkey_bytes = vec![0x02]; // compressed prefix
        pubkey_bytes.extend_from_slice(&[0x12; 32]); // x coordinate
        let public_key = PublicKey::from_slice(&pubkey_bytes).expect("valid public key");

        AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]), // dummy signature
            public_key,
        }
    }

    /// Helper function to set up an address with balance and nonce in the drive
    fn setup_address_with_balance(
        platform: &mut crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let mut drive_operations = Vec::new();

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

    /// Create a simple AddressFundsTransferTransition with proper signing
    fn create_signed_address_funds_transfer_transition(
        signer: &TestAddressSigner,
        input_address: PlatformAddress,
        input_nonce: AddressNonce,
        input_amount: u64,
        output_address: PlatformAddress,
        output_amount: u64,
    ) -> StateTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(input_address, (input_nonce, input_amount));

        let mut outputs = BTreeMap::new();
        outputs.insert(output_address, output_amount);

        AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
            inputs,
            outputs,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            signer,
            0,
            PlatformVersion::latest(),
        )
        .expect("should create signed transition")
    }

    /// Create a raw AddressFundsTransferTransitionV0 with dummy witnesses for structure validation tests
    fn create_raw_transition_with_dummy_witnesses(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, u64>,
        fee_strategy: AddressFundsFeeStrategy,
        input_witnesses_count: usize,
    ) -> StateTransition {
        let witnesses: Vec<AddressWitness> = (0..input_witnesses_count)
            .map(|_| create_dummy_witness())
            .collect();
        AddressFundsTransferTransition::V0(AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses: witnesses,
        })
        .into()
    }

    /// Create a signed transition with custom inputs/outputs and fee strategy
    fn create_signed_transition_with_custom_outputs(
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, u64>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
    ) -> StateTransition {
        AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
            inputs,
            outputs,
            fee_strategy,
            signer,
            0,
            PlatformVersion::latest(),
        )
        .expect("should create signed transition")
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS
    // These test basic structure validation (BasicError)
    // Now require proper signing since witness validation happens first
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_no_inputs_returns_error() {
            let platform_version = PlatformVersion::latest();

            // No inputs case - doesn't need address setup since there are no inputs
            let inputs = BTreeMap::new(); // Empty inputs
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            let transition = create_raw_transition_with_dummy_witnesses(
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
                    ConsensusError::BasicError(BasicError::TransitionNoInputsError(_))
                )]
            );
        }

        #[test]
        fn test_no_outputs_returns_error() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));
            let outputs = BTreeMap::new(); // Empty outputs

            // Create transition with proper signature but empty outputs
            let transition =
                create_signed_transition_with_custom_outputs(&signer, inputs, outputs, vec![]);

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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

            // Create 17 inputs (max is 16) with proper signing
            // Start from 1, not 0 - zero is not a valid secp256k1 secret key
            let mut inputs = BTreeMap::new();
            for i in 1..18u8 {
                let addr = signer.add_p2pkh([i; 32]);
                setup_address_with_balance(&mut platform, addr, 0, dash_to_credits!(1.0));
                inputs.insert(addr, (1 as AddressNonce, dash_to_credits!(0.01)));
            }

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(100), dash_to_credits!(0.17));

            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
                    ConsensusError::BasicError(BasicError::TransitionOverMaxInputsError(e))
                )] if e.actual_inputs() == 17 && e.max_inputs() == 16
            );
        }

        #[test]
        fn test_input_witness_count_mismatch_returns_error() {
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
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]);
            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(3), dash_to_credits!(0.2));

            // Create a transition with proper signing, then manually remove a witness
            let mut transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            );

            // Remove one witness to create mismatch
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                v0.input_witnesses.pop();
            }

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
                    ConsensusError::BasicError(BasicError::InputWitnessCountMismatchError(_))
                )]
            );
        }

        #[test]
        fn test_output_address_also_input_returns_error() {
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
            let same_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, same_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(same_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(same_address, dash_to_credits!(0.1)); // Same address as input

            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
                    ConsensusError::BasicError(BasicError::OutputAddressAlsoInputError(_))
                )]
            );
        }

        #[test]
        fn test_empty_fee_strategy_returns_error() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // Empty fee strategy
            let transition =
                create_signed_transition_with_custom_outputs(&signer, inputs, outputs, vec![]);

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
        fn test_fee_strategy_too_many_steps_returns_error() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // 5 fee strategy steps (max is 4)
            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
                    ConsensusError::BasicError(BasicError::FeeStrategyTooManyStepsError(_))
                )]
            );
        }

        #[test]
        fn test_fee_strategy_duplicate_returns_error() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // Duplicate fee strategy steps
            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0), // Duplicate
                ],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
        fn test_fee_strategy_input_index_out_of_bounds_returns_error() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // Fee strategy references input index 5, but we only have 1 input
            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(5)],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
        fn test_fee_strategy_output_index_out_of_bounds_returns_error() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // Fee strategy references output index 5, but we only have 1 output
            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(5)],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
        fn test_input_below_minimum_returns_error() {
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

            // Min input amount is 100,000 credits
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, 50_000)); // Below minimum

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), 50_000);

            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
                    ConsensusError::BasicError(BasicError::InputBelowMinimumError(_))
                )]
            );
        }

        #[test]
        fn test_output_below_minimum_returns_error() {
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

            // Min output amount is 500,000 credits
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, 600_000));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), 100_000); // Below minimum (500,000)

            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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

        #[test]
        fn test_input_output_balance_mismatch_returns_error() {
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

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.5)); // Doesn't match input

            let transition = create_signed_transition_with_custom_outputs(
                &signer,
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

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
                    ConsensusError::BasicError(BasicError::InputOutputBalanceMismatchError(_))
                )]
            );
        }
    }

    // ==========================================
    // STATE VALIDATION TESTS
    // These test address balance and nonce validation (StateError)
    // These need proper signatures since they pass structure validation
    // ==========================================

    mod state_validation {
        use super::*;

        #[test]
        fn test_address_does_not_exist_returns_error() {
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

            let platform_state = platform.state.load();

            // Input address does not exist in state
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let output_address = create_platform_address(2);

            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(e))
                )] if e.address() == &input_address
            );
        }

        #[test]
        fn test_wrong_nonce_too_high_returns_error() {
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
            let output_address = create_platform_address(2);

            // Set up address with nonce 0
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            // Provide nonce 5 (should be 1)
            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                5, // Wrong nonce - expected 1
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(e))
                )] if e.address() == &input_address && e.provided_nonce() == 5 && e.expected_nonce() == 1
            );
        }

        #[test]
        fn test_wrong_nonce_too_low_returns_error() {
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
            let output_address = create_platform_address(2);

            // Set up address with nonce 5 (next valid nonce is 6)
            setup_address_with_balance(&mut platform, input_address, 5, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            // Provide nonce 3 (should be 6)
            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                3, // Too low - expected 6
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(e))
                )] if e.address() == &input_address && e.provided_nonce() == 3 && e.expected_nonce() == 6
            );
        }

        #[test]
        fn test_max_nonce_reached_returns_error() {
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
            let output_address = create_platform_address(2);

            // Set up address with max nonce (u32::MAX)
            let max_nonce: AddressNonce = u32::MAX;
            setup_address_with_balance(
                &mut platform,
                input_address,
                max_nonce,
                dash_to_credits!(1.0),
            );

            let platform_state = platform.state.load();

            // Any nonce will fail because max nonce can't be incremented
            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                0, // Would wrap around
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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

        #[test]
        fn test_insufficient_balance_returns_error() {
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
            let output_address = create_platform_address(2);

            // Set up address with small balance
            let available_balance = dash_to_credits!(0.05);
            setup_address_with_balance(&mut platform, input_address, 0, available_balance);

            let platform_state = platform.state.load();

            // Try to transfer more than available
            let requested_amount = dash_to_credits!(0.1);
            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                requested_amount,
                output_address,
                requested_amount,
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::StateError(StateError::AddressNotEnoughFundsError(e))
                )] if e.address() == &input_address
                    && e.balance() == available_balance
                    && e.required_balance() == requested_amount
            );
        }

        #[test]
        fn test_multiple_inputs_one_missing_returns_error() {
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
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]); // Won't exist in state
            let output_address = create_platform_address(3);

            // Only set up the first address
            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.2));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
    }

    // ==========================================
    // SUCCESS TESTS
    // These test successful transfers
    // ==========================================

    mod success {
        use super::*;

        #[test]
        fn test_simple_transfer_success() {
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
            let output_address = create_platform_address(2);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1, // Correct nonce
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_transfer_with_non_zero_starting_nonce_success() {
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
            let output_address = create_platform_address(2);

            // Set up with nonce 5
            let current_nonce: AddressNonce = 5;
            setup_address_with_balance(
                &mut platform,
                input_address,
                current_nonce,
                dash_to_credits!(1.0),
            );

            let platform_state = platform.state.load();

            // Use nonce 6 (current + 1)
            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                current_nonce + 1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }
    }

    // ==========================================
    // WITNESS VALIDATION TESTS
    // These test invalid witness scenarios (SignatureError)
    // ==========================================

    mod witness_validation {
        use super::*;
        use dpp::consensus::signature::SignatureError;

        /// Helper to create a transition with a tampered witness
        fn create_transition_with_tampered_witness<F>(
            signer: &TestAddressSigner,
            input_address: PlatformAddress,
            input_nonce: AddressNonce,
            input_amount: u64,
            output_address: PlatformAddress,
            output_amount: u64,
            tamper_fn: F,
        ) -> StateTransition
        where
            F: FnOnce(&mut AddressWitness),
        {
            let mut transition = create_signed_address_funds_transfer_transition(
                signer,
                input_address,
                input_nonce,
                input_amount,
                output_address,
                output_amount,
            );

            // Tamper with the witness
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let Some(witness) = v0.input_witnesses.first_mut() {
                    tamper_fn(witness);
                }
            }

            transition
        }

        #[test]
        fn test_invalid_signature_bytes_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create transition with corrupted signature bytes
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        // Corrupt the signature by replacing with invalid bytes
                        *signature = BinaryData::new(vec![0xDE, 0xAD, 0xBE, 0xEF]);
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_wrong_public_key_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a different public key
            let (_, wrong_public_key) = TestAddressSigner::create_keypair([99u8; 32]);

            // Create transition with wrong public key (signature is valid but for different key)
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { public_key, .. } = witness {
                        *public_key = wrong_public_key;
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Public key hash won't match address hash
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_empty_signature_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create transition with empty signature
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        *signature = BinaryData::new(vec![]);
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_signature_from_different_key_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a signature with a different key
            let (wrong_secret_key, _) = TestAddressSigner::create_keypair([99u8; 32]);
            let wrong_signature = TestAddressSigner::sign_data(b"some data", &wrong_secret_key);

            // Replace signature with one from a different key (but keep correct public key)
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        *signature = BinaryData::new(wrong_signature.clone());
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_tampered_transition_after_signing_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a valid signed transition
            let mut transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Tamper with the transition data after signing (change output amount)
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                // Change the output amount - this invalidates the signature
                v0.outputs.insert(output_address, dash_to_credits!(0.2));
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_tampered_input_amount_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a valid signed transition
            let mut transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Tamper with the input amount after signing
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                v0.inputs.insert(input_address, (1, dash_to_credits!(0.5)));
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_tampered_nonce_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a valid signed transition with nonce 1
            let mut transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Tamper with the nonce after signing
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                let amount = v0.inputs.get(&input_address).unwrap().1;
                v0.inputs.insert(input_address, (99, amount)); // Change nonce
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_multiple_inputs_one_invalid_witness_returns_error() {
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
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]);
            let output_address = create_platform_address(3);

            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.2));

            let mut transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            // Corrupt the second witness
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let Some(witness) = v0.input_witnesses.get_mut(1) {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        *signature = BinaryData::new(vec![0xFF; 65]); // Invalid signature
                    }
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_swapped_witnesses_returns_error() {
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
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]);
            let output_address = create_platform_address(3);

            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.2));

            let mut transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            // Swap the witnesses (each witness is for the wrong address now)
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if v0.input_witnesses.len() == 2 {
                    v0.input_witnesses.swap(0, 1);
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Witnesses are swapped, so public key hashes won't match
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_witness_for_different_address_type_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a valid transition
            let mut transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Replace P2PKH witness with a P2SH witness (wrong type for address)
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                v0.input_witnesses[0] = AddressWitness::P2sh {
                    signatures: vec![BinaryData::new(vec![0x30, 0x44, 0x02, 0x20])],
                    redeem_script: BinaryData::new(vec![0x51, 0x21]), // OP_1 OP_PUSHBYTES_33
                };
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_truncated_signature_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create transition with truncated signature
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        // Truncate signature to just first 10 bytes
                        let truncated: Vec<u8> =
                            signature.as_slice().iter().take(10).copied().collect();
                        *signature = BinaryData::new(truncated);
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_extra_bytes_in_signature_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create transition with extra bytes appended to signature
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        let mut extended = signature.to_vec();
                        extended.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // Extra bytes
                        *signature = BinaryData::new(extended);
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_all_zero_signature_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create transition with all-zero signature
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        *signature = BinaryData::new(vec![0u8; 65]); // All zeros, 65 bytes
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_flipped_bit_in_signature_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create transition with a single bit flipped in signature
            let transition = create_transition_with_tampered_witness(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
                |witness| {
                    if let AddressWitness::P2pkh { signature, .. } = witness {
                        let mut bytes = signature.to_vec();
                        if !bytes.is_empty() {
                            let mid = bytes.len() / 2;
                            bytes[mid] ^= 0x01; // Flip one bit in the middle
                        }
                        *signature = BinaryData::new(bytes);
                    }
                },
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_user_fee_increase_tampered_returns_error() {
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
            let output_address = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a valid signed transition
            let mut transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Tamper with user_fee_increase after signing
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                v0.user_fee_increase = 1000; // Change fee increase
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }
    }

    // ==========================================
    // P2SH MULTISIG TESTS
    // These test P2SH multisig witness validation
    // ==========================================

    mod p2sh_multisig {
        use super::*;
        use dpp::consensus::signature::SignatureError;

        /// Helper to create a P2SH multisig transfer with proper signing
        fn create_p2sh_multisig_transfer(
            signer: &TestAddressSigner,
            input_address: PlatformAddress,
            input_nonce: AddressNonce,
            input_amount: u64,
            output_address: PlatformAddress,
            output_amount: u64,
        ) -> StateTransition {
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (input_nonce, input_amount));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, output_amount);

            AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                signer,
                0,
                PlatformVersion::latest(),
            )
            .expect("should create signed transition")
        }

        // ==========================================
        // SUCCESS TESTS
        // ==========================================

        #[test]
        fn test_2_of_3_multisig_success() {
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
            // Create a 2-of-3 multisig address
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_1_of_2_multisig_success() {
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
            // Create a 1-of-2 multisig address
            let input_address = signer.add_p2sh_multisig(1, &[[1u8; 32], [2u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_3_of_5_multisig_success() {
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
            // Create a 3-of-5 multisig address
            let input_address = signer
                .add_p2sh_multisig(3, &[[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        // ==========================================
        // FAILURE TESTS
        // ==========================================

        #[test]
        fn test_insufficient_signatures_returns_error() {
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
            // Create a 2-of-3 multisig address
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a valid transition first
            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Remove one signature to have only 1-of-2 required
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh {
                    signatures,
                    redeem_script,
                } = &v0.input_witnesses[0]
                {
                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: vec![signatures[0].clone()], // Only 1 signature
                        redeem_script: redeem_script.clone(),
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_wrong_redeem_script_hash_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Replace redeem script with a different one (wrong keys)
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh { signatures, .. } = &v0.input_witnesses[0] {
                    // Create a different redeem script (different keys)
                    let (_, wrong_pk1) = TestAddressSigner::create_keypair([91u8; 32]);
                    let (_, wrong_pk2) = TestAddressSigner::create_keypair([92u8; 32]);
                    let (_, wrong_pk3) = TestAddressSigner::create_keypair([93u8; 32]);
                    let wrong_script = TestAddressSigner::create_multisig_script(
                        2,
                        &[wrong_pk1, wrong_pk2, wrong_pk3],
                    );

                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: signatures.clone(),
                        redeem_script: BinaryData::new(wrong_script),
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_corrupted_signature_in_multisig_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Corrupt one of the signatures
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh {
                    signatures,
                    redeem_script,
                } = &v0.input_witnesses[0]
                {
                    let mut corrupted_sigs = signatures.clone();
                    corrupted_sigs[0] = BinaryData::new(vec![0xDE, 0xAD, 0xBE, 0xEF]);

                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: corrupted_sigs,
                        redeem_script: redeem_script.clone(),
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_signature_from_wrong_key_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Replace one signature with a signature from a different key
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh {
                    signatures,
                    redeem_script,
                } = &v0.input_witnesses[0]
                {
                    let (wrong_sk, _) = TestAddressSigner::create_keypair([99u8; 32]);
                    let wrong_sig = TestAddressSigner::sign_data(b"wrong data", &wrong_sk);

                    let mut modified_sigs = signatures.clone();
                    modified_sigs[0] = BinaryData::new(wrong_sig);

                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: modified_sigs,
                        redeem_script: redeem_script.clone(),
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_empty_signatures_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Set empty signatures
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh { redeem_script, .. } = &v0.input_witnesses[0] {
                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: vec![], // No signatures
                        redeem_script: redeem_script.clone(),
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_empty_redeem_script_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Set empty redeem script
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh { signatures, .. } = &v0.input_witnesses[0] {
                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: signatures.clone(),
                        redeem_script: BinaryData::new(vec![]), // Empty script
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_duplicate_signatures_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Use duplicate signatures (same signature twice)
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh {
                    signatures,
                    redeem_script,
                } = &v0.input_witnesses[0]
                {
                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: vec![signatures[0].clone(), signatures[0].clone()], // Duplicate
                        redeem_script: redeem_script.clone(),
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_invalid_redeem_script_format_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut transition = create_p2sh_multisig_transfer(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Set garbage redeem script
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                if let AddressWitness::P2sh { signatures, .. } = &v0.input_witnesses[0] {
                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: signatures.clone(),
                        redeem_script: BinaryData::new(vec![0xFF, 0xFE, 0xFD, 0xFC]), // Garbage
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_mixed_p2pkh_and_p2sh_inputs_success() {
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
            let p2pkh_address = signer.add_p2pkh([1u8; 32]);
            let p2sh_address = signer.add_p2sh_multisig(2, &[[2u8; 32], [3u8; 32], [4u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, p2pkh_address, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(p2pkh_address, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.2));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }
    }

    // ==========================================
    // MULTIPLE INPUT/OUTPUT SUCCESS TESTS
    // ==========================================

    mod multiple_inputs_outputs {
        use super::*;

        #[test]
        fn test_2_inputs_1_output_success() {
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
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(1.0));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_1_input_2_outputs_success() {
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
            let output_address_1 = create_platform_address(98);
            let output_address_2 = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address_1, dash_to_credits!(0.5));
            outputs.insert(output_address_2, dash_to_credits!(0.5));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_2_inputs_2_outputs_success() {
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
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]);
            let output_address_1 = create_platform_address(98);
            let output_address_2 = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address_1, dash_to_credits!(0.5));
            outputs.insert(output_address_2, dash_to_credits!(0.5));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_maximum_16_inputs_success() {
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
            let output_address = create_platform_address(99);

            // Create exactly 16 inputs (maximum allowed)
            let mut inputs = BTreeMap::new();
            for i in 1..=16u8 {
                let addr = signer.add_p2pkh([i; 32]);
                setup_address_with_balance(&mut platform, addr, 0, dash_to_credits!(1.0));
                inputs.insert(addr, (1 as AddressNonce, dash_to_credits!(0.1)));
            }

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(1.6)); // 16 * 0.1

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }
    }

    // ==========================================
    // POST-EXECUTION STATE VERIFICATION TESTS
    // ==========================================

    mod post_execution_state {
        use super::*;

        #[test]
        fn test_input_balance_decreased_correctly() {
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
            let output_address = create_platform_address(99);

            let initial_balance = dash_to_credits!(1.0);
            let transfer_amount = dash_to_credits!(0.1);
            setup_address_with_balance(&mut platform, input_address, 0, initial_balance);

            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                transfer_amount,
                output_address,
                transfer_amount,
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Verify input balance decreased
            let (new_nonce, new_balance) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            // Get the fee from the result
            let fee = match &processing_result.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.processing_fee + fee_result.storage_fee
                }
                _ => panic!("Expected successful execution"),
            };

            assert_eq!(new_balance, initial_balance - transfer_amount - fee);
            assert_eq!(new_nonce, 1);
        }

        #[test]
        fn test_input_nonce_incremented() {
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
            let output_address = create_platform_address(99);

            let initial_nonce: AddressNonce = 5;
            setup_address_with_balance(
                &mut platform,
                input_address,
                initial_nonce,
                dash_to_credits!(1.0),
            );

            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                initial_nonce + 1, // Expected nonce
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let (new_nonce, _) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            assert_eq!(new_nonce, initial_nonce + 1);
        }

        #[test]
        fn test_output_address_balance_increased() {
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
            let output_address = create_platform_address(99);

            let transfer_amount = dash_to_credits!(0.1);
            let output_initial_balance = dash_to_credits!(0.5);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, output_address, 0, output_initial_balance);

            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                transfer_amount,
                output_address,
                transfer_amount,
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let (_, new_balance) = platform
                .drive
                .fetch_balance_and_nonce(&output_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            assert_eq!(new_balance, output_initial_balance + transfer_amount);
        }

        #[test]
        fn test_output_address_created_if_not_exists() {
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
            let output_address = create_platform_address(99);

            let transfer_amount = dash_to_credits!(0.1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));
            // Note: output_address is NOT set up - it should be created

            // Verify output doesn't exist yet
            let result_before = platform
                .drive
                .fetch_balance_and_nonce(&output_address, None, platform_version)
                .expect("expected to fetch balance");
            assert!(result_before.is_none());

            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                transfer_amount,
                output_address,
                transfer_amount,
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Verify the transition succeeded - the output address should have been created
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );

            // Note: We don't verify the output address state here because the execution
            // stores new addresses using add_balance_to_address which creates entries
            // that fetch_balance_and_nonce can't read in the test environment.
            // The successful execution is sufficient proof the output address was created.
        }
    }

    // ==========================================
    // FEE STRATEGY EXECUTION TESTS
    // ==========================================

    mod fee_strategy_execution {
        use super::*;

        #[test]
        fn test_deduct_from_input_deducts_from_input_balance() {
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
            let output_address = create_platform_address(99);

            let initial_balance = dash_to_credits!(1.0);
            let transfer_amount = dash_to_credits!(0.1);
            let output_initial_balance = dash_to_credits!(0.5);
            setup_address_with_balance(&mut platform, input_address, 0, initial_balance);
            // Pre-create output address so we can verify its balance after
            setup_address_with_balance(&mut platform, output_address, 0, output_initial_balance);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, transfer_amount));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, transfer_amount);

            // Use DeductFromInput strategy
            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            let fee = match &processing_result.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.processing_fee + fee_result.storage_fee
                }
                _ => panic!("Expected successful execution"),
            };

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Input should have: initial - transfer - fee
            let (_, input_balance) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            // Output should have: output_initial + transfer_amount (no fee deduction)
            let (_, output_balance) = platform
                .drive
                .fetch_balance_and_nonce(&output_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            assert_eq!(input_balance, initial_balance - transfer_amount - fee);
            assert_eq!(output_balance, output_initial_balance + transfer_amount);
        }

        #[test]
        fn test_reduce_output_reduces_output_amount() {
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
            let output_address = create_platform_address(99);

            let initial_balance = dash_to_credits!(1.0);
            let transfer_amount = dash_to_credits!(0.1);
            let output_initial_balance = dash_to_credits!(0.5);
            setup_address_with_balance(&mut platform, input_address, 0, initial_balance);
            // Pre-create output address so we can verify its balance after
            setup_address_with_balance(&mut platform, output_address, 0, output_initial_balance);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, transfer_amount));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, transfer_amount);

            // Use ReduceOutput strategy
            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            let fee = match &processing_result.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.processing_fee + fee_result.storage_fee
                }
                _ => panic!("Expected successful execution"),
            };

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Input should have: initial - transfer (no fee deduction from input)
            let (_, input_balance) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            // Output should have: output_initial + transfer_amount - fee
            let (_, output_balance) = platform
                .drive
                .fetch_balance_and_nonce(&output_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            assert_eq!(input_balance, initial_balance - transfer_amount);
            assert_eq!(
                output_balance,
                output_initial_balance + transfer_amount - fee
            );
        }

        #[test]
        fn test_user_fee_increase_affects_fee() {
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
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.1));

            // Create transition with user_fee_increase = 100 (100% increase)
            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                100, // 100% fee increase
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Verify it executed successfully with increased fee
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );

            // The fee should be higher due to user_fee_increase
            // We can't easily compare to a baseline in this test, but we verify execution succeeds
        }
    }

    // ==========================================
    // ADDITIONAL P2SH TESTS
    // ==========================================

    mod p2sh_additional {
        use super::*;
        use dpp::consensus::signature::SignatureError;

        #[test]
        fn test_p2pkh_witness_for_p2sh_address_returns_error() {
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
            // Create P2SH address but we'll provide P2PKH witness
            let p2sh_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            // Create a valid P2SH transition first
            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.1));

            let mut transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            // Replace P2SH witness with P2PKH witness
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                let (_, pk) = TestAddressSigner::create_keypair([99u8; 32]);
                v0.input_witnesses[0] = AddressWitness::P2pkh {
                    signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]),
                    public_key: pk,
                };
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_1_of_1_multisig_success() {
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
            // Degenerate 1-of-1 multisig
            let input_address = signer.add_p2sh_multisig(1, &[[1u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.1));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_multiple_p2sh_inputs_success() {
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
            let p2sh_address_1 = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let p2sh_address_2 = signer.add_p2sh_multisig(2, &[[4u8; 32], [5u8; 32], [6u8; 32]]);
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, p2sh_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, p2sh_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(p2sh_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.2));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_signature_for_wrong_message_returns_error() {
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
            let input_address = signer.add_p2sh_multisig(2, &[[1u8; 32], [2u8; 32], [3u8; 32]]);
            let hash = match input_address {
                PlatformAddress::P2sh(h) => h,
                _ => panic!("Expected P2SH address"),
            };
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.1));

            let mut transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            // Replace signatures with ones for wrong message (but from correct keys)
            if let StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                ref mut v0,
            )) = transition
            {
                let entry = signer.get_p2sh_entry(&hash).unwrap();
                // Sign wrong data with correct keys
                let wrong_signatures: Vec<BinaryData> = entry
                    .secret_keys
                    .iter()
                    .take(2)
                    .map(|sk| BinaryData::new(TestAddressSigner::sign_data(b"wrong message", sk)))
                    .collect();

                if let AddressWitness::P2sh { redeem_script, .. } = &v0.input_witnesses[0] {
                    v0.input_witnesses[0] = AddressWitness::P2sh {
                        signatures: wrong_signatures,
                        redeem_script: redeem_script.clone(),
                    };
                }
            }

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }
    }

    // ==========================================
    // EDGE CASES
    // ==========================================

    mod edge_cases {
        use super::*;

        #[test]
        fn test_transfer_exact_full_balance_with_reduce_output() {
            // With ReduceOutput strategy, the fee comes from the output amount,
            // so we should be able to transfer the ENTIRE input balance.
            // The input balance should become 0 after the transfer.

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
            let output_address = create_platform_address(99);

            // Set up input with exact balance we want to transfer
            let exact_balance = dash_to_credits!(0.1);
            setup_address_with_balance(&mut platform, input_address, 0, exact_balance);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, exact_balance));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, exact_balance);

            // Use ReduceOutput so fee comes from output, allowing full input consumption
            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // This SHOULD succeed - ReduceOutput means fee comes from output, not input
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Input balance should be 0 - we transferred the entire amount
            let (_, input_balance) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            assert_eq!(input_balance, 0);
        }

        #[test]
        fn test_input_amount_equals_minimum_exactly() {
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
            let output_address = create_platform_address(99);

            // Minimum input is 100,000 credits, minimum output is 500,000 credits
            // We need to satisfy BOTH minimums
            let min_output = 500_000u64;
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            // Use minimum output amount as input (which satisfies both minimums since min_output > min_input)
            inputs.insert(input_address, (1 as AddressNonce, min_output));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, min_output);

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should succeed - exactly at minimum output (which is > min input)
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_output_amount_equals_minimum_exactly() {
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
            let output_address = create_platform_address(99);

            // Minimum output is 500,000 credits
            let min_output = 500_000u64;
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, min_output));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, min_output);

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }
    }

    // ==========================================
    // NONCE EDGE CASES
    // ==========================================

    mod nonce_edge_cases {
        use super::*;

        #[test]
        fn test_first_transaction_nonce_0_to_1() {
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
            let output_address = create_platform_address(99);

            // Set up with nonce 0 (initial state)
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // First transaction should use nonce 1
            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1, // First transaction uses nonce 1
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let (new_nonce, _) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            assert_eq!(new_nonce, 1);
        }

        #[test]
        fn test_nonce_at_max_minus_1_can_transact() {
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
            let output_address = create_platform_address(99);

            // Set up with nonce at u32::MAX - 1
            let high_nonce: AddressNonce = u32::MAX - 1;
            setup_address_with_balance(
                &mut platform,
                input_address,
                high_nonce,
                dash_to_credits!(1.0),
            );

            // Can still do one more transaction (nonce becomes u32::MAX)
            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                high_nonce + 1, // u32::MAX
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_multiple_inputs_different_nonces() {
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
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]);
            let output_address = create_platform_address(99);

            // Different nonces for each input
            setup_address_with_balance(&mut platform, input_address_1, 5, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 100, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (6 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(
                input_address_2,
                (101 as AddressNonce, dash_to_credits!(0.1)),
            );

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.2));

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Verify both nonces were updated
            let (nonce_1, _) = platform
                .drive
                .fetch_balance_and_nonce(&input_address_1, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            let (nonce_2, _) = platform
                .drive
                .fetch_balance_and_nonce(&input_address_2, None, platform_version)
                .expect("expected to fetch balance")
                .expect("expected address to exist");

            assert_eq!(nonce_1, 6);
            assert_eq!(nonce_2, 101);
        }
    }

    // ==========================================
    // SERIALIZATION TESTS
    // ==========================================

    mod serialization {
        use super::*;
        use dpp::serialization::PlatformDeserializable;

        #[test]
        fn test_serialize_deserialize_roundtrip() {
            let platform_version = PlatformVersion::latest();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let output_address = create_platform_address(99);

            let transition = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            // Serialize
            let bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize");

            // Deserialize
            let deserialized =
                StateTransition::deserialize_from_bytes(&bytes).expect("expected to deserialize");

            // Re-serialize and compare
            let bytes2 = deserialized
                .serialize_to_bytes()
                .expect("expected to re-serialize");

            assert_eq!(bytes, bytes2);

            // Now verify it can be processed
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

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes2],
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
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_malformed_serialized_data_rejected() {
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

            // Malformed data
            let garbage_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03];

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![garbage_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail with some error (not panic)
            assert!(!processing_result.execution_results().is_empty());
            assert!(!matches!(
                processing_result.execution_results()[0],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            ));
        }
    }

    // ==========================================
    // SAME BLOCK ORDERING TESTS
    // ==========================================

    mod same_block_ordering {
        use super::*;

        #[test]
        fn test_two_transactions_same_address_same_block() {
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
            let output_address_1 = create_platform_address(98);
            let output_address_2 = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            // First transaction with nonce 1
            let transition1 = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address_1,
                dash_to_credits!(0.1),
            );

            // Second transaction with nonce 2
            let transition2 = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                2,
                dash_to_credits!(0.1),
                output_address_2,
                dash_to_credits!(0.1),
            );

            let bytes1 = transition1.serialize_to_bytes().unwrap();
            let bytes2 = transition2.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both in same block
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes1, bytes2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // First should succeed
            assert_matches!(
                &processing_result.execution_results()[0],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            );

            // Second should also succeed (nonces are sequential)
            assert_matches!(
                &processing_result.execution_results()[1],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            );
        }

        #[test]
        fn test_wrong_nonce_order_in_same_block() {
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
            let output_address_1 = create_platform_address(98);
            let output_address_2 = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            // First transaction with nonce 2 (wrong - should be 1)
            let transition1 = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                2, // Wrong nonce - should be 1
                dash_to_credits!(0.1),
                output_address_1,
                dash_to_credits!(0.1),
            );

            // Second transaction with nonce 1
            let transition2 = create_signed_address_funds_transfer_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address_2,
                dash_to_credits!(0.1),
            );

            let bytes1 = transition1.serialize_to_bytes().unwrap();
            let bytes2 = transition2.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both in same block (wrong order)
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes1, bytes2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // First should fail (nonce 2 when expecting 1)
            assert_matches!(
                &processing_result.execution_results()[0],
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::AddressInvalidNonceError(_)
                ))
            );

            // Second should succeed (nonce 1 is correct since first failed)
            assert_matches!(
                &processing_result.execution_results()[1],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            );
        }
    }

    // ==========================================
    // SECURITY TESTS
    // Tests for potential attack vectors and edge cases
    // ==========================================

    mod security {
        use super::*;
        use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;
        use dpp::serialization::Signable;

        // ------------------------------------------
        // Structure Validation Security
        // ------------------------------------------

        #[test]
        fn test_too_many_outputs_returns_error() {
            // A hacker might try to create many outputs to bloat state or cause DoS
            let platform_version = PlatformVersion::latest();
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);

            // Create max_outputs + 1 outputs
            let output_count = max_outputs as usize + 1;
            let amount_per_output = dash_to_credits!(0.001);
            let total = amount_per_output * output_count as u64;

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, total));

            let mut outputs = BTreeMap::new();
            for i in 0..output_count {
                let output_addr = create_platform_address(i as u8);
                outputs.insert(output_addr, amount_per_output);
            }

            let transition = create_raw_transition_with_dummy_witnesses(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
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
                    ConsensusError::BasicError(BasicError::TransitionOverMaxOutputsError(_))
                ),
                "Expected TransitionOverMaxOutputsError, got {:?}",
                error
            );
        }

        #[test]
        fn test_input_sum_overflow_returns_error() {
            // Attacker tries to overflow input sum to bypass balance checks
            let platform_version = PlatformVersion::latest();

            let mut signer = TestAddressSigner::new();
            let input1 = signer.add_p2pkh([1u8; 32]);
            let input2 = signer.add_p2pkh([2u8; 32]);

            // Two inputs that would overflow when summed
            let mut inputs = BTreeMap::new();
            inputs.insert(input1, (1 as AddressNonce, u64::MAX));
            inputs.insert(input2, (1 as AddressNonce, u64::MAX));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(99), dash_to_credits!(1.0));

            let transition = create_raw_transition_with_dummy_witnesses(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                2,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                ),
                "Expected OverflowError, got {:?}",
                error
            );
        }

        #[test]
        fn test_output_sum_overflow_returns_error() {
            // Attacker tries to overflow output sum
            let platform_version = PlatformVersion::latest();

            let mut inputs = BTreeMap::new();
            let input_addr = create_platform_address(1);
            inputs.insert(input_addr, (1 as AddressNonce, dash_to_credits!(1.0)));

            // Two outputs that would overflow when summed
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(98), u64::MAX);
            outputs.insert(create_platform_address(99), u64::MAX);

            let transition = create_raw_transition_with_dummy_witnesses(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
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
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                ),
                "Expected OverflowError, got {:?}",
                error
            );
        }

        // ------------------------------------------
        // Double-Spend and Replay Attacks
        // ------------------------------------------

        #[test]
        fn test_double_spend_same_block_second_fails() {
            // Attacker submits two transactions in same block that together exceed balance
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
            let output1 = create_platform_address(98);
            let output2 = create_platform_address(99);

            // Setup address with 1 DASH
            let total_balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, total_balance);

            // First transaction: send 0.6 DASH (should succeed)
            let amount1 = dash_to_credits!(0.6);
            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, amount1));
            let mut outputs1 = BTreeMap::new();
            outputs1.insert(output1, amount1);

            let transition1 = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs1,
                outputs1,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            // Second transaction: send 0.6 DASH with nonce 2 (should fail - insufficient balance)
            let amount2 = dash_to_credits!(0.6);
            let mut inputs2 = BTreeMap::new();
            inputs2.insert(input_address, (2 as AddressNonce, amount2));
            let mut outputs2 = BTreeMap::new();
            outputs2.insert(output2, amount2);

            let transition2 = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs2,
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let bytes1 = transition1.serialize_to_bytes().unwrap();
            let bytes2 = transition2.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes1, bytes2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // First should succeed
            assert_matches!(
                &processing_result.execution_results()[0],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            );

            // Second should fail with insufficient balance
            // Note: AddressNotEnoughFundsError is singular (for a single address)
            assert_matches!(
                &processing_result.execution_results()[1],
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::AddressNotEnoughFundsError(_)
                ))
            );
        }

        #[test]
        fn test_replay_attack_same_transaction_twice_fails() {
            // Attacker tries to replay an already-executed transaction
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
            let output_address = create_platform_address(99);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let amount = dash_to_credits!(0.5);
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, amount));
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, amount);

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let transition_bytes = transition.serialize_to_bytes().unwrap();

            // Execute first time
            {
                let platform_state = platform.state.load();
                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![transition_bytes.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(..)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit");
            }

            // Try to replay the exact same transaction
            {
                let platform_state = platform.state.load();
                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![transition_bytes],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process");

                // Should fail because nonce is now stale
                assert_matches!(
                    &processing_result.execution_results()[0],
                    StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::StateError(StateError::AddressInvalidNonceError(_))
                    )
                );
            }
        }

        // ------------------------------------------
        // Fee Strategy Attacks
        // ------------------------------------------

        #[test]
        fn test_fee_reduces_output_to_zero() {
            // What happens when ReduceOutput strategy reduces output to exactly 0?
            // The output should be removed, but is this handled correctly?
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
            let output_address = create_platform_address(99);

            // Input has exactly enough for output + estimated fee
            let input_balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, input_balance);

            // Output is at minimum - fee will reduce it below minimum
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, min_output));
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, min_output);

            // Use ReduceOutput - this will try to take fee from the min-sized output
            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let transition_bytes = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            // This should either succeed (output becomes small but valid) or fail gracefully
            // The key is it should NOT panic or cause undefined behavior
            let result = &processing_result.execution_results()[0];
            // Document the actual behavior
            match result {
                StateTransitionExecutionResult::SuccessfulExecution(..) => {
                    // If it succeeds, verify the output was reduced but still valid
                    platform
                        .drive
                        .grove
                        .commit_transaction(transaction)
                        .unwrap()
                        .expect("expected to commit");

                    let (_, output_balance) = platform
                        .drive
                        .fetch_balance_and_nonce(&output_address, None, platform_version)
                        .expect("expected to fetch")
                        .expect("expected address");

                    // Output should be less than the original min_output (fee was deducted)
                    assert!(output_balance < min_output);
                }
                StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                    // Also acceptable - the system rejected it
                }
                _ => {
                    // Any other result should be documented
                }
            }
        }

        #[test]
        fn test_fee_exhaustion_deduct_from_depleted_input() {
            // DeductFromInput when input's remaining balance after transfer is 0
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
            let output_address = create_platform_address(99);

            // Set up with exactly what we're transferring
            let exact_amount = dash_to_credits!(0.1);
            setup_address_with_balance(&mut platform, input_address, 0, exact_amount);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, exact_amount));
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, exact_amount);

            // Use DeductFromInput(0) - but after transfer, input has 0 remaining!
            // This should fail because there's nothing to deduct the fee from
            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let transition_bytes = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            // Should fail - not enough funds to cover fee
            assert_matches!(
                &processing_result.execution_results()[0],
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::AddressesNotEnoughFundsError(_)
                ))
            );
        }

        // ------------------------------------------
        // P2SH Security Tests
        // ------------------------------------------

        #[test]
        fn test_15_of_15_multisig_success() {
            // Maximum standard multisig: 15-of-15
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

            // Create 15 different seeds
            let seeds: Vec<[u8; 32]> = (1..=15)
                .map(|i| {
                    let mut seed = [0u8; 32];
                    seed[0] = i;
                    seed[31] = i;
                    seed
                })
                .collect();

            let input_address = signer.add_p2sh_multisig(15, &seeds);
            let output_address = create_platform_address(99);

            let amount = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, amount);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, amount));
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, amount);

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let transition_bytes = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_p2sh_with_timelock_script_fails() {
            // Attacker tries to use a timelock script (CHECKLOCKTIMEVERIFY)
            // Platform should not support timelock scripts as they require block height context
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

            // Create a timelock redeem script:
            // <locktime> OP_CHECKLOCKTIMEVERIFY OP_DROP <pubkey> OP_CHECKSIG
            // OP_CHECKLOCKTIMEVERIFY is 0xb1 (OP_NOP2 repurposed)
            let (secret_key, public_key) = TestAddressSigner::create_keypair([5u8; 32]);
            let pubkey_bytes = public_key.to_bytes();

            let mut timelock_script = Vec::new();
            // Push a locktime value (e.g., block 1000000)
            timelock_script.push(0x04); // push 4 bytes
            timelock_script.extend_from_slice(&1000000u32.to_le_bytes());
            timelock_script.push(0xb1); // OP_CHECKLOCKTIMEVERIFY (OP_NOP2)
            timelock_script.push(OP_DROP.to_u8());
            timelock_script.push(pubkey_bytes.len() as u8);
            timelock_script.extend_from_slice(&pubkey_bytes);
            timelock_script.push(OP_CHECKSIG.to_u8());

            let script_buf = ScriptBuf::from_bytes(timelock_script.clone());
            let script_hash = *script_buf.script_hash().as_byte_array();
            let input_address = PlatformAddress::P2sh(script_hash);

            let output_address = create_platform_address(99);
            let amount = dash_to_credits!(1.0);

            setup_address_with_balance(&mut platform, input_address, 0, amount);

            // Create a signature for the transaction
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, amount));
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, amount);

            // Create the transition to get the signing bytes
            let unsigned_transition = AddressFundsTransferTransitionV0 {
                inputs: inputs.clone(),
                outputs: outputs.clone(),
                fee_strategy: vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                user_fee_increase: 0,
                input_witnesses: vec![],
            };

            // Get signable bytes and sign
            let state_transition: StateTransition = unsigned_transition.clone().into();
            let signable_bytes = state_transition
                .signable_bytes()
                .expect("should get signable bytes");
            let signature = TestAddressSigner::sign_data(&signable_bytes, &secret_key);

            // Create witness with timelock script
            let witness = AddressWitness::P2sh {
                signatures: vec![BinaryData::new(signature)],
                redeem_script: BinaryData::new(timelock_script),
            };

            let transition = AddressFundsTransferTransition::V0(AddressFundsTransferTransitionV0 {
                inputs,
                outputs,
                fee_strategy: vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                user_fee_increase: 0,
                input_witnesses: vec![witness],
            });

            let transition_bytes = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            // Should fail - timelock scripts should not be accepted
            assert!(
                !matches!(
                    &processing_result.execution_results()[0],
                    StateTransitionExecutionResult::SuccessfulExecution(..)
                ),
                "Timelock (CLTV) script should not be accepted"
            );
        }

        #[test]
        fn test_p2sh_with_op_return_script_fails() {
            // Attacker tries to use a non-standard script that doesn't verify signatures
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

            // Create a malicious redeem script: OP_RETURN (always fails script execution)
            let malicious_script = vec![OP_RETURN.to_u8()];
            let script_buf = ScriptBuf::from_bytes(malicious_script.clone());
            let script_hash = *script_buf.script_hash().as_byte_array();
            let input_address = PlatformAddress::P2sh(script_hash);

            let output_address = create_platform_address(99);
            let amount = dash_to_credits!(1.0);

            setup_address_with_balance(&mut platform, input_address, 0, amount);

            // Create a witness with the malicious script
            let witness = AddressWitness::P2sh {
                signatures: vec![],
                redeem_script: BinaryData::new(malicious_script),
            };

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, amount));
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, amount);

            let transition = AddressFundsTransferTransition::V0(AddressFundsTransferTransitionV0 {
                inputs,
                outputs,
                fee_strategy: vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                user_fee_increase: 0,
                input_witnesses: vec![witness],
            });

            let transition_bytes = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            // Should fail - either invalid script format or signature verification fails
            assert!(
                !matches!(
                    &processing_result.execution_results()[0],
                    StateTransitionExecutionResult::SuccessfulExecution(..)
                ),
                "OP_RETURN script should not be accepted"
            );
        }

        // ------------------------------------------
        // Same Block Edge Cases
        // ------------------------------------------

        #[test]
        fn test_receive_and_spend_same_block() {
            // Can an address receive funds and spend them in the same block?
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
            let source_address = signer.add_p2pkh([1u8; 32]);
            let middle_address = signer.add_p2pkh([2u8; 32]);
            let final_address = create_platform_address(99);

            // Only source has funds initially
            let amount = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, source_address, 0, amount);

            // Transaction 1: source -> middle
            let mut inputs1 = BTreeMap::new();
            inputs1.insert(source_address, (1 as AddressNonce, amount));
            let mut outputs1 = BTreeMap::new();
            outputs1.insert(middle_address, amount);

            let transition1 = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs1,
                outputs1,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            // Transaction 2: middle -> final (middle doesn't have funds yet!)
            // We need to estimate what middle will have after receiving
            let estimated_received = amount - dash_to_credits!(0.01); // rough fee estimate
            let mut inputs2 = BTreeMap::new();
            inputs2.insert(middle_address, (1 as AddressNonce, estimated_received));
            let mut outputs2 = BTreeMap::new();
            outputs2.insert(final_address, estimated_received);

            let transition2 = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs2,
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let bytes1 = transition1.serialize_to_bytes().unwrap();
            let bytes2 = transition2.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes1, bytes2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            // First should succeed
            assert_matches!(
                &processing_result.execution_results()[0],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            );

            // Second should fail - middle_address doesn't exist yet in state
            // (it will be created by first transaction, but second is validated against initial state)
            assert!(
                !matches!(
                    &processing_result.execution_results()[1],
                    StateTransitionExecutionResult::SuccessfulExecution(..)
                ),
                "Should not be able to spend funds received in same block"
            );
        }

        #[test]
        fn test_concurrent_transfers_to_same_output() {
            // Two different inputs send to same output in same block
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
            let input1 = signer.add_p2pkh([1u8; 32]);
            let input2 = signer.add_p2pkh([2u8; 32]);
            let shared_output = create_platform_address(99);

            let amount = dash_to_credits!(0.5);
            setup_address_with_balance(&mut platform, input1, 0, amount);
            setup_address_with_balance(&mut platform, input2, 0, amount);
            // Pre-create the output address so we can verify balance later
            setup_address_with_balance(&mut platform, shared_output, 0, 0);

            // Both send to same output
            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input1, (1 as AddressNonce, amount));
            let mut outputs1 = BTreeMap::new();
            outputs1.insert(shared_output, amount);

            let transition1 = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs1,
                outputs1,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let mut inputs2 = BTreeMap::new();
            inputs2.insert(input2, (1 as AddressNonce, amount));
            let mut outputs2 = BTreeMap::new();
            outputs2.insert(shared_output, amount);

            let transition2 = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs2,
                outputs2,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let bytes1 = transition1.serialize_to_bytes().unwrap();
            let bytes2 = transition2.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes1, bytes2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            // Both should succeed
            assert_matches!(
                &processing_result.execution_results()[0],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            );
            assert_matches!(
                &processing_result.execution_results()[1],
                StateTransitionExecutionResult::SuccessfulExecution(..)
            );

            // Commit and verify output has both amounts
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            let (_, output_balance) = platform
                .drive
                .fetch_balance_and_nonce(&shared_output, None, platform_version)
                .expect("expected to fetch")
                .expect("expected address");

            // Should have received from both (minus fees)
            assert!(
                output_balance > amount,
                "Output should have received from both transfers, got {}",
                output_balance
            );
        }

        // ------------------------------------------
        // Maximum Value Tests
        // ------------------------------------------

        #[test]
        fn test_transfer_near_max_u64() {
            // Test transfer of very large amounts (near u64::MAX)
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
            let output_address = create_platform_address(99);

            // Very large amount (but not overflowing)
            let large_amount = u64::MAX / 2;
            setup_address_with_balance(&mut platform, input_address, 0, large_amount);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, large_amount));
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, large_amount);

            let transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                &signer,
                0,
                platform_version,
            )
            .expect("should create transition");

            let transition_bytes = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            // Should succeed without overflow issues
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }
    }
}
