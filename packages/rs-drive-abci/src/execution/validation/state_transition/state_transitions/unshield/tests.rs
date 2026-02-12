#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, insert_anchor_into_state, insert_dummy_encrypted_notes,
        insert_nullifier_into_state, process_transition, set_pool_total_balance, setup_platform,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
    use dpp::address_funds::PlatformAddress;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
    use dpp::state_transition::unshield_transition::UnshieldTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;

    // ==========================================
    // Helper Functions (transition-specific)
    // ==========================================

    /// Create a dummy PlatformAddress for the output.
    fn create_output_address() -> PlatformAddress {
        let mut hash = [0u8; 20];
        hash[0] = 42;
        hash[19] = 42;
        PlatformAddress::P2pkh(hash)
    }

    /// Builds an `UnshieldTransition` state transition.
    /// No signing needed since unshield transitions have no witnesses.
    fn create_unshield_transition(
        output_address: PlatformAddress,
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: u16,
    ) -> StateTransition {
        StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address,
            amount,
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            user_fee_increase,
        }))
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) unshield
    /// transition. Has a non-zero anchor, valid field sizes, positive amount and value_balance.
    fn create_default_unshield_transition() -> StateTransition {
        create_unshield_transition(
            create_output_address(),
            1000, // amount being unshielded
            vec![create_dummy_serialized_action()],
            0x03,           // spends_enabled | outputs_enabled
            1000,           // value_balance = amount (no fee for simplicity)
            [42u8; 32],     // non-zero anchor
            vec![0u8; 100], // dummy proof bytes
            [0u8; 64],      // dummy binding signature
            0,
        )
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

            let transition = create_unshield_transition(
                create_output_address(),
                1000,
                vec![], // Empty actions — invalid
                0x03,
                1000,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
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
        fn test_zero_amount_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                0, // Zero amount — invalid
                vec![create_dummy_serialized_action()],
                0x03,
                1000,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::UnshieldAmountZeroError(_))
                )]
            );
        }

        #[test]
        fn test_non_positive_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                0, // Zero value_balance — invalid (must be positive)
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
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
        fn test_negative_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000, // Negative value_balance — invalid
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
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
        fn test_value_balance_less_than_amount_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                2000, // amount = 2000
                vec![create_dummy_serialized_action()],
                0x03,
                1000, // value_balance = 1000 < amount — invalid
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::UnshieldValueBalanceBelowAmountError(_))
                )]
            );
        }

        #[test]
        fn test_empty_proof_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                1000,
                [42u8; 32],
                vec![], // Empty proof — invalid
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedEmptyProofError(_))
                )]
            );
        }

        #[test]
        fn test_zero_anchor_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                1000,
                [0u8; 32], // All zeros — invalid
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedZeroAnchorError(_))
                )]
            );
        }
    }

    // ==========================================
    // ANCHOR VALIDATION TESTS (StateError)
    // ==========================================

    mod anchor_validation {
        use super::*;

        #[test]
        fn test_insufficient_pool_notes_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Non-zero anchor that exists in state, but no encrypted notes in pool
            let anchor = [42u8; 32];
            insert_anchor_into_state(&platform, &anchor);

            let transition = create_default_unshield_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InsufficientPoolNotesError(_))
                )]
            );
        }

        #[test]
        fn test_invalid_anchor_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Non-zero anchor that doesn't exist in state
            let transition = create_default_unshield_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidAnchorError(_))
                )]
            );
        }
    }

    // ==========================================
    // NULLIFIER DOUBLE-SPEND TESTS (StateError)
    // ==========================================

    mod nullifier_validation {
        use super::*;

        #[test]
        fn test_nullifier_already_spent_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];
            let nullifier = [1u8; 32]; // Same as create_dummy_serialized_action().nullifier

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Insert the nullifier so it appears already spent
            insert_nullifier_into_state(&platform, &nullifier);

            let transition = create_default_unshield_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::NullifierAlreadySpentError(_))
                )]
            );
        }
    }

    // ==========================================
    // ZK PROOF VERIFICATION TESTS (InvalidShieldedProofError)
    // ==========================================

    mod proof_verification {
        use super::*;
        use grovedb_commitment_tree::{
            new_memory_store, Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            CommitmentTree, ExtractedNoteCommitment, FullViewingKey, MerklePath, Note, NoteValue,
            Position, ProvingKey, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
        };
        use orchard::note::RandomSeed;
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

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

        #[test]
        fn test_invalid_proof_returns_shielded_proof_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // This transition is structurally valid and has a valid anchor,
            // but has random ZK proof data. It should pass structure validation
            // and anchor validation but fail at proof verification.
            let transition = create_default_unshield_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        #[test]
        fn test_valid_unshield_proof_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);
            let mut rng = OsRng;
            let pk = get_proving_key();

            // --- Create keys ---
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            // --- Create a spendable note with value 10,000 ---
            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(10_000), rho, rseed).unwrap();

            // --- Build commitment tree and get anchor + merkle path ---
            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = CommitmentTree::new(new_memory_store(), 100);
            tree.append(cmx, Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.orchard_witness(Position::from(0u64)).unwrap().unwrap();

            // --- Build bundle: spend 10,000 → output 5,000 (value_balance = 5,000) ---
            let mut builder = Builder::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            // Compute platform sighash binding transparent fields (output_address, amount)
            let output_address = create_output_address();
            let amount = 5_000u64; // = value_balance (no fee difference)
            let mut extra_sighash_data = output_address.to_bytes();
            extra_sighash_data.extend_from_slice(&amount.to_le_bytes());
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // value_balance should be 5000 (10,000 spent - 5,000 output)
            assert_eq!(value_balance, 5_000);

            // --- Set up platform state ---
            // Insert anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor_bytes);

            // Set pool total balance so the unshield has sufficient funds
            set_pool_total_balance(&platform, 10_000);

            // --- Create and process transition ---
            // amount = value_balance (no fee difference)
            let transition = create_unshield_transition(
                output_address,
                amount, // amount = 5000
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

        #[test]
        fn test_wrong_encrypted_note_size_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Create action with wrong encrypted_note size
            let mut bad_action = create_dummy_serialized_action();
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 692

            let transition = create_unshield_transition(
                create_output_address(),
                1000,
                vec![bad_action],
                0x03,
                1000,
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }
    }

    // ==========================================
    // SECURITY AUDIT TESTS
    // ==========================================

    mod security_audit {
        use super::*;
        use grovedb_commitment_tree::{
            new_memory_store, Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            CommitmentTree, ExtractedNoteCommitment, FullViewingKey, MerklePath, Note, NoteValue,
            Position, ProvingKey, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
        };
        use orchard::note::RandomSeed;
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

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

        /// Build a valid Orchard bundle for unshield tests (spend > output).
        /// The `output_address` and `amount` are bound to the sighash so that
        /// the resulting bundle can only be used with those specific transparent fields.
        /// Returns (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_unshield_bundle(
            output_address: &PlatformAddress,
            amount: u64,
        ) -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(10_000), rho, rseed).unwrap();

            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = CommitmentTree::new(new_memory_store(), 100);
            tree.append(cmx, Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.orchard_witness(Position::from(0u64)).unwrap().unwrap();

            // Spend 10,000 → output 5,000 → value_balance = 5,000
            let mut builder = Builder::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            // Bind transparent fields (output_address, amount) to the sighash
            let mut extra_sighash_data = output_address.to_bytes();
            extra_sighash_data.extend_from_slice(&amount.to_le_bytes());
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle(&bundle)
        }

        /// AUDIT REGRESSION: Mutating value_balance is now caught by BatchValidator.
        ///
        /// Previously, the code only called `bundle.verify_proof(vk)` which did not
        /// check the binding signature. Now `BatchValidator` verifies the Halo 2 proof
        /// AND the binding signature, which cryptographically binds value_balance to
        /// the value commitments (cv_net). Mutating value_balance changes the bundle
        /// commitment (sighash), causing signature verification to fail.
        ///
        /// Original severity: CRITICAL — now FIXED.
        #[test]
        fn test_valid_proof_with_mutated_value_balance_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Bundle is signed for create_output_address() with amount = 5000
            let output_address = create_output_address();
            let signed_amount = 5_000u64;
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_unshield_bundle(&output_address, signed_amount);
            assert_eq!(value_balance, 5_000);

            // ATTACK: Inflate value_balance from 5000 to 9000
            let mutated_value_balance = 9_000i64;

            // Set pool balance high enough for the inflated amount
            set_pool_total_balance(&platform, 20_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_unshield_transition(
                output_address,
                mutated_value_balance as u64, // amount = 9000 (inflated)
                actions,
                flags,
                mutated_value_balance, // MUTATED: was 5000, now 9000
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: BatchValidator detects the binding signature mismatch.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT REGRESSION: Different output_address is now caught by platform sighash.
        ///
        /// Previously, the output_address was not bound to the Orchard bundle via
        /// sighash, allowing an attacker to substitute a different address while
        /// reusing a valid bundle. Now `compute_platform_sighash()` includes the
        /// output_address and amount in the sighash, so changing the address causes
        /// signature verification to fail.
        ///
        /// Original severity: HIGH — now FIXED.
        #[test]
        fn test_different_output_address_with_same_valid_bundle_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Bundle is signed for the ORIGINAL address with amount = 5000
            let original_address = create_output_address();
            let amount = 5_000u64;
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_unshield_bundle(&original_address, amount);
            assert_eq!(value_balance, 5_000);

            set_pool_total_balance(&platform, 20_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // ATTACK: Use a completely different output address
            let attacker_address = PlatformAddress::P2pkh([0xAA; 20]);

            let transition = create_unshield_transition(
                attacker_address, // ATTACKER's address, not the original recipient
                amount,
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: Platform sighash includes output_address, so changing it
            // causes the sighash to differ from the one used during signing,
            // and signature verification fails.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// Duplicate nullifiers within the same bundle are caught by the
        /// intra-bundle dedup check before reaching proof verification.
        #[test]
        fn test_duplicate_nullifiers_in_same_bundle() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];
            insert_anchor_into_state(&platform, &anchor);
            set_pool_total_balance(&platform, 10_000);

            let action1 = create_dummy_serialized_action();
            let mut action2 = create_dummy_serialized_action();
            action2.cmx = [99u8; 32];

            let transition = create_unshield_transition(
                create_output_address(),
                1000,
                vec![action1, action2], // Both have nullifier [1u8; 32]
                0x03,
                1000,
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Intra-bundle duplicate nullifier check catches this before proof verification
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::NullifierAlreadySpentError(_))
                )]
            );
        }
    }
}
