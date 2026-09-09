#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, process_transition,
        serialize_authorized_bundle_with_flags, setup_platform,
    };
    use crate::platform_types::platform::PlatformRef;
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

    /// Like [`create_asset_lock_proof_with_key`], but funds the asset lock with a caller-chosen
    /// `amount` of duffs. Used by the implicit-fee-cap boundary tests to land the surplus exactly
    /// on the cap.
    fn create_asset_lock_proof_with_key_and_amount(
        rng: &mut StdRng,
        amount_duffs: u64,
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
            Some(amount_duffs),
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
            // Route the asset-lock surplus to a platform address. The fixture lock is ~1 Dash
            // while the shield amount is small, so the surplus exceeds the 0.2-Dash implicit-fee
            // cap; a surplus_output is required for the transition to be accepted (and it
            // exercises the surplus-to-address path end-to-end). Must match the signed literal
            // below so the ECDSA signature commits to the same destination.
            surplus_output: Some(dpp::address_funds::PlatformAddress::P2pkh([0x33; 20])),
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
                surplus_output: Some(dpp::address_funds::PlatformAddress::P2pkh([0x33; 20])),
                signature: BinaryData::new(signature.to_vec()),
            },
        ))
    }

    /// Build a ShieldFromAssetLock StateTransition with `surplus_output: None` and a valid ECDSA
    /// signature over the signable bytes. Used by the implicit-fee-cap boundary tests, which need
    /// the no-`surplus_output` path (where the surplus is implicitly donated to the fee pools and
    /// must not exceed `shielded_implicit_fee_cap`).
    fn create_signed_shield_from_asset_lock_transition_no_surplus(
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
            surplus_output: None,
            signature: Default::default(),
        };

        let state_transition: StateTransition = unsigned.into();
        let signable_bytes = state_transition
            .signable_bytes()
            .expect("should compute signable bytes");

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
                surplus_output: None,
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
                surplus_output: None,
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

        /// Tests `validate_structure` directly to exercise the
        /// `max_shielded_transition_actions` check in isolation, without
        /// depending on the full transition-processing pipeline.
        #[test]
        fn test_too_many_actions_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            // 17 actions exceeds max_shielded_transition_actions (16)
            let actions: Vec<SerializedAction> =
                (0..17).map(|_| create_dummy_serialized_action()).collect();

            let transition = ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: instant_asset_lock_proof_fixture(None, None),
                actions,
                value_balance: 1000,
                anchor: [42u8; 32],
                proof: vec![0u8; 100],
                binding_signature: [0u8; 64],
                surplus_output: None,
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

        #[test]
        fn check_tx_rejects_when_proof_verification_capacity_is_exhausted() {
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
                vec![0u8; 100],
                [0u8; 64],
            );
            let raw_transition = transition
                .serialize_to_bytes()
                .expect("transition should serialize");

            let platform_state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };
            let _occupied_capacity = platform
                .check_tx_proof_verifier
                .try_acquire(usize::MAX)
                .expect("test should occupy all proof-verification capacity");

            let result = platform.check_tx(
                &raw_transition,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                platform_version,
            );

            assert_matches!(
                result,
                Err(Error::Execution(
                    ExecutionError::CheckTxProofVerificationBusy
                ))
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
                    surplus_output: None,
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

            // CheckTx root-invariance guard (devnet paloma h788): `check_tx` asserts under
            // cfg(test) that it never mutates committed grovedb state, so running the
            // canonical valid fixture through it pins the invariant for this transition type.
            {
                use dpp::serialization::PlatformSerializable;
                let guard_serialized_transition = transition
                    .serialize_to_bytes()
                    .expect("expected to serialize transition for the check_tx guard");
                crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
                    &platform,
                    &guard_serialized_transition,
                    "shield from asset lock",
                );
            }

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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
            .expect("expected to verify shield_from_asset_lock proof");

            assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

            // --- Assert result is VerifiedAssetLockConsumedWithAddressInfos ---
            // The transition built by `create_signed_shield_from_asset_lock_transition` sets
            // `surplus_output: Some(P2pkh([0x33; 20]))` (the fixture lock funds a surplus well above
            // the implicit-fee cap, so an explicit surplus output is required). The verifier
            // therefore proves BOTH the consumed asset-lock outpoint AND the surplus-output address
            // balance, returning the combined variant.
            let StateTransitionProofResult::VerifiedAssetLockConsumedWithAddressInfos(
                info,
                balances,
            ) = proof_result
            else {
                panic!(
                    "expected VerifiedAssetLockConsumedWithAddressInfos, got {:?}",
                    proof_result
                );
            };

            // ShieldFromAssetLock always fully consumes the asset lock
            // (remaining_credit_value is set to 0 in action-to-operations conversion).
            assert!(
                matches!(info, StoredAssetLockInfo::FullyConsumed),
                "expected FullyConsumed, got {:?}",
                info
            );

            // The surplus credit must be provably present at the signed surplus-output address.
            let surplus_address = dpp::address_funds::PlatformAddress::P2pkh([0x33; 20]);
            let surplus_balance = balances.get(&surplus_address).unwrap_or_else(|| {
                panic!(
                    "surplus address missing from proven balances: {:?}",
                    balances
                )
            });
            let (_nonce, credits) = surplus_balance
                .unwrap_or_else(|| panic!("surplus address has no credited balance in the proof"));
            assert!(
                credits > 0,
                "expected a positive surplus credit at the surplus-output address, got {}",
                credits
            );
        }

        /// NEGATIVE test for the strict-merged tightening: a proof that carries EXTRA data
        /// beyond `{outpoint, surplus-address}` MUST be rejected by the production verifier.
        ///
        /// The production verify path rebuilds the merged `{outpoint, single surplus address}`
        /// query and verifies it with the STRICT `verify_query_with_absence_proof`, whose
        /// succinctness check rejects any proof that descends into a subtree (a lower layer) the
        /// query did not require. Here we commit the same surplus state, then have the (honest, but
        /// over-broad) prover generate a SUPERSET proof for `{outpoint, surplus-address, + the
        /// genesis-populated Pools subtree}` and verify it against the production single-address
        /// merged query. The strict verifier must reject it because the proof carries an extra
        /// root-level lower layer (`Pools`) the production query never touches.
        ///
        /// For contrast we also confirm the SUBSET verifier (the looser primitive this change
        /// replaced) would have ACCEPTED the same padded proof — demonstrating exactly the hole
        /// the strict merged verification closes.
        ///
        /// (Note: padding with an extra *leaf key* in an already-descended subtree — e.g. a second
        /// address under the same clear-address pool — is NOT what the succinctness check guards
        /// against; such a key is either filtered by the per-layer merk query or surfaced as a
        /// proven absence, and is rejected/ignored regardless of strict-vs-subset. The tightening
        /// specifically removes the tolerance for extra *subtree branches*.)
        #[test]
        fn test_shield_from_asset_lock_padded_proof_is_rejected_by_strict_verify() {
            use drive::drive::RootTree;
            use drive::grovedb::{GroveDb, PathQuery, Query, SizedQuery};

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

            // This helper sets `surplus_output: Some(P2pkh([0x33; 20]))`.
            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let outpoint = {
                use dpp::identity::state_transition::OptionallyAssetLockProved;
                let op = transition
                    .optional_asset_lock_proof()
                    .expect("transition must carry an asset lock proof")
                    .out_point()
                    .expect("transition must carry an outpoint");
                let bytes: [u8; 36] = op.into();
                bytes
            };

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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // --- Reconstruct the PRODUCTION merged query exactly as the verifier does ---
            // {outpoint} ∪ {real surplus address [0x33; 20]}, both with cleared limits, then a
            // limit that can never truncate the legitimate result set.
            let real_surplus = dpp::address_funds::PlatformAddress::P2pkh([0x33; 20]);

            let mut outpoint_query = Query::new();
            outpoint_query.insert_key(outpoint.to_vec());
            let mut outpoint_pq = PathQuery::new(
                vec![vec![RootTree::SpentAssetLockTransactions as u8]],
                SizedQuery::new(outpoint_query, None, None),
            );
            outpoint_pq.query.limit = None;

            let mut real_address_pq =
                Drive::balances_for_clear_addresses_query(std::iter::once(&real_surplus));
            real_address_pq.query.limit = None;

            let mut production_pq = PathQuery::merge(
                vec![&outpoint_pq, &real_address_pq],
                &platform_version.drive.grove_version,
            )
            .expect("merge production query");
            production_pq.query.limit = Some(u16::MAX);

            // Sanity: an HONEST proof for the production query verifies strictly (liveness).
            let honest_proof = platform
                .drive
                .grove_get_proved_path_query(
                    &production_pq,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("honest production proof");
            GroveDb::verify_query_with_absence_proof(
                &honest_proof,
                &production_pq,
                &platform_version.drive.grove_version,
            )
            .expect("strict verify of honest production proof must succeed");

            // --- Build a SUPERSET (padded) proof: {outpoint, real address, + an extra subtree} ---
            // The succinctness check the strict verifier runs rejects a proof that descends into a
            // subtree (a lower layer) the query did not require. So we pad by ALSO descending the
            // genesis-populated `Pools` root subtree, which the production query never touches. The
            // padded proof therefore carries an extra root-level lower layer.
            let mut pools_top = Query::new();
            pools_top.insert_key(vec![RootTree::Pools as u8]);
            pools_top.set_subquery(Query::new_range_full());
            let pools_pq = PathQuery::new(vec![], SizedQuery::new(pools_top, None, None));

            let mut superset_pq = PathQuery::merge(
                vec![&outpoint_pq, &real_address_pq, &pools_pq],
                &platform_version.drive.grove_version,
            )
            .expect("merge superset query");
            superset_pq.query.limit = Some(u16::MAX);

            let padded_proof = platform
                .drive
                .grove_get_proved_path_query(
                    &superset_pq,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("padded superset proof");

            // The STRICT verifier (production behavior) MUST reject the padded proof: it carries
            // an extra address layer the production query did not request.
            let strict_result = GroveDb::verify_query_with_absence_proof(
                &padded_proof,
                &production_pq,
                &platform_version.drive.grove_version,
            );
            assert!(
                strict_result.is_err(),
                "strict verifier must reject a proof padded with an extra address layer, got {:?}",
                strict_result
            );

            // Contrast: the SUBSET verifier (the looser primitive this change replaced) tolerates
            // the extra layer and ACCEPTS the same padded proof — the exact hole now closed.
            let subset_result = GroveDb::verify_subset_query_with_absence_proof(
                &padded_proof,
                &production_pq,
                &platform_version.drive.grove_version,
            );
            assert!(
                subset_result.is_ok(),
                "subset verifier was expected to tolerate the padded proof, got {:?}",
                subset_result
            );
        }

        /// Round-trip prove/verify for the `surplus_output: None` path. The surplus is implicitly
        /// donated to the fee pools (sized to land exactly on the cap so it is accepted), so the
        /// verifier proves ONLY the consumed asset-lock outpoint and returns the plain
        /// `VerifiedAssetLockConsumed` variant — byte-for-byte the pre-change behavior.
        #[test]
        fn test_shield_from_asset_lock_no_surplus_prove_and_verify() {
            use dpp::balances::credits::CREDITS_PER_DUFF;
            use dpp::shielded::compute_minimum_shielded_fee;
            use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition as SfalTransition;
            use dpp::state_transition::StateTransitionEstimatedFeeValidation;

            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Build a valid bundle and size the asset-lock funding so the surplus lands exactly on
            // the implicit-fee cap (the largest surplus the no-`surplus_output` path accepts).
            let cap = platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .shielded_implicit_fee_cap;

            // --- Build a valid single-output Orchard bundle ---
            let mut orchard_rng = OsRng;
            let pk = get_proving_key();
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            // A round base shield value, corrected below so the required lock value is a whole
            // number of duffs.
            const SHIELD_VALUE_BASE: u64 = 1_000_000;

            let mut build_bundle = |shield_value: u64| {
                let mut builder = Builder::<DashMemo>::new(
                    BundleType::Transactional {
                        flags: OrchardFlags::SPENDS_DISABLED,
                        bundle_required: false,
                    },
                    Anchor::empty_tree(),
                );
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
                serialize_authorized_bundle_with_flags(&bundle)
            };

            // Probe to learn the (value-independent) action count, then the pool fee.
            let (probe_actions, _, _, _, _, _) = build_bundle(SHIELD_VALUE_BASE);
            let num_actions = probe_actions.len();
            let shielded_fee = compute_minimum_shielded_fee(num_actions, platform_version)
                .expect("should compute minimum shielded fee");
            let albc = SfalTransition::V0(ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: instant_asset_lock_proof_fixture(None, None),
                actions: vec![create_dummy_serialized_action()],
                value_balance: 1,
                anchor: [1u8; 32],
                proof: vec![0u8; 1],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: Default::default(),
            })
            .calculate_min_required_fee(platform_version)
            .expect("should compute asset-lock base cost");
            let pool_fee = shielded_fee.checked_add(albc).expect("pool fee overflow");

            // Choose `shield_value` so `cap + shield_value + pool_fee` is a whole number of duffs.
            let correction = (CREDITS_PER_DUFF
                - ((cap + SHIELD_VALUE_BASE + pool_fee) % CREDITS_PER_DUFF))
                % CREDITS_PER_DUFF;
            let shield_value = SHIELD_VALUE_BASE + correction;

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_bundle(shield_value);
            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;

            let lock_credits = cap + shield_amount + pool_fee;
            assert_eq!(lock_credits % CREDITS_PER_DUFF, 0);
            let lock_amount_duffs = lock_credits / CREDITS_PER_DUFF;

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) =
                create_asset_lock_proof_with_key_and_amount(&mut rng, lock_amount_duffs);

            let transition = create_signed_shield_from_asset_lock_transition_no_surplus(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let proof_result = platform
                .drive
                .prove_state_transition(&transition, None, platform_version)
                .expect("expected to generate proof for shield_from_asset_lock");

            let proof_bytes = proof_result
                .into_data()
                .expect("expected proof data, not an error");

            let (root_hash, proof_result) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof_bytes,
                &|_| Ok(None),
                platform_version,
            )
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
            .expect("expected to verify shield_from_asset_lock proof");

            assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

            // --- No surplus output → plain VerifiedAssetLockConsumed (unchanged behavior) ---
            let StateTransitionProofResult::VerifiedAssetLockConsumed(info) = proof_result else {
                panic!("expected VerifiedAssetLockConsumed, got {:?}", proof_result);
            };

            assert!(
                matches!(info, StoredAssetLockInfo::FullyConsumed),
                "expected FullyConsumed, got {:?}",
                info
            );
        }
    }

    // ==========================================
    // IMPLICIT FEE CAP BOUNDARY TESTS
    // ==========================================

    /// Boundary tests for the pre-action implicit-fee-cap gate in `transform_into_action`
    /// (Step 7b). When no `surplus_output` is set, the asset-lock surplus is implicitly donated
    /// to the fee pools, but only up to `shielded_implicit_fee_cap`; above the cap the transition
    /// is rejected with `ShieldedImplicitFeeCapExceededError`, forcing the client to set an
    /// explicit `surplus_output`.
    ///
    /// The cap check runs BEFORE ZK proof verification (so an over-cap transition is rejected
    /// cheaply, without paying ~100ms of Halo2 verification). The accepted-case test still needs a
    /// valid proof to succeed, so these tests build a real (valid) Orchard bundle and size the
    /// asset-lock funding so the surplus lands exactly on the boundary:
    ///
    ///   surplus = lock_value_credits − shield_amount − pool_fee
    ///
    /// `pool_fee = compute_minimum_shielded_fee(num_actions) + albc` is independent of the shielded
    /// amount, so we compute it dynamically from the platform version (rather than hardcoding the
    /// fee constants) and then choose `shield_value` to place the surplus precisely at `cap` or
    /// `cap + 1`. The asset-lock funding (`amount_duffs`) is fixed at a round value comfortably
    /// above the cap.
    mod implicit_fee_cap {
        use super::*;
        use dpp::balances::credits::CREDITS_PER_DUFF;
        use dpp::shielded::compute_minimum_shielded_fee;
        use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition as SfalTransition;
        use dpp::state_transition::StateTransitionEstimatedFeeValidation;

        /// Compute the flat `pool_fee` (in credits) that `transform_into_action` charges for a
        /// `ShieldFromAssetLock` with `num_actions` Orchard actions, mirroring the production
        /// computation exactly: `compute_minimum_shielded_fee(num_actions) + albc`. The fee depends
        /// only on the action count (not the shielded amount), so once we know the action count of
        /// the built bundle we can solve for the funding that lands the surplus on the cap boundary.
        fn pool_fee_for_actions(num_actions: usize, platform_version: &PlatformVersion) -> u64 {
            let shielded_fee = compute_minimum_shielded_fee(num_actions, platform_version)
                .expect("should compute minimum shielded fee");

            // `albc` is the asset-lock base cost, exposed via the estimated-fee validation trait.
            // Build a throwaway transition just to invoke `calculate_min_required_fee` (the value
            // depends only on the platform version, not on the transition contents).
            let dummy = ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: instant_asset_lock_proof_fixture(None, None),
                actions: vec![create_dummy_serialized_action()],
                value_balance: 1,
                anchor: [1u8; 32],
                proof: vec![0u8; 1],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: Default::default(),
            };
            let albc = SfalTransition::V0(dummy)
                .calculate_min_required_fee(platform_version)
                .expect("should compute asset-lock base cost");

            shielded_fee
                .checked_add(albc)
                .expect("pool fee should not overflow")
        }

        /// Build a valid single-output Orchard bundle that shields exactly `shield_value` credits,
        /// returning the serialized pieces consumed by the transition builder plus the bundle's
        /// action count.
        ///
        /// (Real proving — uses the cached `get_proving_key`, so each build is a few seconds.)
        fn build_valid_shield_bundle(
            shield_value: u64,
        ) -> (
            Vec<SerializedAction>,
            u64,
            usize,
            [u8; 32],
            Vec<u8>,
            [u8; 64],
        ) {
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
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(shield_value),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut orchard_rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            // No extra_sighash_data for shield_from_asset_lock (empty, like shield).
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut orchard_rng).unwrap();
            let bundle = proven.apply_signatures(orchard_rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            assert!(value_balance < 0, "shield value_balance must be negative");
            let shield_amount = (-value_balance) as u64;
            assert_eq!(
                shield_amount, shield_value,
                "the bundle must shield exactly the requested value"
            );
            let num_actions = actions.len();

            (
                actions,
                shield_amount,
                num_actions,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            )
        }

        /// Pieces needed to drive a boundary case end-to-end: a valid bundle whose surplus lands
        /// exactly on `target_surplus`, plus the asset-lock funding (in duffs) that achieves it.
        struct BoundaryCase {
            actions: Vec<SerializedAction>,
            shield_amount: u64,
            anchor: [u8; 32],
            proof: Vec<u8>,
            binding_signature: [u8; 64],
            lock_amount_duffs: u64,
        }

        /// Build a valid `ShieldFromAssetLock` bundle and matching asset-lock funding such that the
        /// transform's surplus (`lock_credits − shield_amount − pool_fee`) equals `target_surplus`
        /// exactly.
        ///
        /// The asset-lock value is quantised to whole duffs (× `CREDITS_PER_DUFF`), so the surplus
        /// can only land on a multiple-of-`CREDITS_PER_DUFF` lattice unless we tune the shielded
        /// amount. We therefore:
        ///   1. Build a probe bundle to learn the (value-independent) action count → `pool_fee`.
        ///   2. Pick `shield_value ≡ −(target_surplus + pool_fee) (mod CREDITS_PER_DUFF)` so that
        ///      `target_surplus + shield_value + pool_fee` is an exact number of duffs.
        ///   3. Rebuild the bundle with that `shield_value` (the action count is independent of the
        ///      value, so `pool_fee` is unchanged) and derive the funding `amount_duffs`.
        fn build_boundary_case(
            target_surplus: u64,
            platform_version: &PlatformVersion,
        ) -> BoundaryCase {
            // A round base shield value, comfortably positive after the modular correction below.
            const SHIELD_VALUE_BASE: u64 = 1_000_000;

            // Step 1: probe to learn the action count and thus the pool fee.
            let (_, _, probe_num_actions, _, _, _) = build_valid_shield_bundle(SHIELD_VALUE_BASE);
            let pool_fee = pool_fee_for_actions(probe_num_actions, platform_version);

            // Step 2: choose `shield_value` so the required lock value is a whole number of duffs.
            let correction = (CREDITS_PER_DUFF
                - ((target_surplus + SHIELD_VALUE_BASE + pool_fee) % CREDITS_PER_DUFF))
                % CREDITS_PER_DUFF;
            let shield_value = SHIELD_VALUE_BASE + correction;

            // Step 3: build the real bundle and derive the funding.
            let (actions, shield_amount, num_actions, anchor, proof, binding_signature) =
                build_valid_shield_bundle(shield_value);
            assert_eq!(
                num_actions, probe_num_actions,
                "action count must not depend on the shielded value"
            );

            let lock_credits = target_surplus + shield_amount + pool_fee;
            assert_eq!(
                lock_credits % CREDITS_PER_DUFF,
                0,
                "lock value must be a whole number of duffs"
            );
            let lock_amount_duffs = lock_credits / CREDITS_PER_DUFF;

            // The surplus the transform will compute (`lock_credits − shield_amount − pool_fee`)
            // lands exactly on the target by construction.
            assert_eq!(
                lock_credits - shield_amount - pool_fee,
                target_surplus,
                "surplus must land exactly on the target"
            );

            BoundaryCase {
                actions,
                shield_amount,
                anchor,
                proof,
                binding_signature,
                lock_amount_duffs,
            }
        }

        fn implicit_fee_cap(platform_version: &PlatformVersion) -> u64 {
            platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .shielded_implicit_fee_cap
        }

        /// `surplus == cap` with `surplus_output == None` is ACCEPTED (the surplus is implicitly
        /// donated to the fee pools, exactly at the cap).
        #[test]
        fn test_surplus_equal_to_cap_is_accepted() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let cap = implicit_fee_cap(platform_version);
            let case = build_boundary_case(cap, platform_version);

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) =
                create_asset_lock_proof_with_key_and_amount(&mut rng, case.lock_amount_duffs);

            let transition = create_signed_shield_from_asset_lock_transition_no_surplus(
                asset_lock_proof,
                &asset_lock_pk,
                case.actions,
                case.shield_amount,
                case.anchor,
                case.proof,
                case.binding_signature,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        /// `surplus == cap + 1` with `surplus_output == None` is REJECTED with
        /// `ShieldedImplicitFeeCapExceededError`.
        #[test]
        fn test_surplus_one_over_cap_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let cap = implicit_fee_cap(platform_version);
            let case = build_boundary_case(cap + 1, platform_version);

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) =
                create_asset_lock_proof_with_key_and_amount(&mut rng, case.lock_amount_duffs);

            let transition = create_signed_shield_from_asset_lock_transition_no_surplus(
                asset_lock_proof,
                &asset_lock_pk,
                case.actions,
                case.shield_amount,
                case.anchor,
                case.proof,
                case.binding_signature,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // The cap check now runs at Step 7b, BEFORE proof verification, so the over-cap
            // transition is rejected there as an UnpaidConsensusError (the cap check precedes both
            // the proof and action construction, so no penalty is applied to the asset lock — the
            // proof is never even verified).
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedImplicitFeeCapExceededError(_))
                )]
            );
        }
    }

    // ==========================================
    // CREDIT CONSERVATION TESTS (block-level sum-tree balance)
    // ==========================================

    /// Block-level credit-conservation regression tests for the `ShieldFromAssetLock` SURPLUS path.
    ///
    /// The other surplus tests in this file drive the transition through `process_transition`, which
    /// runs only `process_raw_state_transitions` — they never reach
    /// `process_block_fees_and_validate_sum_trees`, the end-of-block credit-conservation check whose
    /// `CorruptedCreditsNotBalanced` failure is the chain-halt this whole PR exists to prevent. These
    /// tests instead run the FULL block pipeline via `process_state_transitions`, which calls
    /// `process_block_fees_and_validate_sum_trees`; with `verify_sum_trees` enabled (the default),
    /// the helper's `.expect("expected to process block fees")` panics if the consumed asset-lock
    /// value, the shielded-pool credit, the surplus-address credit, and the fee-pool credit do not
    /// sum-tree-balance — so a passing test is a genuine block-level proof of conservation.
    ///
    /// A `ShieldFromAssetLock` is NOT credit-neutral the way a transparent `Shield` is: it INJECTS
    /// new system credits (the consumed asset-lock value) and distributes them across the shielded
    /// pool (`shield_amount`), the surplus-output address (`surplus_amount`, when set), and the fee
    /// pools (`pool_fee`, plus any surplus folded in when no `surplus_output` is set). The invariant
    /// is therefore: `total_credits_in_platform` rises by exactly the consumed lock value, which must
    /// equal `shield_amount + surplus_amount + pool_fee`, and the sum trees stay balanced.
    mod credit_conservation {
        use super::*;
        use crate::execution::validation::state_transition::tests::process_state_transitions;
        use dpp::balances::credits::CREDITS_PER_DUFF;
        use dpp::block::block_info::BlockInfo;
        use dpp::shielded::compute_minimum_shielded_fee;
        use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition as SfalTransition;
        use dpp::state_transition::StateTransitionEstimatedFeeValidation;

        /// The default `instant_asset_lock_proof_fixture` funds a single 1-DASH credit output.
        const DEFAULT_FIXTURE_LOCK_DUFFS: u64 = 100_000_000;

        /// Compute the flat `pool_fee` (`compute_minimum_shielded_fee(num_actions) + albc`) exactly
        /// as `transform_into_action` does, so the conservation assertions use the production fee.
        fn pool_fee_for_actions(num_actions: usize, platform_version: &PlatformVersion) -> u64 {
            let shielded_fee = compute_minimum_shielded_fee(num_actions, platform_version)
                .expect("should compute minimum shielded fee");
            let albc = SfalTransition::V0(ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: instant_asset_lock_proof_fixture(None, None),
                actions: vec![create_dummy_serialized_action()],
                value_balance: 1,
                anchor: [1u8; 32],
                proof: vec![0u8; 1],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: Default::default(),
            })
            .calculate_min_required_fee(platform_version)
            .expect("should compute asset-lock base cost");
            shielded_fee.checked_add(albc).expect("pool fee overflow")
        }

        /// Build a real, valid single-output Orchard shield bundle of `shield_value` credits.
        fn build_valid_shield_bundle(
            shield_value: u64,
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut orchard_rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                Anchor::empty_tree(),
            );
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
            assert!(value_balance < 0, "shield value_balance must be negative");
            let shield_amount = (-value_balance) as u64;
            assert_eq!(
                shield_amount, shield_value,
                "the bundle must shield exactly the requested value"
            );
            (
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            )
        }

        /// SURPLUS-TO-ADDRESS path: a `ShieldFromAssetLock` with `surplus_output: Some(addr)` and a
        /// non-zero surplus must (a) keep all credits sum-tree balanced through the full block
        /// pipeline, (b) raise the shielded pool by exactly `shield_amount`, (c) raise the surplus
        /// address balance by exactly `surplus_amount`, and (d) raise `total_credits_in_platform` by
        /// exactly the consumed asset-lock value (`= shield_amount + surplus_amount + pool_fee`).
        #[tokio::test]
        async fn test_shield_from_asset_lock_with_surplus_output_conserves_credits() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Platform starts balanced; this is the production invariant the block-end check enforces.
            let credits_before = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits before");
            assert!(
                credits_before
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must be balanced before the shield_from_asset_lock: {credits_before}"
            );

            // Build a real valid bundle (shield 5000) over the default 1-DASH fixture lock. The
            // surplus (≈1 DASH minus the small shield + fee) far exceeds the implicit-fee cap, so an
            // explicit `surplus_output` is required — exactly what this builder sets.
            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) = create_asset_lock_proof_with_key(&mut rng);

            let shield_value = 5_000u64;
            let (actions, shield_amount, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shield_bundle(shield_value);

            // Production fee + the consumed lock value, so we can pin the surplus exactly.
            let pool_fee = pool_fee_for_actions(actions.len(), platform_version);
            let consumed = DEFAULT_FIXTURE_LOCK_DUFFS * CREDITS_PER_DUFF;
            let surplus_amount = consumed
                .checked_sub(shield_amount)
                .and_then(|v| v.checked_sub(pool_fee))
                .expect("surplus must be non-negative");
            assert!(
                surplus_amount > 0,
                "this test must exercise a non-zero surplus routed to the address"
            );

            let transition = create_signed_shield_from_asset_lock_transition(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            // --- Run the FULL block pipeline (execute + distribute fees + validate sum trees).
            //     `process_state_transitions` calls `process_block_fees_and_validate_sum_trees`; with
            //     `verify_sum_trees` enabled (the default) its `.expect(...)` panics here on a
            //     `CorruptedCreditsNotBalanced` — the block-level conservation assertion. ---
            let platform_state = platform.state.load();
            let (_fee_results, _processed_block_fees) = process_state_transitions(
                &platform,
                &[transition],
                BlockInfo::default(),
                &platform_state,
            );

            let credits_after = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits after");

            // (a) Credits remain sum-tree balanced (the invariant whose failure halts the chain).
            assert!(
                credits_after
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must remain balanced after shield_from_asset_lock with surplus output: \
                 {credits_after}"
            );

            // (b) The shielded pool rose by exactly `shield_amount`.
            assert_eq!(
                credits_after.total_in_shielded_balances
                    - credits_before.total_in_shielded_balances,
                shield_amount as i64,
                "shielded pool must gain exactly the shield amount"
            );

            // (c) The surplus-output address balance rose by exactly `surplus_amount`. The fresh
            //     platform has no other addresses, so the address-tree delta equals the surplus
            //     credited to the signed surplus-output address (`P2pkh([0x33; 20])`).
            assert_eq!(
                credits_after.total_in_addresses - credits_before.total_in_addresses,
                surplus_amount as i64,
                "the surplus_output address must gain exactly the surplus amount"
            );

            // (d) `total_credits_in_platform` rose by exactly the consumed asset-lock value, which
            //     equals `shield_amount + surplus_amount + pool_fee`.
            assert_eq!(
                credits_after.total_credits_in_platform - credits_before.total_credits_in_platform,
                consumed,
                "platform total must rise by exactly the consumed asset-lock value"
            );
            assert_eq!(
                consumed,
                shield_amount + surplus_amount + pool_fee,
                "consumed lock value must split exactly into shield + surplus + pool fee"
            );
        }

        /// NO-SURPLUS-OUTPUT path: a `ShieldFromAssetLock` with `surplus_output: None` folds the
        /// surplus into the fee pools (allowed only up to the implicit-fee cap). It must keep credits
        /// sum-tree balanced through the full block pipeline, raise the shielded pool by exactly
        /// `shield_amount`, create NO address balance, and raise `total_credits_in_platform` by
        /// exactly the consumed asset-lock value.
        #[tokio::test]
        async fn test_shield_from_asset_lock_without_surplus_output_conserves_credits() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let credits_before = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits before");
            assert!(
                credits_before
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must be balanced before the shield_from_asset_lock: {credits_before}"
            );

            // Size the lock so the surplus lands exactly on the implicit-fee cap — the largest
            // surplus the no-`surplus_output` path accepts. Build the bundle, then derive the funding.
            let cap = platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .shielded_implicit_fee_cap;

            // Probe to learn the action count → pool_fee, then choose `shield_value` so the required
            // lock value is a whole number of duffs (the fixture quantises funding to duffs).
            const SHIELD_VALUE_BASE: u64 = 1_000_000;
            let (probe_actions, _, _, _, _) = build_valid_shield_bundle(SHIELD_VALUE_BASE);
            let pool_fee = pool_fee_for_actions(probe_actions.len(), platform_version);
            let correction = (CREDITS_PER_DUFF
                - ((cap + SHIELD_VALUE_BASE + pool_fee) % CREDITS_PER_DUFF))
                % CREDITS_PER_DUFF;
            let shield_value = SHIELD_VALUE_BASE + correction;

            let (actions, shield_amount, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shield_bundle(shield_value);

            let consumed = cap + shield_amount + pool_fee;
            assert_eq!(
                consumed % CREDITS_PER_DUFF,
                0,
                "lock value must be a whole number of duffs"
            );
            let lock_amount_duffs = consumed / CREDITS_PER_DUFF;
            // The surplus the transform computes lands exactly on the cap by construction.
            assert_eq!(consumed - shield_amount - pool_fee, cap);

            let mut rng = StdRng::seed_from_u64(567);
            let (asset_lock_proof, asset_lock_pk) =
                create_asset_lock_proof_with_key_and_amount(&mut rng, lock_amount_duffs);

            let transition = create_signed_shield_from_asset_lock_transition_no_surplus(
                asset_lock_proof,
                &asset_lock_pk,
                actions,
                shield_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let platform_state = platform.state.load();
            let (_fee_results, _processed_block_fees) = process_state_transitions(
                &platform,
                &[transition],
                BlockInfo::default(),
                &platform_state,
            );

            let credits_after = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits after");

            // Credits remain sum-tree balanced.
            assert!(
                credits_after
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must remain balanced after shield_from_asset_lock without surplus output: \
                 {credits_after}"
            );

            // The shielded pool rose by exactly `shield_amount`.
            assert_eq!(
                credits_after.total_in_shielded_balances
                    - credits_before.total_in_shielded_balances,
                shield_amount as i64,
                "shielded pool must gain exactly the shield amount"
            );

            // NO address balance was created for the (absent) surplus — it folded into the fee pools.
            assert_eq!(
                credits_after.total_in_addresses - credits_before.total_in_addresses,
                0,
                "no surplus_output means no address balance is created"
            );

            // `total_credits_in_platform` rose by exactly the consumed asset-lock value.
            assert_eq!(
                credits_after.total_credits_in_platform - credits_before.total_credits_in_platform,
                consumed,
                "platform total must rise by exactly the consumed asset-lock value"
            );
        }
    }
}
