#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::signature::SignatureError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::dashcore::{Network, PrivateKey};
    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::platform_value::BinaryData;
    use dpp::serialization::{PlatformSerializable, Signable};
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
    use grovedb_commitment_tree::{
        Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
        Flags as OrchardFlags, FullViewingKey, NoteValue, ProvingKey, Scope, SpendingKey,
    };
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::rngs::OsRng;
    use rand::SeedableRng;
    use std::sync::OnceLock;

    // ==========================================
    // Helper Functions
    // ==========================================

    /// Create a `SerializedAction` with syntactically valid sizes but meaningless crypto data.
    /// Passes structure validation (correct field sizes) but will fail ZK proof verification.
    fn create_dummy_serialized_action() -> SerializedAction {
        SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 692], // epk(32) + enc(580) + out(80)
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    /// Creates an asset lock proof and returns it with the private key bytes for ECDSA signing.
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

    /// Build a ShieldFromAssetLock StateTransition with the given fields and ECDSA signature.
    /// The `signature` field is computed over the signable bytes using `dashcore::signer::sign`.
    fn create_signed_shield_from_asset_lock_transition(
        asset_lock_proof: dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        asset_lock_private_key: &[u8],
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: u16,
    ) -> StateTransition {
        // Create unsigned transition to compute signable bytes
        let unsigned = ShieldFromAssetLockTransitionV0 {
            asset_lock_proof: asset_lock_proof.clone(),
            actions: actions.clone(),
            flags,
            value_balance,
            anchor,
            proof: proof.clone(),
            binding_signature,
            user_fee_increase,
            signature: Default::default(),
        };

        let state_transition: StateTransition = unsigned.into();
        let signable_bytes = state_transition
            .signable_bytes()
            .expect("should compute signable bytes");

        // Sign with the asset lock private key (ECDSA)
        let signature =
            dpp::dashcore::signer::sign(&signable_bytes, asset_lock_private_key).unwrap();

        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof,
                actions,
                flags,
                value_balance,
                anchor,
                proof,
                binding_signature,
                user_fee_increase,
                signature: BinaryData::new(signature.to_vec()),
            },
        ))
    }

    /// Build a ShieldFromAssetLock StateTransition with dummy (invalid) signature.
    /// Used for structure validation tests where the error is caught before signature check.
    fn create_unsigned_shield_from_asset_lock_transition(
        asset_lock_proof: dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
    ) -> StateTransition {
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof,
                actions,
                flags,
                value_balance,
                anchor,
                proof,
                binding_signature,
                user_fee_increase: 0,
                signature: BinaryData::new(vec![0u8; 65]), // dummy signature
            },
        ))
    }

    /// Standard platform setup for tests with instant lock signature verification disabled.
    fn setup_platform(
    ) -> crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike> {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state()
    }

    /// Execute a state transition through the full processing pipeline and return the result.
    fn process_transition(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        transition: StateTransition,
        platform_version: &PlatformVersion,
    ) -> crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult
    {
        let transition_bytes = transition
            .serialize_to_bytes()
            .expect("should serialize transition");
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();

        platform
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
            .expect("expected to process state transition")
    }

    // ==========================================
    // Orchard Proving Key (cached, ~30s to build)
    // ==========================================

    static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
    fn get_proving_key() -> &'static ProvingKey {
        TEST_PROVING_KEY.get_or_init(ProvingKey::build)
    }

    /// Extract serialized fields from an authorized Orchard bundle into the
    /// platform-compatible format: (actions, flags, value_balance, anchor, proof, binding_sig).
    fn serialize_authorized_bundle(
        bundle: &Bundle<OrchardAuthorized, i64>,
    ) -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
        let actions: Vec<SerializedAction> = bundle
            .actions()
            .iter()
            .map(|action| {
                let enc = action.encrypted_note();
                let mut encrypted_note = Vec::with_capacity(692);
                encrypted_note.extend_from_slice(&enc.epk_bytes);
                encrypted_note.extend_from_slice(&enc.enc_ciphertext);
                encrypted_note.extend_from_slice(&enc.out_ciphertext);

                SerializedAction {
                    nullifier: action.nullifier().to_bytes(),
                    rk: <[u8; 32]>::from(action.rk()),
                    cmx: action.cmx().to_bytes(),
                    encrypted_note,
                    cv_net: action.cv_net().to_bytes(),
                    spend_auth_sig: <[u8; 64]>::from(action.authorization()),
                }
            })
            .collect();

        let flags = bundle.flags().to_byte();
        let value_balance = *bundle.value_balance();
        let anchor = bundle.anchor().to_bytes();
        let proof = bundle.authorization().proof().as_ref().to_vec();
        let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());

        (actions, flags, value_balance, anchor, proof, binding_sig)
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_empty_actions_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _pk) = create_asset_lock_proof_with_key(&mut rng);

            let transition = create_unsigned_shield_from_asset_lock_transition(
                asset_lock_proof,
                vec![], // Empty actions -- invalid
                0x03,
                -1000,
                [0u8; 32],
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedNoActionsError(_))
                )]
            );
        }

        #[test]
        fn test_positive_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(568);
            let (asset_lock_proof, _pk) = create_asset_lock_proof_with_key(&mut rng);

            let transition = create_unsigned_shield_from_asset_lock_transition(
                asset_lock_proof,
                vec![create_dummy_serialized_action()],
                0x03,
                1000, // Positive -- invalid for shielding (must be negative)
                [0u8; 32],
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }

        #[test]
        fn test_zero_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(569);
            let (asset_lock_proof, _pk) = create_asset_lock_proof_with_key(&mut rng);

            let transition = create_unsigned_shield_from_asset_lock_transition(
                asset_lock_proof,
                vec![create_dummy_serialized_action()],
                0x03,
                0, // Zero -- invalid for shielding (must be negative)
                [0u8; 32],
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }

        #[test]
        fn test_empty_proof_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(570);
            let (asset_lock_proof, _pk) = create_asset_lock_proof_with_key(&mut rng);

            let transition = create_unsigned_shield_from_asset_lock_transition(
                asset_lock_proof,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                [0u8; 32],
                vec![], // Empty proof -- invalid
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedEmptyProofError(_))
                )]
            );
        }
    }

    // ==========================================
    // ASSET LOCK VALIDATION TESTS
    // ==========================================

    mod asset_lock_validation {
        use super::*;

        /// Happy path for asset lock validation: valid instant asset lock with properly signed
        /// ECDSA signature, but dummy ZK proof data. The transition should pass structure
        /// validation, asset lock validation, and ECDSA signature verification, then fail
        /// at ZK proof verification (producing a PaidConsensusError with penalty action).
        #[test]
        fn test_valid_instant_asset_lock_creates_action() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Use a shield amount much smaller than the asset lock value (1 Dash = 100_000_000 duffs)
            let shield_amount = 5000u64;
            let value_balance = -(shield_amount as i64);

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                vec![create_dummy_serialized_action()],
                0x03, // spends_enabled | outputs_enabled
                value_balance,
                [0u8; 32],      // dummy anchor
                vec![0u8; 100], // dummy proof bytes
                [0u8; 64],      // dummy binding signature
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Asset lock validation + ECDSA sig passed, but ZK proof failed.
            // This produces a PaidConsensusError (the asset lock is partially consumed as penalty).
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                    ..
                }]
            );
        }
    }

    // ==========================================
    // SIGNATURE VALIDATION TESTS (SignatureError)
    // ==========================================

    mod signature_validation {
        use super::*;

        #[test]
        fn test_wrong_ecdsa_signature_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Build transition with a completely zeroed (invalid) signature
            let transition = StateTransition::ShieldFromAssetLock(
                ShieldFromAssetLockTransition::V0(ShieldFromAssetLockTransitionV0 {
                    asset_lock_proof,
                    actions: vec![create_dummy_serialized_action()],
                    flags: 0x03,
                    value_balance: -5000,
                    anchor: [0u8; 32],
                    proof: vec![0u8; 100],
                    binding_signature: [0u8; 64],
                    user_fee_increase: 0,
                    signature: BinaryData::new(vec![0u8; 65]), // zeroed invalid signature
                }),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // ECDSA verification fails because the signature does not match the public key
            // derived from the asset lock output. This is caught after asset lock validation
            // but before ZK proof verification.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(SignatureError::BasicECDSAError(_))
                )]
            );
        }

        #[test]
        fn test_signature_from_different_key_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, _correct_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Generate a different key pair to sign with (wrong key)
            let wrong_private_key = PrivateKey::from_byte_array(
                &[42u8; 32], // arbitrary seed that is different
                Network::Testnet,
            )
            .unwrap();

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &wrong_private_key.inner.secret_bytes(), // Wrong key
                vec![create_dummy_serialized_action()],
                0x03,
                -5000,
                [0u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(SignatureError::BasicECDSAError(_))
                )]
            );
        }
    }

    // ==========================================
    // ZK PROOF VERIFICATION TESTS
    // ==========================================

    mod proof_verification {
        use super::*;

        /// End-to-end test: valid instant asset lock + valid Orchard bundle = success.
        ///
        /// This test builds a real Orchard bundle with ProvingKey (~30s on first run,
        /// cached via OnceLock for subsequent tests).
        #[test]
        fn test_valid_proof_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Build a valid Orchard bundle (shield = outputs only)
            let mut orchard_rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            let shield_value = 5000u64;
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(shield_value),
                    [0u8; 512],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            // No extra_sighash_data for shield_from_asset_lock (empty, like shield)
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // value_balance should be negative for shield (money going into pool)
            assert!(value_balance < 0);

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        /// Test that a structurally valid transition with dummy ZK proof data is rejected
        /// with a PaidConsensusError (penalty applied via PartiallyUseAssetLockAction).
        #[test]
        fn test_invalid_proof_returns_shielded_proof_error_with_penalty() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                vec![create_dummy_serialized_action()],
                0x03,
                -5000,
                [0u8; 32],
                vec![0u8; 100], // random proof data
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Passes structure + asset lock + ECDSA, fails at ZK proof verification.
            // The asset lock is partially consumed as penalty.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                    ..
                }]
            );
        }
    }

    // ==========================================
    // SECURITY AUDIT TESTS
    // ==========================================

    mod security_audit {
        use super::*;

        /// AUDIT FIX VERIFICATION: `value_balance = i64::MIN` no longer panics.
        ///
        /// Previously, `(-v0.value_balance) as u64` with i64::MIN caused an
        /// overflow panic. The transform_into_action code now uses `checked_neg()`
        /// which returns a consensus error instead of panicking.
        #[test]
        fn test_i64_min_value_balance_handled() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // i64::MIN is negative, so it passes the structure validation (value_balance < 0),
            // but checked_neg() on i64::MIN returns None, triggering the overflow guard in
            // transform_into_action. Since this error occurs after the asset lock proof
            // validation, it is a paid error (PartiallyUseAssetLockAction).
            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                vec![create_dummy_serialized_action()],
                0x03,
                i64::MIN, // -9223372036854775808 -- would overflow on negation
                [0u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            // Should return a consensus error, not panic
            let processing_result = process_transition(&platform, transition, platform_version);

            // The checked_neg overflow is caught in transform_into_action as an
            // InvalidShieldedProofError. Since it happens after asset lock validation,
            // we expect it to be reported as an UnpaidConsensusError (the overflow check
            // is done before consuming the asset lock value, so no penalty is applied).
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT FIX VERIFICATION: Mutated value_balance is rejected by BatchValidator.
        ///
        /// This test builds a valid Orchard bundle with value_balance = -5000, then
        /// mutates value_balance to -100000 in the transition. The binding signature
        /// no longer matches, causing proof verification to fail.
        #[test]
        fn test_valid_proof_with_mutated_value_balance_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Build a valid Orchard bundle
            let mut orchard_rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            assert!(value_balance < 0);
            let honest_shield_amount = (-value_balance) as u64;
            assert_eq!(honest_shield_amount, 5_000);

            // ATTACK: Mutate value_balance to claim shielding 100,000 instead of 5,000
            let mutated_value_balance = -100_000i64;

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                flags,
                mutated_value_balance, // MUTATED
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // BatchValidator now verifies binding signature. Mutated value_balance
            // changes the bundle commitment / sighash, causing verification to fail.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                    ..
                }]
            );
        }

        /// Verify that zeroed spend_auth_sig values (all zeros) in actions are rejected.
        #[test]
        fn test_zeroed_signatures_in_actions_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // Build a valid Orchard bundle first, then zero out the spend_auth_sig
            let mut orchard_rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (mut actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // Zero out all spend_auth_sig values
            for action in actions.iter_mut() {
                action.spend_auth_sig = [0u8; 64];
            }

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Zeroed spend_auth_sig causes BatchValidator to reject the bundle
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                    ..
                }]
            );
        }
    }
}
