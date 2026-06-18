mod nonce;
mod state;
mod structure;

use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_credit_transfer::state::v0::IdentityCreditTransferStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_credit_transfer::structure::v0::IdentityCreditTransferStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::PlatformStateV0Methods;

impl StateTransitionActionTransformer for IdentityCreditTransferTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        _validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_transfer_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit transfer transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityCreditTransferTransition {
    fn validate_basic_structure(
        &self,
        _network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_transfer_state_transition
            .basic_structure
        {
            Some(0) => {
                // There is nothing expensive here
                self.validate_basic_structure_v0()
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit transfer transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity credit transfer transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionStateValidation for IdentityCreditTransferTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        _block_info: &BlockInfo,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_transfer_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit transfer transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::signer::Signer;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::Identifier;
    use dpp::serialization::{PlatformSerializable, Signable};
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    /// Creates an identity with a master authentication key and a TRANSFER purpose key,
    /// adds it to the platform, and returns the identity, signer, and transfer key.
    fn setup_identity_with_transfer_key(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();
        let mut rng = StdRng::seed_from_u64(seed);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(critical_public_key.clone(), private_key);

        let (transfer_key, transfer_private_key) =
            IdentityPublicKey::random_key_with_known_attributes(
                2,
                &mut rng,
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
                KeyType::ECDSA_SECP256K1,
                None,
                platform_version,
            )
            .expect("expected to get transfer key pair");

        signer.add_identity_public_key(transfer_key.clone(), transfer_private_key);

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key),
                (1, critical_public_key),
                (2, transfer_key.clone()),
            ]),
            balance: credits,
            revision: 0,
        }
        .into();

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
            .expect("expected to add a new identity");

        (identity, signer, transfer_key)
    }

    async fn create_signed_transfer(
        identity_id: dpp::prelude::Identifier,
        recipient_id: dpp::prelude::Identifier,
        amount: u64,
        nonce: u64,
        signer: &simple_signer::signer::SimpleSigner,
        key: &dpp::identity::IdentityPublicKey,
    ) -> Vec<u8> {
        let transfer: IdentityCreditTransferTransition = IdentityCreditTransferTransitionV0 {
            identity_id,
            recipient_id,
            amount,
            nonce,
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut st: StateTransition = transfer.into();
        let data = st.signable_bytes().expect("expected signable bytes");
        st.set_signature(
            signer
                .sign(key, data.as_slice())
                .await
                .expect("expected to sign"),
        );
        st.serialize_to_bytes().expect("expected to serialize")
    }

    #[tokio::test]
    async fn test_identity_credit_transfer_to_self_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, transfer_key) =
            setup_identity_with_transfer_key(&mut platform, 500, dash_to_credits!(1.0));

        let platform_state = platform.state.load();

        // Try to transfer to self -- same identity_id and recipient_id
        let transfer_bytes = create_signed_transfer(
            identity.id(),
            identity.id(), // self-transfer
            200_000,
            1,
            &signer,
            &transfer_key,
        )
        .await;

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transfer_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_credit_transfer_below_minimum_amount_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, transfer_key) =
            setup_identity_with_transfer_key(&mut platform, 500, dash_to_credits!(1.0));

        let platform_state = platform.state.load();

        // Transfer amount below minimum (100_000)
        let transfer_bytes = create_signed_transfer(
            identity.id(),
            Identifier::random(), // any valid recipient
            99_999,               // below MIN_TRANSFER_AMOUNT of 100_000
            1,
            &signer,
            &transfer_key,
        )
        .await;

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transfer_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_credit_transfer_sender_not_found() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // Create an identity with a transfer key (will be used to sign, but the
        // transition will reference a non-existent sender identity_id)
        let (_existing_identity, signer, transfer_key) =
            setup_identity_with_transfer_key(&mut platform, 502, dash_to_credits!(1.0));

        let platform_state = platform.state.load();

        // Use a fake sender identity_id that doesn't exist in the platform
        let fake_sender_id = Identifier::random();
        let transfer: IdentityCreditTransferTransition = IdentityCreditTransferTransitionV0 {
            identity_id: fake_sender_id,
            recipient_id: Identifier::random(),
            amount: 200_000,
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: transfer_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut st: StateTransition = transfer.into();
        let data = st.signable_bytes().expect("signable bytes");
        st.set_signature(
            signer
                .sign(&transfer_key, data.as_slice())
                .await
                .expect("sign"),
        );
        let transfer_bytes = st.serialize_to_bytes().expect("serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transfer_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Sender identity not found should result in a consensus error
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::SignatureError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_credit_transfer_insufficient_balance() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // Create sender with very low balance
        let (sender, signer, transfer_key) =
            setup_identity_with_transfer_key(&mut platform, 600, dash_to_credits!(0.001));

        let (recipient, _, _) =
            setup_identity_with_transfer_key(&mut platform, 601, dash_to_credits!(0.5));

        let platform_state = platform.state.load();

        // Try to transfer more than the sender's balance
        let transfer_bytes = create_signed_transfer(
            sender.id(),
            recipient.id(),
            dash_to_credits!(1.0), // more than the 0.001 balance
            1,
            &signer,
            &transfer_key,
        )
        .await;

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transfer_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Insufficient balance is caught by the balance pre-check which returns
        // an unpaid consensus error since the identity can't afford processing fees
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_credit_transfer_recipient_not_found() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (sender, signer, transfer_key) =
            setup_identity_with_transfer_key(&mut platform, 700, dash_to_credits!(1.0));

        let platform_state = platform.state.load();

        // Transfer to a non-existent recipient
        let fake_recipient_id = Identifier::random();
        let transfer_bytes = create_signed_transfer(
            sender.id(),
            fake_recipient_id,
            200_000,
            1,
            &signer,
            &transfer_key,
        )
        .await;

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transfer_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Recipient not found returns IdentityNotFoundError (a SignatureError variant)
        // as an unpaid consensus error since the state validation doesn't produce
        // an execution event on failure
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::SignatureError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_credit_transfer_successful() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (sender, signer, transfer_key) =
            setup_identity_with_transfer_key(&mut platform, 800, dash_to_credits!(1.0));

        let (recipient, _, _) =
            setup_identity_with_transfer_key(&mut platform, 801, dash_to_credits!(0.5));

        let platform_state = platform.state.load();

        let transfer_amount = 500_000u64;
        let transfer_bytes = create_signed_transfer(
            sender.id(),
            recipient.id(),
            transfer_amount,
            1,
            &signer,
            &transfer_key,
        )
        .await;

        // CheckTx root-invariance guard (devnet paloma h788): `check_tx` asserts under
        // cfg(test) that it never mutates committed grovedb state, so running the canonical
        // valid fixture through it pins the invariant for this transition type.
        crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
            &platform,
            &transfer_bytes,
            "identity credit transfer",
        );

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transfer_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Verify recipient balance increased
        let recipient_balance = platform
            .drive
            .fetch_identity_balance(recipient.id().into_buffer(), None, platform_version)
            .expect("expected to get balance")
            .expect("expected balance to exist");

        assert!(recipient_balance > dash_to_credits!(0.5));
    }
}
