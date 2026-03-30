#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, process_transition,
        serialize_authorized_bundle_with_flags, setup_platform,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
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
        Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags, FullViewingKey, NoteValue,
        Scope, SpendingKey,
    };
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::rngs::OsRng;
    use rand::SeedableRng;

    // ==========================================
    // Helper Functions (transition-specific)
    // ==========================================

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
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
    ) -> StateTransition {
        // Create unsigned transition to compute signable bytes
        let unsigned = ShieldFromAssetLockTransitionV0 {
            asset_lock_proof: asset_lock_proof.clone(),
            actions: actions.clone(),
            value_balance,
            anchor,
            proof: proof.clone(),
            binding_signature,
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
                value_balance,
                anchor,
                proof,
                binding_signature,
                signature: BinaryData::new(signature.to_vec()),
            },
        ))
    }

    /// Build a ShieldFromAssetLock StateTransition with dummy (invalid) signature.
    /// Used for structure validation tests where the error is caught before signature check.
    fn create_unsigned_shield_from_asset_lock_transition(
        asset_lock_proof: dpp::identity::state_transition::asset_lock_proof::AssetLockProof,
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
    ) -> StateTransition {
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof,
                actions,
                value_balance,
                anchor,
                proof,
                binding_signature,
                signature: BinaryData::new(vec![0u8; 65]), // dummy signature
            },
        ))
    }

    // (Orchard ProvingKey and serialize_authorized_bundle are now shared
    //  via test_helpers::get_proving_key / serialize_authorized_bundle_with_flags)

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
                1000,
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

        /// Tests validate_structure directly because 101 actions exceed the
        /// max_state_transition_size (20KB) before reaching the actions count check
        /// in the full pipeline.
        #[test]
        fn test_too_many_actions_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            // 101 actions exceeds max_shielded_transition_actions (100)
            let actions: Vec<SerializedAction> =
                (0..101).map(|_| create_dummy_serialized_action()).collect();

            let transition = ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: instant_asset_lock_proof_fixture(None, None),
                actions,
                value_balance: 1000,
                anchor: [42u8; 32],
                proof: vec![0u8; 100],
                binding_signature: [0u8; 64],
                signature: Default::default(),
            };

            let result = transition.validate_structure(platform_version);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ShieldedTooManyActionsError(_)
                )]
            );
        }

        #[test]
        fn test_zero_anchor_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(571);
            let (asset_lock_proof, _pk) = create_asset_lock_proof_with_key(&mut rng);

            let transition = create_unsigned_shield_from_asset_lock_transition(
                asset_lock_proof,
                vec![create_dummy_serialized_action()],
                1000,
                [0u8; 32], // Zero anchor — invalid
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedZeroAnchorError(_))
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
                0, // Zero -- invalid for shielding
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
                1000,
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

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                vec![create_dummy_serialized_action()],
                shield_amount,
                [42u8; 32], // non-zero anchor (won't match any stored anchor, but proof check is first)
                vec![0u8; 100], // dummy proof bytes
                [0u8; 64],  // dummy binding signature
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
                    value_balance: 5000,
                    anchor: [42u8; 32],
                    proof: vec![0u8; 100],
                    binding_signature: [0u8; 64],
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
                5000,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
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
            let mut builder = Builder::<DashMemo>::new(
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
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            // No extra_sighash_data for shield_from_asset_lock (empty, like shield)
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            // value_balance should be negative for shield (money going into pool)
            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
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
                5000,
                [42u8; 32],
                vec![0u8; 100], // random proof data
                [0u8; 64],
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

        #[test]
        fn test_value_balance_exceeding_i64_max_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(572);
            let (asset_lock_proof, _pk) = create_asset_lock_proof_with_key(&mut rng);

            let transition = create_unsigned_shield_from_asset_lock_transition(
                asset_lock_proof,
                vec![create_dummy_serialized_action()],
                i64::MAX as u64 + 1, // Exceeds i64::MAX
                [42u8; 32],
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
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            assert!(value_balance < 0);
            let honest_shield_amount = (-value_balance) as u64;
            assert_eq!(honest_shield_amount, 5_000);

            // ATTACK: Mutate value_balance to claim shielding 100,000 instead of 5,000
            let mutated_value_balance = 100_000u64;

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                mutated_value_balance, // MUTATED
                anchor_bytes,
                proof_bytes,
                binding_sig,
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
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (mut actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            // Zero out all spend_auth_sig values
            for action in actions.iter_mut() {
                action.spend_auth_sig = [0u8; 64];
            }

            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
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

    // ==========================================
    // RETURN PROOF TESTS
    // ==========================================

    mod return_proof {
        use super::*;
        use dpp::asset_lock::StoredAssetLockInfo;
        use dpp::block::block_info::BlockInfo;
        use dpp::serialization::PlatformSerializable;
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use drive::drive::Drive;
        use grovedb_commitment_tree::{
            Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags, FullViewingKey,
            NoteValue, Scope, SpendingKey,
        };
        use rand::rngs::OsRng;

        #[test]
        fn test_shield_from_asset_lock_prove_and_verify() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            // --- Build a valid Orchard bundle (shield = outputs only) ---
            let mut orchard_rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            let shield_value = 5_000u64;
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(shield_value),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;

            // --- Build and sign the transition ---
            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            // --- Serialize and process with manual transaction so we can commit before proving ---
            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("should serialize transition");

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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // Commit the transaction so prove_state_transition can read committed state
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // --- Generate proof ---
            let proof_result = platform
                .drive
                .prove_state_transition(&transition, None, platform_version)
                .expect("expected to generate proof for shield_from_asset_lock");

            let proof_bytes = proof_result
                .into_data()
                .expect("expected proof data, not an error");

            // --- Verify proof ---
            let (root_hash, proof_result) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof_bytes,
                &|_| Ok(None),
                platform_version,
            )
            .expect("expected to verify shield_from_asset_lock proof");

            assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

            // --- Assert result is VerifiedAssetLockConsumed ---
            let StateTransitionProofResult::VerifiedAssetLockConsumed(info) = proof_result else {
                panic!("expected VerifiedAssetLockConsumed, got {:?}", proof_result);
            };

            // ShieldFromAssetLock always fully consumes the asset lock
            // (remaining_credit_value is set to 0 in action-to-operations conversion).
            assert!(
                matches!(info, StoredAssetLockInfo::FullyConsumed),
                "expected FullyConsumed, got {:?}",
                info
            );
        }
    }
}
