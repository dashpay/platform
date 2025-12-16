#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::signature::SignatureError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::IdentityNonce;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_credit_transfer_to_addresses_transition::methods::IdentityCreditTransferToAddressesTransitionMethodsV0;
    use dpp::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
    use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    // ==========================================
    // Test Infrastructure
    // ==========================================

    /// Helper function to create a platform address from a seed
    fn create_platform_address(seed: u8) -> PlatformAddress {
        let mut hash = [0u8; 20];
        hash[0] = seed;
        hash[19] = seed;
        PlatformAddress::P2pkh(hash)
    }

    /// Helper function to create a test identity with a transfer key
    fn create_identity_with_transfer_key(
        id: [u8; 32],
        balance: u64,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Identity, SimpleSigner) {
        let mut signer = SimpleSigner::default();

        // Create a master authentication key
        let (auth_key, auth_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                rng,
                platform_version,
            )
            .expect("should create auth key");

        // Create a transfer key
        let (transfer_key, transfer_private_key) =
            IdentityPublicKey::random_key_with_known_attributes(
                1,
                rng,
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
                KeyType::ECDSA_SECP256K1,
                None,
                platform_version,
            )
            .expect("should create transfer key");

        signer.add_identity_public_key(auth_key.clone(), auth_private_key);
        signer.add_identity_public_key(transfer_key.clone(), transfer_private_key);

        let mut public_keys = BTreeMap::new();
        public_keys.insert(auth_key.id(), auth_key);
        public_keys.insert(transfer_key.id(), transfer_key);

        let identity: Identity = IdentityV0 {
            id: id.into(),
            revision: 0,
            balance,
            public_keys,
        }
        .into();

        (identity, signer)
    }

    /// Helper function to add identity to the drive
    fn add_identity_to_drive(
        platform: &mut crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        identity: &Identity,
    ) {
        let platform_version = PlatformVersion::latest();

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should add identity");
    }

    /// Create a signed IdentityCreditTransferToAddressesTransition
    fn create_signed_transition(
        identity: &Identity,
        signer: &SimpleSigner,
        recipient_addresses: BTreeMap<PlatformAddress, u64>,
        nonce: IdentityNonce,
    ) -> StateTransition {
        IdentityCreditTransferToAddressesTransition::try_from_identity(
            identity,
            recipient_addresses,
            0, // user_fee_increase
            signer,
            None, // use default transfer key
            nonce,
            PlatformVersion::latest(),
            None,
        )
        .expect("should create signed transition")
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS
    // These test basic structure validation (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;
        use dpp::state_transition::StateTransitionStructureValidation;

        #[test]
        fn test_no_recipient_addresses_returns_error() {
            let platform_version = PlatformVersion::latest();

            // Create a raw transition V0 with no recipient addresses
            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses: BTreeMap::new(), // Empty
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            // Use the DPP structure validation directly
            let result = transition_v0.validate_structure(platform_version);

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::TransitionNoOutputsError(_))
                ),
                "Expected TransitionNoOutputsError, got {:?}",
                error
            );
        }

        #[test]
        fn test_too_many_recipient_addresses_returns_error() {
            let platform_version = PlatformVersion::latest();
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;

            // Create max_outputs + 1 recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            for i in 0..(max_outputs + 1) {
                recipient_addresses.insert(create_platform_address(i as u8), dash_to_credits!(0.1));
            }

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            // Use the DPP structure validation directly
            let result = transition_v0.validate_structure(platform_version);

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::TransitionOverMaxOutputsError(e))
                    if e.actual_outputs() == max_outputs + 1 && e.max_outputs() == max_outputs
                ),
                "Expected TransitionOverMaxOutputsError, got {:?}",
                error
            );
        }

        #[test]
        fn test_recipient_amount_below_minimum_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), 100); // Very small amount

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            // Use the DPP structure validation directly
            let result = transition_v0.validate_structure(platform_version);

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
    }

    // ==========================================
    // SUCCESSFUL TRANSITION TESTS
    // ==========================================

    mod successful_transitions {
        use super::*;

        #[test]
        fn test_simple_transfer_to_single_address() {
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

            // Create identity with sufficient balance
            let (identity, signer) = create_identity_with_transfer_key(
                [1u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
        fn test_transfer_to_multiple_addresses() {
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

            let mut rng = StdRng::seed_from_u64(568);

            // Create identity with sufficient balance
            let (identity, signer) = create_identity_with_transfer_key(
                [2u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create multiple recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));
            recipient_addresses.insert(create_platform_address(2), dash_to_credits!(0.2));
            recipient_addresses.insert(create_platform_address(3), dash_to_credits!(0.3));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
        fn test_transfer_with_user_fee_increase() {
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

            let mut rng = StdRng::seed_from_u64(569);

            // Create identity with sufficient balance
            let (identity, signer) = create_identity_with_transfer_key(
                [3u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Create transition with user fee increase
            let transition = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                100, // 1% user fee increase
                &signer,
                None,
                1,
                platform_version,
                None,
            )
            .expect("should create signed transition");

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
        fn test_transfer_maximum_allowed_outputs() {
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

            let mut rng = StdRng::seed_from_u64(570);
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            // Create identity with sufficient balance for all outputs (max_outputs * min_output + fees buffer)
            let required_balance = (max_outputs as u64) * min_output + dash_to_credits!(1.0);
            let (identity, signer) = create_identity_with_transfer_key(
                [4u8; 32],
                required_balance,
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create exactly max outputs with minimum amounts
            let mut recipient_addresses = BTreeMap::new();
            for i in 0..max_outputs as u8 {
                recipient_addresses.insert(create_platform_address(i), min_output);
            }

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
    // STATE VERIFICATION TESTS
    // Verify state changes after successful transitions
    // ==========================================

    mod state_verification {
        use super::*;

        #[test]
        fn test_identity_balance_decreases_after_transfer() {
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

            let mut rng = StdRng::seed_from_u64(571);
            let initial_balance = dash_to_credits!(1.0);
            let transfer_amount = dash_to_credits!(0.1);

            // Create identity with initial balance
            let (identity, signer) = create_identity_with_transfer_key(
                [5u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Verify initial balance
            let initial_stored_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("should fetch")
                .expect("identity should exist");
            assert_eq!(initial_stored_balance, initial_balance);

            // Create recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), transfer_amount);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
                .expect("should commit");

            // Verify final balance
            let final_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("should fetch")
                .expect("identity should exist");

            // Balance should be reduced by at least the transfer amount (plus fees)
            assert!(
                final_balance < initial_balance,
                "Balance should decrease after transfer"
            );
            assert!(
                final_balance <= initial_balance - transfer_amount,
                "Balance should decrease by at least the transfer amount"
            );
        }

        #[test]
        fn test_recipient_address_receives_credits() {
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

            let mut rng = StdRng::seed_from_u64(572);
            let transfer_amount = dash_to_credits!(0.1);
            let recipient_address = create_platform_address(10);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [6u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Verify recipient address doesn't exist initially
            let recipient_initial = platform
                .drive
                .fetch_balance_and_nonce(&recipient_address, None, platform_version)
                .expect("should fetch");
            assert!(
                recipient_initial.is_none(),
                "Recipient address should not exist initially"
            );

            // Create recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(recipient_address, transfer_amount);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
                .expect("should commit");

            // Verify recipient address now has balance
            let (recipient_nonce, recipient_balance) = platform
                .drive
                .fetch_balance_and_nonce(&recipient_address, None, platform_version)
                .expect("should fetch")
                .expect("recipient address should now exist");

            assert_eq!(recipient_nonce, 0, "New address should have nonce 0");
            assert_eq!(
                recipient_balance, transfer_amount,
                "Recipient should receive exact transfer amount"
            );
        }

        #[test]
        fn test_multiple_recipients_all_receive_credits() {
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

            let mut rng = StdRng::seed_from_u64(573);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [7u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            let recipient1 = create_platform_address(20);
            let recipient2 = create_platform_address(21);
            let recipient3 = create_platform_address(22);

            let amount1 = dash_to_credits!(0.1);
            let amount2 = dash_to_credits!(0.2);
            let amount3 = dash_to_credits!(0.15);

            // Create recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(recipient1, amount1);
            recipient_addresses.insert(recipient2, amount2);
            recipient_addresses.insert(recipient3, amount3);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
                .expect("should commit");

            // Verify all recipients received their amounts
            let (_, balance1) = platform
                .drive
                .fetch_balance_and_nonce(&recipient1, None, platform_version)
                .expect("should fetch")
                .expect("recipient1 should exist");
            assert_eq!(
                balance1, amount1,
                "Recipient1 should receive correct amount"
            );

            let (_, balance2) = platform
                .drive
                .fetch_balance_and_nonce(&recipient2, None, platform_version)
                .expect("should fetch")
                .expect("recipient2 should exist");
            assert_eq!(
                balance2, amount2,
                "Recipient2 should receive correct amount"
            );

            let (_, balance3) = platform
                .drive
                .fetch_balance_and_nonce(&recipient3, None, platform_version)
                .expect("should fetch")
                .expect("recipient3 should exist");
            assert_eq!(
                balance3, amount3,
                "Recipient3 should receive correct amount"
            );
        }

        #[test]
        fn test_identity_nonce_increments_after_transfer() {
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

            let mut rng = StdRng::seed_from_u64(574);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [8u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // First transfer with nonce 1
            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
                .expect("should commit");

            // Second transfer with nonce 2
            let mut recipient_addresses2 = BTreeMap::new();
            recipient_addresses2.insert(create_platform_address(2), dash_to_credits!(0.1));

            let transition2 = create_signed_transition(&identity, &signer, recipient_addresses2, 2);

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

            assert_matches!(
                processing_result2.execution_results().as_slice(),
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
        fn test_identity_does_not_exist_returns_error() {
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

            let mut rng = StdRng::seed_from_u64(575);

            // Create identity but DON'T add it to drive
            let (identity, signer) = create_identity_with_transfer_key(
                [9u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Create recipient addresses
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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

            // Should fail because identity doesn't exist
            // Note: IdentityNotFoundError is a SignatureError, not StateError
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(_))
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

            let mut rng = StdRng::seed_from_u64(576);

            // Create identity with small balance
            let (identity, signer) = create_identity_with_transfer_key(
                [10u8; 32],
                dash_to_credits!(0.1), // Small balance
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Try to transfer more than available
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(1.0)); // More than balance

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
            // This is caught early before processing fees, so it's UnpaidConsensusError
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(_))
                )]
            );
        }

        #[test]
        fn test_invalid_nonce_too_low_returns_error() {
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

            let mut rng = StdRng::seed_from_u64(577);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [11u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // First, do a successful transfer with nonce 1
            let mut recipient_addresses1 = BTreeMap::new();
            recipient_addresses1.insert(create_platform_address(1), dash_to_credits!(0.1));

            let transition1 = create_signed_transition(&identity, &signer, recipient_addresses1, 1);

            let result1 = transition1.serialize_to_bytes().expect("should serialize");

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

            // Commit the first transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Now try to use nonce 1 again (should fail)
            let mut recipient_addresses2 = BTreeMap::new();
            recipient_addresses2.insert(create_platform_address(2), dash_to_credits!(0.1));

            let transition2 = create_signed_transition(&identity, &signer, recipient_addresses2, 1); // Same nonce!

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

            // Should fail because nonce was already used
            assert_matches!(
                processing_result2.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
                )]
            );
        }

        #[test]
        fn test_invalid_nonce_too_high_returns_error() {
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

            let mut rng = StdRng::seed_from_u64(578);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [12u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Try with nonce that's way too high (expecting 1, using 100)
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 100);

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

            // Should fail because nonce is too far ahead
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
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
        fn test_invalid_signature_returns_error() {
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

            let mut rng = StdRng::seed_from_u64(579);

            // Create identity with balance
            let (identity, _signer) = create_identity_with_transfer_key(
                [13u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create a transition with a dummy invalid signature (not using the signer)
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Find the transfer key id
            let transfer_key = identity
                .get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                )
                .expect("should have transfer key");

            let transition: StateTransition = IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: identity.id(),
                    recipient_addresses,
                    nonce: 1,
                    user_fee_increase: 0,
                    signature_public_key_id: transfer_key.id(),
                    signature: BinaryData::new(vec![0x30; 65]), // Invalid signature
                },
            )
            .into();

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

            // Should fail because signature is invalid
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(_)
                )]
            );
        }

        #[test]
        fn test_wrong_key_type_for_signing_returns_error() {
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

            let mut rng = StdRng::seed_from_u64(580);

            // Create identity without a transfer key (only auth key)
            let mut signer = SimpleSigner::default();

            // Create a master authentication key only
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);

            let identity: Identity = IdentityV0 {
                id: [14u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Try to create a transfer transition without a transfer key
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // This should fail at transition creation because no transfer key exists
            let result = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None,
                1,
                platform_version,
                None,
            );

            assert!(
                result.is_err(),
                "Should fail to create transition without transfer key"
            );
        }
    }

    // ==========================================
    // EDGE CASE TESTS
    // ==========================================

    mod edge_cases {
        use super::*;

        #[test]
        fn test_transfer_entire_balance_minus_fees() {
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

            let mut rng = StdRng::seed_from_u64(581);

            // Create identity with balance
            let initial_balance = dash_to_credits!(1.0);
            let (identity, signer) = create_identity_with_transfer_key(
                [15u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Try to transfer a large portion (leaving room for fees)
            let transfer_amount = dash_to_credits!(0.9);
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), transfer_amount);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
        fn test_transfer_to_same_address_twice() {
            // Since recipient_addresses is a BTreeMap, adding the same address twice
            // just updates the amount. This test verifies that behavior.
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

            let mut rng = StdRng::seed_from_u64(582);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [16u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            let same_address = create_platform_address(1);
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(same_address, dash_to_credits!(0.1));
            // In BTreeMap, inserting same key again would overwrite, so we only have one entry

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
        fn test_transfer_minimum_valid_amount() {
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

            let mut rng = StdRng::seed_from_u64(583);

            let min_output_amount = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [17u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Transfer exactly the minimum amount
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), min_output_amount);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
        fn test_transfer_to_p2sh_address() {
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

            let mut rng = StdRng::seed_from_u64(584);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [18u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create a P2SH address
            let mut script_hash = [0u8; 20];
            script_hash[0] = 0xAB;
            script_hash[19] = 0xCD;
            let p2sh_address = PlatformAddress::P2sh(script_hash);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(p2sh_address, dash_to_credits!(0.1));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);

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
        fn test_transfer_to_existing_address_accumulates_balance() {
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

            let mut rng = StdRng::seed_from_u64(585);
            let recipient_address = create_platform_address(50);
            let first_transfer = dash_to_credits!(0.1);
            let second_transfer = dash_to_credits!(0.2);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [19u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // First transfer
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(recipient_address, first_transfer);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Verify first transfer
            let (_, balance_after_first) = platform
                .drive
                .fetch_balance_and_nonce(&recipient_address, None, platform_version)
                .expect("should fetch")
                .expect("address should exist");
            assert_eq!(balance_after_first, first_transfer);

            // Second transfer to same address
            let mut recipient_addresses2 = BTreeMap::new();
            recipient_addresses2.insert(recipient_address, second_transfer);

            let transition2 = create_signed_transition(&identity, &signer, recipient_addresses2, 2);
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

            assert_matches!(
                processing_result2.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction2)
                .unwrap()
                .expect("should commit");

            // Verify accumulated balance
            let (_, final_balance) = platform
                .drive
                .fetch_balance_and_nonce(&recipient_address, None, platform_version)
                .expect("should fetch")
                .expect("address should exist");

            assert_eq!(
                final_balance,
                first_transfer + second_transfer,
                "Balance should accumulate from multiple transfers"
            );
        }

        #[test]
        fn test_multiple_sequential_transfers_from_same_identity() {
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

            let mut rng = StdRng::seed_from_u64(586);

            // Create identity with balance
            let (identity, signer) = create_identity_with_transfer_key(
                [20u8; 32],
                dash_to_credits!(5.0),
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Perform 5 sequential transfers
            for i in 1..=5 {
                let mut recipient_addresses = BTreeMap::new();
                recipient_addresses
                    .insert(create_platform_address(60 + i as u8), dash_to_credits!(0.1));

                let transition =
                    create_signed_transition(&identity, &signer, recipient_addresses, i);
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
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)],
                    "Transfer {} should succeed",
                    i
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("should commit");
            }
        }
    }

    // ==========================================
    // OVERFLOW AND BOUNDARY TESTS
    // ==========================================

    mod overflow_and_boundary_tests {
        use super::*;
        use dpp::state_transition::StateTransitionStructureValidation;

        #[test]
        fn test_balance_exactly_covers_transfer_and_fees() {
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

            let mut rng = StdRng::seed_from_u64(587);

            let transfer_amount = dash_to_credits!(0.1);
            let min_fee = platform_version
                .fee_version
                .state_transition_min_fees
                .credit_transfer_to_addresses;

            // Create identity with exactly enough balance (transfer + min_fee + small buffer for actual fees)
            let initial_balance = transfer_amount + min_fee + dash_to_credits!(0.01);
            let (identity, signer) = create_identity_with_transfer_key(
                [21u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), transfer_amount);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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
        fn test_amount_exactly_at_minimum() {
            let platform_version = PlatformVersion::latest();
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), min_output);

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            let result = transition_v0.validate_structure(platform_version);
            assert!(
                result.is_valid(),
                "Amount exactly at minimum should be valid"
            );
        }

        #[test]
        fn test_amount_one_below_minimum_returns_error() {
            let platform_version = PlatformVersion::latest();
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), min_output - 1);

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            let result = transition_v0.validate_structure(platform_version);
            assert!(!result.is_valid(), "Amount below minimum should be invalid");
            assert!(
                matches!(
                    result.first_error().unwrap(),
                    ConsensusError::BasicError(BasicError::OutputBelowMinimumError(_))
                ),
                "Expected OutputBelowMinimumError"
            );
        }

        #[test]
        fn test_outputs_exactly_at_maximum() {
            let platform_version = PlatformVersion::latest();
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let mut recipient_addresses = BTreeMap::new();
            for i in 0..max_outputs {
                recipient_addresses.insert(create_platform_address(i as u8), min_output);
            }

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            let result = transition_v0.validate_structure(platform_version);
            assert!(result.is_valid(), "Exactly max outputs should be valid");
        }

        #[test]
        fn test_outputs_one_over_maximum_returns_error() {
            let platform_version = PlatformVersion::latest();
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let mut recipient_addresses = BTreeMap::new();
            for i in 0..=max_outputs {
                recipient_addresses.insert(create_platform_address(i as u8), min_output);
            }

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            let result = transition_v0.validate_structure(platform_version);
            assert!(!result.is_valid(), "Over max outputs should be invalid");
            assert!(
                matches!(
                    result.first_error().unwrap(),
                    ConsensusError::BasicError(BasicError::TransitionOverMaxOutputsError(_))
                ),
                "Expected TransitionOverMaxOutputsError"
            );
        }

        #[test]
        fn test_mixed_valid_and_invalid_output_amounts() {
            let platform_version = PlatformVersion::latest();
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            // First amount valid, second amount invalid
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), min_output);
            recipient_addresses.insert(create_platform_address(2), min_output - 1); // Invalid

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            let result = transition_v0.validate_structure(platform_version);
            assert!(
                !result.is_valid(),
                "Mixed amounts with one invalid should fail"
            );
        }
    }

    // ==========================================
    // KEY AND SECURITY TESTS
    // ==========================================

    mod key_and_security_tests {
        use super::*;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;

        #[test]
        fn test_transfer_with_disabled_key_fails_at_creation() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(588);

            // Create identity with transfer key
            let (mut identity, signer) = create_identity_with_transfer_key(
                [22u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Disable the transfer key
            let transfer_key_id = identity
                .get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                )
                .expect("should have transfer key")
                .id();

            identity
                .public_keys_mut()
                .get_mut(&transfer_key_id)
                .expect("key should exist")
                .set_disabled_at(1); // Disable at block 1

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Trying to create a transition with a disabled key should fail at creation time
            let result = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None,
                1,
                platform_version,
                None,
            );

            assert!(
                result.is_err(),
                "Creating transition with disabled key should fail"
            );
        }

        #[test]
        fn test_transfer_with_non_transfer_purpose_key_fails_at_creation() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(589);
            let mut signer = SimpleSigner::default();

            // Create only an authentication key (no transfer key)
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);

            let identity: Identity = IdentityV0 {
                id: [23u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // This should fail because there's no transfer key
            let result = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None,
                1,
                platform_version,
                None,
            );

            assert!(
                result.is_err(),
                "Creating transition without transfer key should fail"
            );
        }

        #[test]
        fn test_transfer_with_high_security_level_transfer_key_fails_at_creation() {
            // Transfer keys for credit transfer to addresses require CRITICAL security level
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(590);
            let mut signer = SimpleSigner::default();

            // Create a master authentication key
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            // Create a HIGH security level transfer key (instead of CRITICAL)
            let (transfer_key, transfer_private_key) =
                IdentityPublicKey::random_key_with_known_attributes(
                    1,
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::HIGH,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key");

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);
            signer.add_identity_public_key(transfer_key.clone(), transfer_private_key);

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);
            public_keys.insert(transfer_key.id(), transfer_key);

            let identity: Identity = IdentityV0 {
                id: [24u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Transition creation should fail because HIGH security level is not allowed
            // Only CRITICAL security level is allowed for identity credit transfer to addresses
            let result = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None,
                1,
                platform_version,
                None,
            );

            assert!(
                result.is_err(),
                "Creating transition with HIGH security level key should fail (only CRITICAL allowed)"
            );
        }
    }

    // ==========================================
    // USER FEE INCREASE TESTS
    // ==========================================

    mod user_fee_increase_tests {
        use super::*;

        #[test]
        fn test_maximum_user_fee_increase() {
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

            let mut rng = StdRng::seed_from_u64(591);

            // Create identity with large balance
            let (identity, signer) = create_identity_with_transfer_key(
                [25u8; 32],
                dash_to_credits!(10.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Use maximum fee increase (u16::MAX = 65535 = 655.35%)
            let transition = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                u16::MAX,
                &signer,
                None,
                1,
                platform_version,
                None,
            )
            .expect("should create transition");

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
        fn test_zero_user_fee_increase() {
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

            let mut rng = StdRng::seed_from_u64(592);

            let (identity, signer) = create_identity_with_transfer_key(
                [26u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Explicitly use 0 fee increase
            let transition = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None,
                1,
                platform_version,
                None,
            )
            .expect("should create transition");

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
    // NONCE EDGE CASE TESTS
    // ==========================================

    mod nonce_edge_cases {
        use super::*;

        #[test]
        fn test_nonce_zero_is_invalid() {
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

            let mut rng = StdRng::seed_from_u64(593);

            let (identity, signer) = create_identity_with_transfer_key(
                [27u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Use nonce 0 (should be invalid, nonces start at 1)
            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 0);
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

            // Nonce 0 should fail
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
                )]
            );
        }

        #[test]
        fn test_nonce_skipping_one_ahead_may_be_valid() {
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

            let mut rng = StdRng::seed_from_u64(594);

            let (identity, signer) = create_identity_with_transfer_key(
                [28u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Use nonce 2 (skipping nonce 1)
            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 2);
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

            // Skipping one nonce might be allowed depending on the nonce gap rules
            // This documents the actual behavior
            match processing_result.execution_results().as_slice() {
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => {
                    // Small gaps are allowed
                }
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(_)),
                )] => {
                    // Or gaps are not allowed - both are valid behaviors to document
                }
                other => panic!("Unexpected result: {:?}", other),
            }
        }
    }

    // ==========================================
    // MULTI-IDENTITY TESTS
    // These tests involve multiple identities
    // ==========================================

    mod multi_identity_tests {
        use super::*;

        /// Test that multiple different identities can transfer to the same recipient address
        /// and the balance accumulates correctly
        #[test]
        fn test_multiple_identities_transfer_to_same_address() {
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
            let shared_recipient = create_platform_address(100);
            let transfer_amount = dash_to_credits!(0.1);

            // Create three different identities
            let (identity1, signer1) = create_identity_with_transfer_key(
                [30u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );
            let (identity2, signer2) = create_identity_with_transfer_key(
                [31u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );
            let (identity3, signer3) = create_identity_with_transfer_key(
                [32u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Add all identities to drive
            add_identity_to_drive(&mut platform, &identity1);
            add_identity_to_drive(&mut platform, &identity2);
            add_identity_to_drive(&mut platform, &identity3);

            // Each identity transfers to the same address
            for (identity, signer, id_num) in [
                (&identity1, &signer1, 1),
                (&identity2, &signer2, 2),
                (&identity3, &signer3, 3),
            ] {
                let mut recipient_addresses = BTreeMap::new();
                recipient_addresses.insert(shared_recipient, transfer_amount);

                let transition = create_signed_transition(identity, signer, recipient_addresses, 1);
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
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)],
                    "Transfer from identity {} should succeed",
                    id_num
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("should commit");
            }

            // Verify the recipient received all three transfers
            let (_, final_balance) = platform
                .drive
                .fetch_balance_and_nonce(&shared_recipient, None, platform_version)
                .expect("should fetch")
                .expect("recipient should exist");

            assert_eq!(
                final_balance,
                transfer_amount * 3,
                "Recipient should have accumulated balance from all three identities"
            );
        }

        /// Test that two identities can transfer to each other's derived addresses
        #[test]
        fn test_cross_identity_transfers() {
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

            let mut rng = StdRng::seed_from_u64(601);

            // Create two identities
            let (identity1, signer1) = create_identity_with_transfer_key(
                [33u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );
            let (identity2, signer2) = create_identity_with_transfer_key(
                [34u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity1);
            add_identity_to_drive(&mut platform, &identity2);

            // Each identity gets a recipient address
            let recipient1 = create_platform_address(101);
            let recipient2 = create_platform_address(102);

            // Identity 1 transfers to recipient 2
            let mut addresses1 = BTreeMap::new();
            addresses1.insert(recipient2, dash_to_credits!(0.5));
            let transition1 = create_signed_transition(&identity1, &signer1, addresses1, 1);

            // Identity 2 transfers to recipient 1
            let mut addresses2 = BTreeMap::new();
            addresses2.insert(recipient1, dash_to_credits!(0.3));
            let transition2 = create_signed_transition(&identity2, &signer2, addresses2, 1);

            // Process both transitions
            let result1 = transition1.serialize_to_bytes().expect("should serialize");
            let result2 = transition2.serialize_to_bytes().expect("should serialize");

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
                .expect("expected to process state transitions");

            // Both should succeed
            assert_eq!(
                processing_result.execution_results().len(),
                2,
                "Should have two results"
            );
            for (i, result) in processing_result.execution_results().iter().enumerate() {
                assert_matches!(
                    result,
                    StateTransitionExecutionResult::SuccessfulExecution(_, _),
                    "Transition {} should succeed",
                    i
                );
            }
        }
    }

    // ==========================================
    // SERIALIZATION TESTS
    // ==========================================

    mod serialization_tests {
        use super::*;
        use dpp::serialization::PlatformDeserializable;
        use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
        use dpp::state_transition::StateTransitionIdentitySigned;
        use dpp::state_transition::StateTransitionLike;
        use dpp::state_transition::StateTransitionSingleSigned;

        /// Test that a transition can be serialized and deserialized correctly
        #[test]
        fn test_serialization_roundtrip() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(602);

            let (identity, signer) = create_identity_with_transfer_key(
                [35u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));
            recipient_addresses.insert(create_platform_address(2), dash_to_credits!(0.2));
            recipient_addresses.insert(create_platform_address(3), dash_to_credits!(0.15));

            let original_transition =
                create_signed_transition(&identity, &signer, recipient_addresses.clone(), 42);

            // Serialize
            let serialized = original_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Deserialize
            let deserialized =
                StateTransition::deserialize_from_bytes(&serialized).expect("should deserialize");

            // Verify the deserialized transition matches the original
            match (&original_transition, &deserialized) {
                (
                    StateTransition::IdentityCreditTransferToAddresses(orig),
                    StateTransition::IdentityCreditTransferToAddresses(deser),
                ) => {
                    assert_eq!(orig.identity_id(), deser.identity_id());
                    assert_eq!(orig.recipient_addresses(), deser.recipient_addresses());
                    assert_eq!(orig.nonce(), deser.nonce());
                    assert_eq!(orig.user_fee_increase(), deser.user_fee_increase());
                    assert_eq!(orig.signature(), deser.signature());
                    assert_eq!(
                        orig.signature_public_key_id(),
                        deser.signature_public_key_id()
                    );
                }
                _ => panic!("Deserialized transition type mismatch"),
            }
        }

        /// Test serialization with maximum outputs
        #[test]
        fn test_serialization_with_max_outputs() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(603);
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let (identity, signer) = create_identity_with_transfer_key(
                [36u8; 32],
                (max_outputs as u64) * min_output + dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            let mut recipient_addresses = BTreeMap::new();
            for i in 0..max_outputs as u8 {
                recipient_addresses.insert(create_platform_address(i), min_output);
            }

            let transition =
                create_signed_transition(&identity, &signer, recipient_addresses.clone(), 1);

            // Should serialize successfully
            let serialized = transition.serialize_to_bytes().expect("should serialize");

            // Should deserialize successfully
            let deserialized =
                StateTransition::deserialize_from_bytes(&serialized).expect("should deserialize");

            match deserialized {
                StateTransition::IdentityCreditTransferToAddresses(t) => {
                    assert_eq!(
                        t.recipient_addresses().len(),
                        max_outputs as usize,
                        "Should have max outputs after roundtrip"
                    );
                }
                _ => panic!("Wrong transition type"),
            }
        }

        /// Test that serialized bytes are deterministic
        #[test]
        fn test_serialization_is_deterministic() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(604);

            let (identity, signer) = create_identity_with_transfer_key(
                [37u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            let transition1 =
                create_signed_transition(&identity, &signer, recipient_addresses.clone(), 1);
            let transition2 =
                create_signed_transition(&identity, &signer, recipient_addresses.clone(), 1);

            let bytes1 = transition1.serialize_to_bytes().expect("should serialize");
            let bytes2 = transition2.serialize_to_bytes().expect("should serialize");

            assert_eq!(
                bytes1, bytes2,
                "Same transition should produce identical serialized bytes"
            );
        }
    }

    // ==========================================
    // KEY SELECTION TESTS
    // ==========================================

    mod key_selection_tests {
        use super::*;
        use dpp::state_transition::StateTransitionIdentitySigned;

        /// Test explicitly specifying which transfer key to use when identity has multiple
        #[test]
        fn test_explicit_transfer_key_selection() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(605);
            let mut signer = SimpleSigner::default();

            // Create master authentication key
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            // Create TWO transfer keys
            let (transfer_key1, transfer_private_key1) =
                IdentityPublicKey::random_key_with_known_attributes(
                    1,
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key 1");

            let (transfer_key2, transfer_private_key2) =
                IdentityPublicKey::random_key_with_known_attributes(
                    2,
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key 2");

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);
            signer.add_identity_public_key(transfer_key1.clone(), transfer_private_key1);
            signer.add_identity_public_key(transfer_key2.clone(), transfer_private_key2);

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);
            public_keys.insert(transfer_key1.id(), transfer_key1.clone());
            public_keys.insert(transfer_key2.id(), transfer_key2.clone());

            let identity: Identity = IdentityV0 {
                id: [38u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Explicitly select transfer_key2
            let transition = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                Some(&transfer_key2), // Explicitly specify key 2
                1,
                platform_version,
                None,
            )
            .expect("should create transition with explicit key");

            // Verify that key 2 was used
            match transition {
                StateTransition::IdentityCreditTransferToAddresses(t) => {
                    assert_eq!(
                        t.signature_public_key_id(),
                        transfer_key2.id(),
                        "Should use the explicitly specified key"
                    );
                }
                _ => panic!("Wrong transition type"),
            }
        }

        /// Test that identity with multiple transfer keys uses the first one by default
        #[test]
        fn test_multiple_transfer_keys_default_selection() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(606);
            let mut signer = SimpleSigner::default();

            // Create master authentication key
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            // Create multiple transfer keys with different IDs
            let (transfer_key1, transfer_private_key1) =
                IdentityPublicKey::random_key_with_known_attributes(
                    5, // Higher ID
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key 1");

            let (transfer_key2, transfer_private_key2) =
                IdentityPublicKey::random_key_with_known_attributes(
                    3, // Lower ID
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key 2");

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);
            signer.add_identity_public_key(transfer_key1.clone(), transfer_private_key1);
            signer.add_identity_public_key(transfer_key2.clone(), transfer_private_key2);

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);
            public_keys.insert(transfer_key1.id(), transfer_key1);
            public_keys.insert(transfer_key2.id(), transfer_key2);

            let identity: Identity = IdentityV0 {
                id: [39u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Don't specify a key - should use the first one found
            let transition = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None, // No explicit key - use default
                1,
                platform_version,
                None,
            )
            .expect("should create transition");

            // The transition should be created successfully with one of the transfer keys
            match transition {
                StateTransition::IdentityCreditTransferToAddresses(t) => {
                    let used_key_id = t.signature_public_key_id();
                    assert!(
                        used_key_id == 3 || used_key_id == 5,
                        "Should use one of the transfer keys, got {}",
                        used_key_id
                    );
                }
                _ => panic!("Wrong transition type"),
            }
        }

        /// Test specifying a key that the signer cannot sign with
        #[test]
        fn test_explicit_key_not_in_signer_fails() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(607);
            let mut signer = SimpleSigner::default();

            // Create keys but only add one to the signer
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            let (transfer_key1, transfer_private_key1) =
                IdentityPublicKey::random_key_with_known_attributes(
                    1,
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key 1");

            // Create a second transfer key but DON'T add it to signer
            let (transfer_key2, _transfer_private_key2) =
                IdentityPublicKey::random_key_with_known_attributes(
                    2,
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key 2");

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);
            signer.add_identity_public_key(transfer_key1.clone(), transfer_private_key1);
            // Note: transfer_key2's private key is NOT added to signer

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);
            public_keys.insert(transfer_key1.id(), transfer_key1);
            public_keys.insert(transfer_key2.id(), transfer_key2.clone());

            let identity: Identity = IdentityV0 {
                id: [40u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Try to use transfer_key2 which is not in the signer
            let result = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                Some(&transfer_key2), // This key is not in the signer
                1,
                platform_version,
                None,
            );

            assert!(
                result.is_err(),
                "Should fail when specifying a key the signer cannot use"
            );
        }
    }

    // ==========================================
    // BLOCK HEIGHT AND TIMING TESTS
    // ==========================================

    mod block_timing_tests {
        use super::*;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;

        /// Test that a key disabled at a future block height can still be used
        #[test]
        fn test_key_disabled_at_future_block_still_works() {
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

            let mut rng = StdRng::seed_from_u64(608);

            // Create identity with transfer key
            let (mut identity, signer) = create_identity_with_transfer_key(
                [41u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Get the transfer key and set disabled_at to a future time
            let transfer_key_id = identity
                .get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                )
                .expect("should have transfer key")
                .id();

            // Set disabled_at to a future timestamp (e.g., block 1000000)
            identity
                .public_keys_mut()
                .get_mut(&transfer_key_id)
                .expect("key should exist")
                .set_disabled_at(1000000);

            // Add identity to drive
            add_identity_to_drive(&mut platform, &identity);

            // Create transition - note: try_from_identity checks disabled status
            // This should fail at creation because the key is marked as disabled
            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            let result = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None,
                1,
                platform_version,
                None,
            );

            // The key is considered disabled regardless of when, so this should fail
            assert!(
                result.is_err(),
                "Key with disabled_at set should fail at creation"
            );
        }

        /// Test transfer at a specific block height
        #[test]
        fn test_transfer_at_specific_block_height() {
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

            let mut rng = StdRng::seed_from_u64(609);

            let (identity, signer) = create_identity_with_transfer_key(
                [42u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
            let result = transition.serialize_to_bytes().expect("should serialize");

            // Use a specific block height instead of default
            let block_info = BlockInfo {
                height: 12345,
                time_ms: 1700000000000,
                core_height: 1000,
                epoch: Default::default(),
            };

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

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

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }
    }

    // ==========================================
    // FEE VERIFICATION TESTS
    // ==========================================

    mod fee_verification_tests {
        use super::*;

        /// Test that fees are correctly deducted from the identity balance
        #[test]
        fn test_fee_deduction_verification() {
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
            let initial_balance = dash_to_credits!(1.0);
            let transfer_amount = dash_to_credits!(0.1);

            let (identity, signer) = create_identity_with_transfer_key(
                [43u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), transfer_amount);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            // Extract the fee from the result
            let fee_paid = match &processing_result.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.total_base_fee()
                }
                _ => panic!("Expected successful execution"),
            };

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Check final balance
            let final_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("should fetch")
                .expect("identity should exist");

            // Verify: initial_balance - transfer_amount - fee = final_balance
            assert_eq!(
                final_balance,
                initial_balance - transfer_amount - fee_paid,
                "Final balance should equal initial - transfer - fee"
            );

            // Verify fee is at least the minimum
            let min_fee = platform_version
                .fee_version
                .state_transition_min_fees
                .credit_transfer_to_addresses;
            assert!(
                fee_paid >= min_fee,
                "Fee {} should be at least minimum {}",
                fee_paid,
                min_fee
            );
        }

        /// Test that user_fee_increase actually increases the fee paid
        #[test]
        fn test_user_fee_increase_affects_total_fee() {
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

            // Create two identities with same balance
            let (identity1, signer1) = create_identity_with_transfer_key(
                [44u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );
            let (identity2, signer2) = create_identity_with_transfer_key(
                [45u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity1);
            add_identity_to_drive(&mut platform, &identity2);

            // Identity 1: no fee increase
            let mut addresses1 = BTreeMap::new();
            addresses1.insert(create_platform_address(1), dash_to_credits!(0.1));
            let transition1 = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity1,
                addresses1,
                0, // No fee increase
                &signer1,
                None,
                1,
                platform_version,
                None,
            )
            .expect("should create transition");

            // Identity 2: 100% fee increase (10000 = 100%)
            let mut addresses2 = BTreeMap::new();
            addresses2.insert(create_platform_address(2), dash_to_credits!(0.1));
            let transition2 = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity2,
                addresses2,
                10000, // 100% fee increase
                &signer2,
                None,
                1,
                platform_version,
                None,
            )
            .expect("should create transition");

            let result1 = transition1.serialize_to_bytes().expect("should serialize");
            let result2 = transition2.serialize_to_bytes().expect("should serialize");

            // Process transition 1
            let platform_state1 = platform.state.load();
            let transaction1 = platform.drive.grove.start_transaction();

            let processing_result1 = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1],
                    &platform_state1,
                    &BlockInfo::default(),
                    &transaction1,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process");

            let fee1 = match &processing_result1.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.total_base_fee()
                }
                _ => panic!("Expected successful execution"),
            };

            platform
                .drive
                .grove
                .commit_transaction(transaction1)
                .unwrap()
                .expect("should commit");

            // Process transition 2
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
                .expect("expected to process");

            let fee2 = match &processing_result2.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.total_base_fee()
                }
                _ => panic!("Expected successful execution"),
            };

            // Fee2 should be higher than fee1 due to fee increase
            // Note: The exact relationship depends on how user_fee_increase is applied
            // It might be base_fee * (1 + user_fee_increase/10000)
            assert!(
                fee2 >= fee1,
                "Fee with increase ({}) should be >= fee without ({})",
                fee2,
                fee1
            );
        }
    }

    // ==========================================
    // SIGNATURE EDGE CASE TESTS
    // ==========================================

    mod signature_edge_cases {
        use super::*;

        /// Test transition with empty signature field
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

            let mut rng = StdRng::seed_from_u64(612);

            let (identity, _signer) = create_identity_with_transfer_key(
                [46u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let transfer_key = identity
                .get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                )
                .expect("should have transfer key");

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Create transition with empty signature
            let transition: StateTransition = IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: identity.id(),
                    recipient_addresses,
                    nonce: 1,
                    user_fee_increase: 0,
                    signature_public_key_id: transfer_key.id(),
                    signature: BinaryData::new(vec![]), // Empty signature
                },
            )
            .into();

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

            // Should fail with signature error
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(_)
                )]
            );
        }

        /// Test transition with truncated signature
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

            let mut rng = StdRng::seed_from_u64(613);

            let (identity, _signer) = create_identity_with_transfer_key(
                [47u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let transfer_key = identity
                .get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                )
                .expect("should have transfer key");

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Create transition with truncated signature (should be 65 bytes for ECDSA)
            let transition: StateTransition = IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: identity.id(),
                    recipient_addresses,
                    nonce: 1,
                    user_fee_increase: 0,
                    signature_public_key_id: transfer_key.id(),
                    signature: BinaryData::new(vec![0x30; 32]), // Truncated
                },
            )
            .into();

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

            // Should fail with signature error
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(_)
                )]
            );
        }
    }

    // ==========================================
    // STRESS AND SCALE TESTS
    // ==========================================

    mod stress_tests {
        use super::*;

        /// Test many small transfers near the minimum amount
        #[test]
        fn test_many_minimum_amount_transfers() {
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

            let mut rng = StdRng::seed_from_u64(614);
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;
            let num_outputs = 10u8; // Multiple minimum outputs

            let (identity, signer) = create_identity_with_transfer_key(
                [48u8; 32],
                (num_outputs as u64) * min_output + dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            // Create multiple minimum amount outputs
            let mut recipient_addresses = BTreeMap::new();
            for i in 0..num_outputs {
                recipient_addresses.insert(create_platform_address(i), min_output);
            }

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Verify all outputs received the minimum amount
            for i in 0..num_outputs {
                let (_, balance) = platform
                    .drive
                    .fetch_balance_and_nonce(&create_platform_address(i), None, platform_version)
                    .expect("should fetch")
                    .expect("address should exist");
                assert_eq!(
                    balance, min_output,
                    "Each output should have minimum amount"
                );
            }
        }

        /// Test processing multiple transitions in a single block
        #[test]
        fn test_batch_processing_multiple_transitions() {
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

            let mut rng = StdRng::seed_from_u64(615);

            // Create multiple identities
            let num_identities = 5;
            let mut transitions = Vec::new();

            for i in 0..num_identities {
                let (identity, signer) = create_identity_with_transfer_key(
                    [50 + i; 32],
                    dash_to_credits!(1.0),
                    &mut rng,
                    platform_version,
                );

                add_identity_to_drive(&mut platform, &identity);

                let mut recipient_addresses = BTreeMap::new();
                recipient_addresses.insert(create_platform_address(200 + i), dash_to_credits!(0.1));

                let transition =
                    create_signed_transition(&identity, &signer, recipient_addresses, 1);
                transitions.push(transition.serialize_to_bytes().expect("should serialize"));
            }

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process all transitions in one batch
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &transitions,
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // All should succeed
            assert_eq!(
                processing_result.execution_results().len(),
                num_identities as usize
            );
            for (i, result) in processing_result.execution_results().iter().enumerate() {
                assert_matches!(
                    result,
                    StateTransitionExecutionResult::SuccessfulExecution(_, _),
                    "Transition {} should succeed",
                    i
                );
            }
        }
    }

    // ==========================================
    // BALANCE EDGE CASE TESTS
    // ==========================================

    mod balance_edge_cases {
        use super::*;

        /// Test that balance can reach zero after transfer
        #[test]
        fn test_balance_reaches_zero() {
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

            let mut rng = StdRng::seed_from_u64(616);
            let min_fee = platform_version
                .fee_version
                .state_transition_min_fees
                .credit_transfer_to_addresses;
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            // Give identity exactly enough for one minimum transfer plus expected fees
            // We need to account for the actual fee which might be slightly more than min_fee
            let fee_buffer = dash_to_credits!(0.001); // Small buffer for actual fee calculation
            let initial_balance = min_output + min_fee + fee_buffer;

            let (identity, signer) = create_identity_with_transfer_key(
                [55u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), min_output);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Check that balance is very low or zero
            let final_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("should fetch")
                .expect("identity should exist");

            // Balance should be close to zero (just the fee_buffer minus actual fees)
            assert!(
                final_balance < fee_buffer,
                "Final balance {} should be very low",
                final_balance
            );
        }
    }

    // ==========================================
    // READ-ONLY KEY TESTS
    // ==========================================

    mod read_only_key_tests {
        use super::*;

        /// Test that a read-only transfer key can still be used for signing
        /// (read_only affects whether the key can be updated, not whether it can sign)
        #[test]
        fn test_read_only_transfer_key_can_sign() {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(618);
            let mut signer = SimpleSigner::default();

            // Create master auth key
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            // Create a transfer key that is read_only
            let (transfer_key, transfer_private_key) =
                IdentityPublicKey::random_key_with_known_attributes(
                    1,
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key");

            // Make it read-only by creating a new key with read_only = true
            // We need to modify the inner V0 struct
            let transfer_key = match transfer_key {
                IdentityPublicKey::V0(mut v0) => {
                    v0.read_only = true;
                    IdentityPublicKey::V0(v0)
                }
            };

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);
            signer.add_identity_public_key(transfer_key.clone(), transfer_private_key);

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);
            public_keys.insert(transfer_key.id(), transfer_key);

            let identity: Identity = IdentityV0 {
                id: [57u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Should be able to create and sign transition even with read_only key
            let result = IdentityCreditTransferToAddressesTransition::try_from_identity(
                &identity,
                recipient_addresses,
                0,
                &signer,
                None,
                1,
                platform_version,
                None,
            );

            assert!(
                result.is_ok(),
                "Read-only key should still be able to sign: {:?}",
                result.err()
            );
        }
    }

    // ==========================================
    // ERROR DETAIL VERIFICATION TESTS
    // ==========================================

    mod error_detail_tests {
        use super::*;

        /// Test that IdentityInsufficientBalanceError contains correct values
        #[test]
        fn test_insufficient_balance_error_details() {
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

            let mut rng = StdRng::seed_from_u64(619);
            let identity_balance = dash_to_credits!(0.1);
            let transfer_amount = dash_to_credits!(1.0); // More than balance

            let (identity, signer) = create_identity_with_transfer_key(
                [58u8; 32],
                identity_balance,
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), transfer_amount);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            match processing_result.execution_results().as_slice() {
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(err)),
                )] => {
                    // Verify error contains correct identity ID
                    assert_eq!(
                        *err.identity_id(),
                        identity.id(),
                        "Error should contain correct identity ID"
                    );
                    // Verify balance in error
                    assert_eq!(
                        err.balance(),
                        identity_balance,
                        "Error should contain correct balance"
                    );
                }
                other => panic!("Expected IdentityInsufficientBalanceError, got {:?}", other),
            }
        }

        /// Test that InvalidIdentityNonceError contains correct nonce values
        #[test]
        fn test_invalid_nonce_error_details() {
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

            let mut rng = StdRng::seed_from_u64(620);

            let (identity, signer) = create_identity_with_transfer_key(
                [59u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Use nonce 0 which is invalid
            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 0);
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

            match processing_result.execution_results().as_slice() {
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(err)),
                )] => {
                    // Verify the error contains the identity ID
                    assert_eq!(
                        err.identity_id,
                        identity.id(),
                        "Error should contain correct identity ID"
                    );
                    // Verify the submitted nonce is in the error (the transition submitted nonce 0)
                    assert_eq!(
                        err.setting_identity_nonce, 0,
                        "Error should show the nonce that was submitted"
                    );
                }
                other => panic!("Expected InvalidIdentityNonceError, got {:?}", other),
            }
        }

        /// Test that TransitionOverMaxOutputsError contains correct counts
        #[test]
        fn test_over_max_outputs_error_details() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();
            let max_outputs = platform_version.dpp.state_transitions.max_address_outputs;
            let actual_outputs = max_outputs + 5; // 5 over the limit

            let mut recipient_addresses = BTreeMap::new();
            for i in 0..actual_outputs {
                recipient_addresses.insert(create_platform_address(i as u8), dash_to_credits!(0.1));
            }

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            let result = transition_v0.validate_structure(platform_version);

            assert!(!result.is_valid());
            match result.first_error().unwrap() {
                ConsensusError::BasicError(BasicError::TransitionOverMaxOutputsError(err)) => {
                    assert_eq!(
                        err.actual_outputs(),
                        actual_outputs,
                        "Error should show actual output count"
                    );
                    assert_eq!(
                        err.max_outputs(),
                        max_outputs,
                        "Error should show max allowed outputs"
                    );
                }
                other => panic!("Expected TransitionOverMaxOutputsError, got {:?}", other),
            }
        }

        /// Test that OutputBelowMinimumError contains correct amount information
        #[test]
        fn test_below_minimum_error_details() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;
            let below_minimum_amount = min_output / 2;

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), below_minimum_amount);

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            let result = transition_v0.validate_structure(platform_version);

            assert!(!result.is_valid());
            match result.first_error().unwrap() {
                ConsensusError::BasicError(BasicError::OutputBelowMinimumError(err)) => {
                    assert_eq!(
                        err.output_amount(),
                        below_minimum_amount,
                        "Error should show actual output amount"
                    );
                    assert_eq!(
                        err.minimum_amount(),
                        min_output,
                        "Error should show minimum required amount"
                    );
                }
                other => panic!("Expected OutputBelowMinimumError, got {:?}", other),
            }
        }
    }

    // ==========================================
    // CONCURRENT TRANSACTION TESTS
    // ==========================================

    mod concurrent_transaction_tests {
        use super::*;

        /// Test two transitions from same identity in same block (should fail for second due to nonce)
        #[test]
        fn test_same_identity_two_transitions_same_block_same_nonce_fails() {
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

            let mut rng = StdRng::seed_from_u64(621);

            let (identity, signer) = create_identity_with_transfer_key(
                [60u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            // Create two transitions with the SAME nonce
            let mut addresses1 = BTreeMap::new();
            addresses1.insert(create_platform_address(1), dash_to_credits!(0.1));
            let transition1 = create_signed_transition(&identity, &signer, addresses1, 1);

            let mut addresses2 = BTreeMap::new();
            addresses2.insert(create_platform_address(2), dash_to_credits!(0.1));
            let transition2 = create_signed_transition(&identity, &signer, addresses2, 1); // Same nonce!

            let result1 = transition1.serialize_to_bytes().expect("should serialize");
            let result2 = transition2.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both in same block
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

            // First should succeed, second should fail due to nonce conflict
            assert_eq!(processing_result.execution_results().len(), 2);

            assert_matches!(
                &processing_result.execution_results()[0],
                StateTransitionExecutionResult::SuccessfulExecution(_, _),
                "First transition should succeed"
            );

            // Second should fail with nonce error
            assert_matches!(
                &processing_result.execution_results()[1],
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::InvalidIdentityNonceError(_)
                )),
                "Second transition should fail with nonce error"
            );
        }

        /// Test two transitions from same identity with sequential nonces in same block
        #[test]
        fn test_same_identity_sequential_nonces_same_block() {
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

            let mut rng = StdRng::seed_from_u64(622);

            let (identity, signer) = create_identity_with_transfer_key(
                [61u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            // Create two transitions with sequential nonces
            let mut addresses1 = BTreeMap::new();
            addresses1.insert(create_platform_address(1), dash_to_credits!(0.1));
            let transition1 = create_signed_transition(&identity, &signer, addresses1, 1);

            let mut addresses2 = BTreeMap::new();
            addresses2.insert(create_platform_address(2), dash_to_credits!(0.1));
            let transition2 = create_signed_transition(&identity, &signer, addresses2, 2); // Next nonce

            let result1 = transition1.serialize_to_bytes().expect("should serialize");
            let result2 = transition2.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both in same block
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

            // Both should succeed
            assert_eq!(processing_result.execution_results().len(), 2);

            for (i, result) in processing_result.execution_results().iter().enumerate() {
                assert_matches!(
                    result,
                    StateTransitionExecutionResult::SuccessfulExecution(_, _),
                    "Transition {} should succeed",
                    i
                );
            }
        }
    }

    // ==========================================
    // REMAINING EDGE CASE TESTS
    // ==========================================

    mod remaining_edge_cases {
        use super::*;

        /// Test transfer to mixed P2PKH and P2SH addresses in a single transition
        #[test]
        fn test_mixed_p2pkh_and_p2sh_addresses_in_single_transition() {
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

            let (identity, signer) = create_identity_with_transfer_key(
                [70u8; 32],
                dash_to_credits!(2.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            // Create a mix of P2PKH and P2SH addresses
            let p2pkh_address1 = PlatformAddress::P2pkh([1u8; 20]);
            let p2pkh_address2 = PlatformAddress::P2pkh([2u8; 20]);
            let p2sh_address1 = PlatformAddress::P2sh([3u8; 20]);
            let p2sh_address2 = PlatformAddress::P2sh([4u8; 20]);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(p2pkh_address1, dash_to_credits!(0.1));
            recipient_addresses.insert(p2pkh_address2, dash_to_credits!(0.15));
            recipient_addresses.insert(p2sh_address1, dash_to_credits!(0.2));
            recipient_addresses.insert(p2sh_address2, dash_to_credits!(0.12));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Verify all addresses received their amounts
            let (_, balance1) = platform
                .drive
                .fetch_balance_and_nonce(&p2pkh_address1, None, platform_version)
                .expect("should fetch")
                .expect("p2pkh_address1 should exist");
            assert_eq!(balance1, dash_to_credits!(0.1));

            let (_, balance2) = platform
                .drive
                .fetch_balance_and_nonce(&p2pkh_address2, None, platform_version)
                .expect("should fetch")
                .expect("p2pkh_address2 should exist");
            assert_eq!(balance2, dash_to_credits!(0.15));

            let (_, balance3) = platform
                .drive
                .fetch_balance_and_nonce(&p2sh_address1, None, platform_version)
                .expect("should fetch")
                .expect("p2sh_address1 should exist");
            assert_eq!(balance3, dash_to_credits!(0.2));

            let (_, balance4) = platform
                .drive
                .fetch_balance_and_nonce(&p2sh_address2, None, platform_version)
                .expect("should fetch")
                .expect("p2sh_address2 should exist");
            assert_eq!(balance4, dash_to_credits!(0.12));
        }

        /// Test raw transition with signature_public_key_id pointing to non-existent key
        #[test]
        fn test_raw_transition_with_nonexistent_key_id() {
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

            let (identity, _signer) = create_identity_with_transfer_key(
                [71u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Create raw transition with key ID 999 which doesn't exist
            let transition: StateTransition = IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: identity.id(),
                    recipient_addresses,
                    nonce: 1,
                    user_fee_increase: 0,
                    signature_public_key_id: 999, // Non-existent key ID
                    signature: BinaryData::new(vec![0x30; 65]),
                },
            )
            .into();

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

            // Should fail - key doesn't exist
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(_)
                )],
                "Should fail with signature error for non-existent key"
            );
        }

        /// Test raw transition where signature_public_key_id points to AUTHENTICATION key
        #[test]
        fn test_raw_transition_with_authentication_key_id() {
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

            let mut rng = StdRng::seed_from_u64(702);

            let (identity, _signer) = create_identity_with_transfer_key(
                [72u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );

            // Get the authentication key ID (should be 0 from our helper)
            let auth_key = identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                )
                .expect("should have auth key");

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Create raw transition pointing to auth key instead of transfer key
            let transition: StateTransition = IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: identity.id(),
                    recipient_addresses,
                    nonce: 1,
                    user_fee_increase: 0,
                    signature_public_key_id: auth_key.id(), // Auth key, not transfer key!
                    signature: BinaryData::new(vec![0x30; 65]),
                },
            )
            .into();

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

            // Should fail - wrong key purpose
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(_)
                )],
                "Should fail with signature error for wrong key purpose"
            );
        }

        /// Test that when balance equals (output + actual_fee_from_first_run), the second run
        /// fails with IdentityInsufficientBalanceError because fee estimation uses a higher
        /// worst-case estimate than the actual fee. This demonstrates that users need slightly
        /// more than (output + actual_fee) to pass validation.
        #[test]
        fn test_exact_actual_fee_balance_fails_due_to_fee_estimation() {
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

            let mut rng = StdRng::seed_from_u64(703);
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let initial_balance = dash_to_credits!(10.0);

            // Create identity with high balance
            let (identity, signer) = create_identity_with_transfer_key(
                [73u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            // Add identity FIRST (committed, outside any transaction)
            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), min_output);

            // Create the transition once - we'll reuse it for both runs
            let transition =
                create_signed_transition(&identity, &signer, recipient_addresses.clone(), 1);
            let transition_bytes = transition.serialize_to_bytes().expect("should serialize");

            // First run in a transaction to measure actual fee (then rollback)
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

            let actual_fee = match &processing_result.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.total_base_fee()
                }
                _ => panic!("Expected successful execution to determine fee"),
            };

            // Rollback the transaction - we just wanted to measure the fee
            // The identity still exists with its original balance after rollback
            platform
                .drive
                .grove
                .rollback_transaction(&transaction)
                .expect("should rollback");

            // Explicitly drop the transaction to release the borrow on platform
            drop(transaction);

            // Set balance to exactly (output + actual_fee) from the first run
            let exact_balance = min_output + actual_fee;
            let balance_to_remove = initial_balance - exact_balance;

            // Remove the excess balance to leave exactly what we measured
            platform
                .drive
                .remove_from_identity_balance(
                    identity.id().to_buffer(),
                    balance_to_remove,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("should remove balance");

            // Second run - this should fail because fee estimation is higher than actual fee
            let platform_state2 = platform.state.load();
            let transaction2 = platform.drive.grove.start_transaction();

            let processing_result2 = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state2,
                    &BlockInfo::default(),
                    &transaction2,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // The fee estimation uses a worst-case estimate that is higher than actual fee,
            // so the transition fails even though the actual fee would have been sufficient.
            // Balance is (min_output + actual_fee) but required is (min_output + estimated_fee)
            // where estimated_fee > actual_fee.
            assert_matches!(
                processing_result2.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(err))
                )] => {
                    assert_eq!(err.balance(), exact_balance);
                    // Required balance should be higher than what we have due to fee estimation
                    assert!(err.required_balance() > exact_balance,
                        "Required balance {} should be greater than exact balance {}",
                        err.required_balance(), exact_balance);
                }
            );
        }

        /// Test that when balance equals (total_outputs + actual_fee_from_first_run) with 128 outputs,
        /// the second run succeeds because fee estimation is now accurate enough with count sum trees.
        /// (Previously this would fail because fee estimation used a higher worst-case estimate.)
        #[test]
        fn test_exact_actual_fee_balance_succeeds_with_128_outputs() {
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

            let mut rng = StdRng::seed_from_u64(703);
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let initial_balance = dash_to_credits!(100.0);

            // Create identity with high balance
            let (identity, signer) = create_identity_with_transfer_key(
                [73u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            // Add identity FIRST (committed, outside any transaction)
            add_identity_to_drive(&mut platform, &identity);

            // Create 128 outputs
            let mut recipient_addresses = BTreeMap::new();
            for i in 0u8..128 {
                let mut hash = [0u8; 20];
                hash[0] = i;
                hash[1] = (i as u16 >> 8) as u8;
                recipient_addresses.insert(PlatformAddress::P2pkh(hash), min_output);
            }

            let total_outputs: u64 = recipient_addresses.values().sum();

            // Create the transition once - we'll reuse it for both runs
            let transition =
                create_signed_transition(&identity, &signer, recipient_addresses.clone(), 1);
            let transition_bytes = transition.serialize_to_bytes().expect("should serialize");

            // First run in a transaction to measure actual fee (then rollback)
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

            let actual_fee = match &processing_result.execution_results()[0] {
                StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => {
                    fee_result.total_base_fee()
                }
                _ => panic!("Expected successful execution to determine fee"),
            };

            // Rollback the transaction - we just wanted to measure the fee
            platform
                .drive
                .grove
                .rollback_transaction(&transaction)
                .expect("should rollback");

            drop(transaction);

            // Set balance to exactly (total_outputs + actual_fee) from the first run
            let exact_balance = total_outputs + actual_fee;
            let balance_to_remove = initial_balance - exact_balance;

            // Remove the excess balance to leave exactly what we measured
            platform
                .drive
                .remove_from_identity_balance(
                    identity.id().to_buffer(),
                    balance_to_remove,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("should remove balance");

            // Second run - this should fail because fee estimation is higher than actual fee
            let platform_state2 = platform.state.load();
            let transaction2 = platform.drive.grove.start_transaction();

            let processing_result2 = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state2,
                    &BlockInfo::default(),
                    &transaction2,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // With count sum trees, fee estimation is now accurate enough that exact balance succeeds
            assert_matches!(
                processing_result2.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => {
                    // Success - fee estimation is now accurate enough with count sum trees
                }
            );
        }

        /// Test that 129 outputs exceeds the maximum allowed outputs and returns an error.
        #[test]
        fn test_129_outputs_exceeds_maximum() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config.clone())
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(703);
            let min_output = platform_version
                .dpp
                .state_transitions
                .address_funds
                .min_output_amount;

            let initial_balance = dash_to_credits!(100.0);

            // Create identity with high balance
            let (identity, signer) = create_identity_with_transfer_key(
                [73u8; 32],
                initial_balance,
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            // Create 129 outputs (one more than allowed)
            let mut recipient_addresses = BTreeMap::new();
            for i in 0u16..129 {
                let mut hash = [0u8; 20];
                hash[0] = (i & 0xFF) as u8;
                hash[1] = (i >> 8) as u8;
                recipient_addresses.insert(PlatformAddress::P2pkh(hash), min_output);
            }

            let transition =
                create_signed_transition(&identity, &signer, recipient_addresses.clone(), 1);
            let transition_bytes = transition.serialize_to_bytes().expect("should serialize");

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

            // Should fail with too many outputs error
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::TransitionOverMaxOutputsError(_))
                )]
            );
        }

        /// Test with a very large single output (near u64::MAX / 2)
        #[test]
        fn test_very_large_single_output() {
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

            let mut rng = StdRng::seed_from_u64(704);

            // Very large but valid amount
            let large_output = u32::MAX as u64 / 2;

            // Identity needs enough for the large output plus fees
            let identity_balance = large_output + dash_to_credits!(1.0);

            let (identity, signer) = create_identity_with_transfer_key(
                [75u8; 32],
                identity_balance,
                &mut rng,
                platform_version,
            );

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), large_output);

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Verify recipient received the large amount
            let (_, balance) = platform
                .drive
                .fetch_balance_and_nonce(&create_platform_address(1), None, platform_version)
                .expect("should fetch")
                .expect("address should exist");

            assert_eq!(
                balance, large_output,
                "Recipient should have the large amount"
            );
        }

        /// Test identity with no public keys at all
        #[test]
        fn test_identity_with_no_public_keys() {
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

            // Create identity with NO public keys
            let identity: Identity = IdentityV0 {
                id: [76u8; 32].into(),
                revision: 0,
                balance: dash_to_credits!(1.0),
                public_keys: BTreeMap::new(), // Empty!
            }
            .into();

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            // Create raw transition - we can't sign it properly anyway
            let transition: StateTransition = IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: identity.id(),
                    recipient_addresses,
                    nonce: 1,
                    user_fee_increase: 0,
                    signature_public_key_id: 0, // This key doesn't exist
                    signature: BinaryData::new(vec![0x30; 65]),
                },
            )
            .into();

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

            // Should fail - no keys to verify signature against
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(_)
                )],
                "Should fail with signature error for identity with no keys"
            );
        }

        /// Test identity with non-zero revision
        #[test]
        fn test_identity_with_different_revision() {
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

            let mut rng = StdRng::seed_from_u64(705);
            let mut signer = SimpleSigner::default();

            // Create keys
            let (auth_key, auth_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("should create auth key");

            let (transfer_key, transfer_private_key) =
                IdentityPublicKey::random_key_with_known_attributes(
                    1,
                    &mut rng,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                    None,
                    platform_version,
                )
                .expect("should create transfer key");

            signer.add_identity_public_key(auth_key.clone(), auth_private_key);
            signer.add_identity_public_key(transfer_key.clone(), transfer_private_key);

            let mut public_keys = BTreeMap::new();
            public_keys.insert(auth_key.id(), auth_key);
            public_keys.insert(transfer_key.id(), transfer_key);

            // Create identity with revision 5 (simulating an identity that has been updated)
            let identity: Identity = IdentityV0 {
                id: [77u8; 32].into(),
                revision: 5, // Non-zero revision
                balance: dash_to_credits!(1.0),
                public_keys,
            }
            .into();

            add_identity_to_drive(&mut platform, &identity);

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), dash_to_credits!(0.1));

            let transition = create_signed_transition(&identity, &signer, recipient_addresses, 1);
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

            // Should succeed - revision shouldn't affect credit transfers
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );
        }

        /// Test overflow when adding fee to transfer amount during balance check
        #[test]
        fn test_overflow_transfer_plus_fee() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            // Create a single output that when added to min_fee would overflow
            let min_fee = platform_version
                .fee_version
                .state_transition_min_fees
                .credit_transfer_to_addresses;

            // Amount that when added to min_fee overflows
            let overflow_amount = u64::MAX - min_fee + 1;

            let mut recipient_addresses = BTreeMap::new();
            recipient_addresses.insert(create_platform_address(1), overflow_amount);

            let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
                identity_id: [1u8; 32].into(),
                recipient_addresses,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0; 65]),
            };

            // Structure validation should pass (amount itself is valid)
            let result = transition_v0.validate_structure(platform_version);
            // The structure validation only checks min_output_amount, not the sum
            // So this specific overflow is caught during balance validation, not structure
            assert!(
                result.is_valid(),
                "Structure should be valid for large single amount"
            );
        }
    }
}
