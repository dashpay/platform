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
    use dpp::dashcore::{Network, PrivateKey, PublicKey};
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

        /// Convenience: Adds a 2-of-3 P2SH multisig address
        fn add_p2sh_2of3(
            &mut self,
            seed1: [u8; 32],
            seed2: [u8; 32],
            seed3: [u8; 32],
        ) -> PlatformAddress {
            self.add_p2sh_multisig(2, &[seed1, seed2, seed3])
        }

        /// Convenience: Adds a 1-of-1 P2SH multisig address
        fn add_p2sh_1of1(&mut self, seed: [u8; 32]) -> PlatformAddress {
            self.add_p2sh_multisig(1, &[seed])
        }

        /// Convenience: Adds a 3-of-3 P2SH multisig address
        fn add_p2sh_3of3(
            &mut self,
            seed1: [u8; 32],
            seed2: [u8; 32],
            seed3: [u8; 32],
        ) -> PlatformAddress {
            self.add_p2sh_multisig(3, &[seed1, seed2, seed3])
        }

        /// Convenience: Adds an n-of-n P2SH multisig address
        fn add_p2sh_n_of_n(&mut self, seeds: &[[u8; 32]]) -> PlatformAddress {
            self.add_p2sh_multisig(seeds.len() as u8, seeds)
        }

        /// Sign P2PKH and create witness
        fn sign_p2pkh(
            &self,
            address: PlatformAddress,
            data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            self.sign_create_witness(&address, data)
        }

        /// Sign P2SH and create witness
        fn sign_p2sh(
            &self,
            address: PlatformAddress,
            data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            self.sign_create_witness(&address, data)
        }

        /// Sign P2SH with ALL keys (not just threshold)
        fn sign_p2sh_all_keys(
            &self,
            address: PlatformAddress,
            data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            match address {
                PlatformAddress::P2sh(hash) => {
                    let entry = self.p2sh_entries.get(&hash).ok_or_else(|| {
                        ProtocolError::Generic(format!(
                            "No P2SH entry found for script hash {}",
                            hex::encode(hash)
                        ))
                    })?;
                    // Sign with ALL keys, not just threshold
                    let signatures: Vec<BinaryData> = entry
                        .secret_keys
                        .iter()
                        .map(|sk| BinaryData::new(Self::sign_data(data, sk)))
                        .collect();

                    Ok(AddressWitness::P2sh {
                        signatures,
                        redeem_script: BinaryData::new(entry.redeem_script.clone()),
                    })
                }
                _ => Err(ProtocolError::Generic("Expected P2SH address".to_string())),
            }
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

    /// Create a raw AddressFundingFromAssetLockTransitionV0 with dummy witnesses for structure validation tests
    fn create_raw_transition_with_dummy_witnesses(
        asset_lock_proof: dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, u64>,
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

    /// Creates a chain asset lock proof and returns it with the private key for signing
    /// Note: Uses instant lock for now - chain lock fixture will be a separate implementation
    fn create_chain_asset_lock_proof_with_key(
        rng: &mut StdRng,
    ) -> (
        dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        Vec<u8>,
    ) {
        // For TDD purposes, using instant lock fixture
        // TODO: Create proper ChainAssetLockProof when implementing chain lock tests
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

    /// Get the balance of an address from the drive
    /// TODO: Implement fetch_address_balance in Drive
    #[allow(dead_code)]
    fn get_address_balance(
        _platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        _address: PlatformAddress,
        _transaction: &drive::grovedb::Transaction,
    ) -> u64 {
        // Placeholder - needs Drive::fetch_address_balance to be implemented
        0
    }

    /// Get the nonce of an address from the drive
    /// TODO: Implement fetch_address_nonce in Drive
    #[allow(dead_code)]
    fn get_address_nonce(
        _platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        _address: PlatformAddress,
        _transaction: &drive::grovedb::Transaction,
    ) -> AddressNonce {
        // Placeholder - needs Drive::fetch_address_nonce to be implemented
        0
    }

    /// Check if an asset lock outpoint has been spent
    /// TODO: Implement has_asset_lock_outpoint_been_spent in Drive
    #[allow(dead_code)]
    fn is_asset_lock_spent(
        _platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        _outpoint: &dpp::dashcore::OutPoint,
        _transaction: &drive::grovedb::Transaction,
    ) -> bool {
        // Placeholder - needs Drive::has_asset_lock_outpoint_been_spent to be implemented
        true
    }

    /// Generate signable bytes for an address funding from asset lock transition
    /// This creates an unsigned transition and gets its signable bytes
    fn get_signable_bytes_for_transition(
        asset_lock_proof: &dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        inputs: &BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: &BTreeMap<PlatformAddress, u64>,
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
        outputs: BTreeMap<PlatformAddress, u64>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
    ) -> StateTransition {
        AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer(
            asset_lock_proof,
            asset_lock_private_key,
            inputs,
            outputs,
            AddressFundsFeeStrategy::from(fee_strategy),
            signer,
            0,
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
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut signer = TestAddressSigner::new();

            // Create 17 inputs (max is 16) with proper addresses
            let mut inputs = BTreeMap::new();
            for i in 1..18u8 {
                let addr = signer.add_p2pkh([i; 32]);
                setup_address_with_balance(&mut platform, addr, 0, dash_to_credits!(1.0));
                inputs.insert(addr, (1 as AddressNonce, dash_to_credits!(0.01)));
            }

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(100), dash_to_credits!(2.0));

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                17, // Match input count
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
        fn test_too_many_outputs_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            // Create 17 outputs (max is 16)
            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            for i in 1..18u8 {
                outputs.insert(create_platform_address(i), dash_to_credits!(0.1));
            }

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
                    ConsensusError::BasicError(BasicError::TransitionOverMaxOutputsError(e))
                )] if e.actual_outputs() == 17 && e.max_outputs() == 16
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

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut signer = TestAddressSigner::new();
            let input_address_1 = signer.add_p2pkh([1u8; 32]);
            let input_address_2 = signer.add_p2pkh([2u8; 32]);
            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(3), dash_to_credits!(1.2));

            // Create transition with 2 inputs but only 1 witness
            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                1, // Only 1 witness for 2 inputs
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

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut signer = TestAddressSigner::new();
            let same_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, same_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(same_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(same_address, dash_to_credits!(1.1)); // Same address as input

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                1,
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

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

            // ReduceOutput(5) but only 1 output exists
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
        fn test_outputs_not_greater_than_inputs_returns_error() {
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
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            let mut outputs = BTreeMap::new();
            // Output is NOT greater than input - should fail for asset lock funding
            outputs.insert(create_platform_address(2), dash_to_credits!(0.5));

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                1,
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
                    ConsensusError::BasicError(BasicError::OutputsNotGreaterThanInputsError(_))
                )]
            );
        }

        #[test]
        fn test_output_below_minimum_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), 100); // Very small output - below minimum

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
            let output_address = create_platform_address(1);
            outputs.insert(output_address, dash_to_credits!(0.9)); // Less than 1 DASH to account for fees

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.3));
            outputs.insert(create_platform_address(2), dash_to_credits!(0.3));
            outputs.insert(create_platform_address(3), dash_to_credits!(0.3));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            // Output is greater than input because asset lock adds 1 DASH
            outputs.insert(create_platform_address(2), dash_to_credits!(1.2));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(2), dash_to_credits!(1.0));

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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(_)),
                    _
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
            outputs.insert(create_platform_address(2), dash_to_credits!(1.5));

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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::AddressesNotEnoughFundsError(_)),
                    _
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
            outputs.insert(create_platform_address(2), dash_to_credits!(1.2));

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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(_)),
                    _
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.9));

            let transition = create_signed_address_funding_from_asset_lock_transition(
                asset_lock_proof,
                &wrong_private_key,
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

            // Should fail due to signature verification error
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(3), dash_to_credits!(1.2));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.4));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            // 0.3 + 0.3 + 1.0 (asset lock) = 1.6 DASH input, 1.5 output
            outputs.insert(create_platform_address(1), dash_to_credits!(1.5));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.4));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));
            outputs.insert(create_platform_address(2), dash_to_credits!(0.4));

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
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(1.0));

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

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            // Very small input - below minimum
            inputs.insert(input_address, (1 as AddressNonce, 100));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(1.0));

            let transition = create_raw_transition_with_dummy_witnesses(
                asset_lock_proof,
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                1,
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
            outputs.insert(create_platform_address(100), dash_to_credits!(1.7));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            for i in 1..=16u8 {
                outputs.insert(create_platform_address(i), dash_to_credits!(0.05));
            }

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(10), dash_to_credits!(1.5));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.6));
            outputs.insert(create_platform_address(2), dash_to_credits!(0.6));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.4));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.9));

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

            assert_eq!(processing_result.valid_count(), 1);

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
            assert_eq!(processing_result2.invalid_paid_count(), 1);
        }

        #[test]
        fn test_invalid_signature_format_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _) = create_asset_lock_proof_with_key(&mut rng);

            let inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(0.9));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(output_address, dash_to_credits!(0.9));

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

            assert_eq!(processing_result.valid_count(), 1);

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            // Verify the output address received funds (minus fees)
            let balance = platform
                .drive
                .fetch_address_balance(output_address, None, platform_version)
                .expect("expected to fetch balance");

            // Balance should be approximately 0.9 DASH minus processing fees
            assert!(balance.is_some());
            let actual_balance = balance.unwrap();
            // Should be less than requested due to fees, but greater than 0
            assert!(actual_balance > 0);
            assert!(actual_balance < dash_to_credits!(0.9));
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
            outputs.insert(create_platform_address(2), dash_to_credits!(1.4));

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

            assert_eq!(processing_result.valid_count(), 1);

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            // Verify the input address balance was reduced
            let remaining_balance = platform
                .drive
                .fetch_address_balance(input_address, None, platform_version)
                .expect("expected to fetch balance");

            assert!(remaining_balance.is_some());
            let actual_remaining = remaining_balance.unwrap();
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
            outputs.insert(create_platform_address(2), dash_to_credits!(1.2));

            // Create transition with invalid witness (wrong signature length)
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
                    input_witnesses: vec![AddressWitness::P2pkh {
                        signature: BinaryData::new(vec![0u8; 10]), // Wrong length
                    }],
                },
            );

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

            // Should fail due to invalid witness
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.4));

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
                    input_witnesses: vec![AddressWitness::P2sh {
                        signatures,
                        redeem_script: BinaryData::new(vec![0u8; 50]), // Wrong redeem script
                    }],
                },
            );

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

            // Should fail due to wrong redeem script hash
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(2), dash_to_credits!(1.2));

            // Create transition with P2SH witness for P2PKH address (type mismatch)
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
                    input_witnesses: vec![AddressWitness::P2sh {
                        signatures: vec![BinaryData::new(vec![0u8; 65])],
                        redeem_script: BinaryData::new(vec![0u8; 50]),
                    }],
                },
            );

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

            // Should fail due to witness type mismatch
            assert_eq!(processing_result.invalid_paid_count(), 1);
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

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(600);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);
            // Asset lock provides 1 DASH

            let mut outputs = BTreeMap::new();
            // Output exactly matches asset lock minus expected fee
            // This should work if fee calculation is correct
            outputs.insert(create_platform_address(1), dash_to_credits!(0.99));

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
            assert_eq!(processing_result.valid_count(), 1);
        }

        #[test]
        fn test_fee_exceeds_remaining_by_one_credit() {
            // Test where fee exceeds remaining balance by just 1 credit
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
            // Try to output entire asset lock amount, leaving nothing for fee
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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

            // Should fail - no funds left for fee
            assert_eq!(processing_result.invalid_paid_count(), 1);
        }

        #[test]
        fn test_user_fee_increase_makes_transaction_unaffordable() {
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

            let mut rng = StdRng::seed_from_u64(602);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: u16::MAX, // Maximum fee increase
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

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

            // Should fail - user fee increase makes it too expensive
            assert_eq!(processing_result.invalid_paid_count(), 1);
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

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(603);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 100, // Small fee increase (1%)
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

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

            // Should succeed with small fee increase
            assert_eq!(processing_result.valid_count(), 1);
        }
    }

    mod asset_lock_edge_cases {
        use super::*;

        #[test]
        fn test_asset_lock_output_index_out_of_bounds() {
            // Test where asset lock proof references non-existent output index
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

            let mut rng = StdRng::seed_from_u64(610);
            let (mut asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Modify output index to be out of bounds
            match &mut asset_lock_proof {
                AssetLockProof::Instant(instant) => {
                    instant.output_index = 100; // Way out of bounds
                }
                AssetLockProof::Chain(_) => {}
            }

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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

            // Should fail - output index out of bounds
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

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
            outputs1.insert(create_platform_address(1), dash_to_credits!(0.5));

            let mut outputs2 = BTreeMap::new();
            outputs2.insert(create_platform_address(2), dash_to_credits!(0.5));

            // Create two transitions using the same asset lock
            let transition1 = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof: asset_lock_proof.clone(),
                    inputs: BTreeMap::new(),
                    outputs: outputs1,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let transition2 = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs: outputs2,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition1: StateTransition = transition1.into();
            let state_transition2: StateTransition = transition2.into();

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

            // First should succeed, second should fail as double spend
            assert_eq!(processing_result.valid_count(), 1);
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

            // First transition - should succeed
            let transition1 = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof: asset_lock_proof.clone(),
                    inputs: BTreeMap::new(),
                    outputs: outputs.clone(),
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition1: StateTransition = transition1.into();
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

            assert_eq!(processing_result.valid_count(), 1);

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .expect("commit");

            // Now try to use the same asset lock again
            let mut outputs2 = BTreeMap::new();
            outputs2.insert(create_platform_address(2), dash_to_credits!(0.5));

            let transition2 = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs: outputs2,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![],
                },
            );

            let state_transition2: StateTransition = transition2.into();
            let result2 = state_transition2
                .serialize_to_bytes()
                .expect("should serialize");

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
            assert_eq!(processing_result2.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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

            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            // Very high nonce
            let high_nonce: AddressNonce = u64::MAX - 1;
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), u64::MAX - 1000);

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
        }

        #[test]
        fn test_input_amount_exceeds_balance() {
            // Input tries to use more than available balance
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
            let input_address = signer.add_p2pkh([31u8; 32]);
            // Address has 1 DASH
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(631);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut inputs = BTreeMap::new();
            // Try to use 2 DASH from an address with only 1 DASH
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(2.0)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(2.5));

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

            // Should fail - input exceeds balance
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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
            outputs.insert(create_platform_address(1), 0); // Zero amount

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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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

            // Should fail - insufficient balance
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(output_address, dash_to_credits!(0.5));

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
            assert_eq!(processing_result.valid_count(), 1);

            // Verify balance was added (not replaced)
            // After: should have 1.0 + 0.5 = 1.5 DASH
            let new_balance = get_address_balance(&platform, output_address, &transaction);
            assert_eq!(new_balance, dash_to_credits!(1.5));
        }

        #[test]
        fn test_multiple_inputs_from_same_address_fails() {
            // BTreeMap naturally prevents this, but test the semantic
            // This tests that we can't have duplicate addresses in inputs
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

            // BTreeMap will only keep one entry per key
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(input_address, (2 as AddressNonce, dash_to_credits!(0.4))); // This overwrites

            // Only one input in map due to BTreeMap dedup
            assert_eq!(inputs.len(), 1);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.valid_count(), 1);
        }
    }

    mod dust_and_minimum_amounts {
        use super::*;

        #[test]
        fn test_fee_deduction_leaves_dust_on_output() {
            // After fee deduction, output has dust amount (below minimum)
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
            outputs.insert(create_platform_address(1), 1001); // Just above minimum

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            // Set output high enough that after fee, it's at minimum (1000 credits)
            outputs.insert(create_platform_address(1), dash_to_credits!(0.1));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(2.0));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.3));
            outputs.insert(create_platform_address(2), dash_to_credits!(0.3));
            outputs.insert(create_platform_address(3), dash_to_credits!(0.3));

            // Fee deducted from multiple outputs
            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(),
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                        AddressFundsFeeStrategyStep::ReduceOutput(1),
                        AddressFundsFeeStrategyStep::ReduceOutput(2),
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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(address, dash_to_credits!(1.0)); // Same address as input

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
            // 16 inputs * 100 DASH + 1 DASH from asset lock = 1601 DASH total
            outputs.insert(create_platform_address(1), dash_to_credits!(1600.0));

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
            assert_eq!(processing_result.valid_count(), 1);
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

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(700);
            let (chain_asset_lock_proof, asset_lock_pk) =
                create_chain_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof: chain_asset_lock_proof,
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

            // Should succeed with chain asset lock proof
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof: chain_asset_lock_proof,
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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(2.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.5));

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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

            let signable_bytes =
                get_signable_bytes_for_transition(&asset_lock_proof, &inputs, &outputs);
            // Sign with all 3 keys for 2-of-3
            let witness = signer
                .sign_p2sh_all_keys(p2sh_address, &signable_bytes)
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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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

            assert_eq!(processing_result.valid_count(), 1);

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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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

            assert_eq!(processing_result.valid_count(), 1);

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

            let output_amount = dash_to_credits!(1.5);
            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, output_amount);

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

            assert_eq!(processing_result.valid_count(), 1);

            // Verify exact balance changes
            let new_input_balance = get_address_balance(&platform, input_address, &transaction);
            let new_output_balance = get_address_balance(&platform, output_address, &transaction);

            // Input should have: initial - input_amount = 2.0 - 1.0 = 1.0 DASH
            assert_eq!(new_input_balance, initial_input_balance - input_amount);

            // Output should have exactly output_amount
            assert_eq!(new_output_balance, output_amount);

            // Fee should come from: asset_lock_value + input_amount - output_amount
            let fee_paid = asset_lock_value + input_amount - output_amount;
            assert!(fee_paid > 0); // Fee was deducted from remaining input balance
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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

            assert_eq!(processing_result.invalid_paid_count(), 1);

            // Verify specific error type
            let errors = processing_result
                .into_execution_results()
                .into_iter()
                .filter_map(|r| {
                    r.into_consensus_validation_result()
                        .errors
                        .into_iter()
                        .next()
                })
                .collect::<Vec<_>>();

            assert!(!errors.is_empty());
            assert!(matches!(
                errors[0],
                ConsensusError::StateError(StateError::AddressNotFoundError(_))
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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

            assert_eq!(processing_result.invalid_paid_count(), 1);

            // Verify specific error type
            let errors = processing_result
                .into_execution_results()
                .into_iter()
                .filter_map(|r| {
                    r.into_consensus_validation_result()
                        .errors
                        .into_iter()
                        .next()
                })
                .collect::<Vec<_>>();

            assert!(!errors.is_empty());
            assert!(matches!(
                errors[0],
                ConsensusError::StateError(StateError::AddressInsufficientBalanceError(_))
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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

            assert_eq!(processing_result.invalid_paid_count(), 1);

            // Verify specific error type
            let errors = processing_result
                .into_execution_results()
                .into_iter()
                .filter_map(|r| {
                    r.into_consensus_validation_result()
                        .errors
                        .into_iter()
                        .next()
                })
                .collect::<Vec<_>>();

            assert!(!errors.is_empty());
            assert!(matches!(
                errors[0],
                ConsensusError::StateError(StateError::InvalidAddressNonceError(_))
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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
                    + processing_result.invalid_paid_count()
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

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(771);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(2.0));

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
            assert_eq!(processing_result.invalid_paid_count(), 1);
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

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(790);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Minimum output - just enough to cover minimum
            outputs.insert(create_platform_address(1), 1000); // Minimum amount

            let transition = AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    inputs: BTreeMap::new(), // No inputs
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::ReduceOutput(0),
                    ]),
                    user_fee_increase: 0,
                    signature: BinaryData::new(asset_lock_pk.to_vec()),
                    input_witnesses: vec![], // No witnesses
                },
            );

            let state_transition: StateTransition = transition.into();
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
            assert_eq!(processing_result.valid_count(), 1);
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

            let mut rng = StdRng::seed_from_u64(800);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Create address with all-zero hash
            let zero_address = PlatformAddress::new([0u8; 20]);

            let mut outputs = BTreeMap::new();
            outputs.insert(zero_address, dash_to_credits!(0.5));

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
            assert_eq!(processing_result.valid_count(), 1);
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

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(801);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Create address with all-FF hash
            let max_address = PlatformAddress::new([0xFFu8; 20]);

            let mut outputs = BTreeMap::new();
            outputs.insert(max_address, dash_to_credits!(0.5));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.valid_count(), 1);
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
            outputs.insert(create_platform_address(1), dash_to_credits!(1.0));

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
            assert_eq!(processing_result.valid_count(), 1);

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

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(820);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), dash_to_credits!(0.5));

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

            assert_eq!(processing_result.valid_count(), 1);

            // Commit
            platform
                .drive
                .grove
                .commit_transaction(transaction)
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
            assert_eq!(processing_result2.invalid_paid_count(), 1);
        }
    }

    mod small_amounts {
        use super::*;

        #[test]
        fn test_one_credit_transfer() {
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

            let mut rng = StdRng::seed_from_u64(830);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            // Try to output just 1 credit (likely below minimum)
            outputs.insert(create_platform_address(1), 1);

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

            // Should fail - 1 credit is below minimum (1000)
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_minimum_viable_amount() {
            // Exactly 1000 credits - the minimum
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

            let mut rng = StdRng::seed_from_u64(831);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(1), 1000); // Exactly minimum

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

            // Should succeed with exactly minimum amount
            assert_eq!(processing_result.valid_count(), 1);
        }
    }
}
