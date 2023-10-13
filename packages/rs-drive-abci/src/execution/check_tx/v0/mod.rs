use crate::error::Error;
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
#[cfg(test)]
use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult;
#[cfg(test)]
use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult::ConsensusExecutionError;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::fee::fee_result::FeeResult;
use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::StateTransition;
#[cfg(test)]
use dpp::validation::SimpleConsensusValidationResult;
use dpp::validation::ValidationResult;
#[cfg(test)]
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[cfg(test)]
    pub(in crate::execution) fn execute_tx(
        &self,
        raw_tx: Vec<u8>,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<StateTransitionExecutionResult, Error> {
        let state_transition =
            StateTransition::deserialize_from_bytes(raw_tx.as_slice()).map_err(Error::Protocol)?;
        let state_read_guard = self.state.read().unwrap();
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: &state_read_guard,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };

        let state_transition_execution_event =
            process_state_transition(&platform_ref, state_transition, Some(transaction))?;

        if state_transition_execution_event.is_valid() {
            let platform_version = platform_ref.state.current_platform_version()?;
            let execution_event = state_transition_execution_event.into_data()?;
            self.execute_event(execution_event, block_info, transaction, platform_version)
        } else {
            Ok(ConsensusExecutionError(
                SimpleConsensusValidationResult::new_with_errors(
                    state_transition_execution_event.errors,
                ),
            ))
        }
    }

    /// Checks a state transition to determine if it should be added to the mempool.
    ///
    /// This function performs a few checks, including validating the state transition and ensuring that the
    /// user can pay for it. It may be inaccurate in rare cases, so the proposer needs to re-check transactions
    /// before proposing a block.
    ///
    /// # Arguments
    ///
    /// * `raw_tx` - A raw transaction represented as a vector of bytes.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<FeeResult, ConsensusError>, Error>` - If the state transition passes all
    ///   checks, it returns a `ValidationResult` with fee information. If any check fails, it returns an `Error`.
    pub(super) fn check_tx_v0(
        &self,
        raw_tx: &[u8],
    ) -> Result<ValidationResult<FeeResult, ConsensusError>, Error> {
        let state_transition = match StateTransition::deserialize_from_bytes(raw_tx) {
            Ok(state_transition) => state_transition,
            Err(err) => {
                return Ok(ValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                        SerializedObjectParsingError::new(err.to_string()),
                    )),
                ))
            }
        };
        let state_read_guard = self.state.read().unwrap();
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: &state_read_guard,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };
        let execution_event = process_state_transition(&platform_ref, state_transition, None)?;

        let platform_version = platform_ref.state.current_platform_version()?;

        // We should run the execution event in dry run to see if we would have enough fees for the transaction

        // We need the approximate block info
        if let Some(block_info) = state_read_guard.last_committed_block_info().as_ref() {
            // We do not put the transaction, because this event happens outside of a block
            execution_event.and_then_borrowed_validation(|execution_event| {
                self.validate_fees_of_event(
                    execution_event,
                    block_info.basic_info(),
                    None,
                    platform_version,
                )
            })
        } else {
            execution_event.and_then_borrowed_validation(|execution_event| {
                self.validate_fees_of_event(
                    execution_event,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult::SuccessfulPaidExecution;
    use crate::platform_types::system_identity_public_keys::v0::SystemIdentityPublicKeysV0;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::signature::SignatureError;
    use simple_signer::signer::SimpleSigner;

    use dpp::consensus::ConsensusError;
    use dpp::dashcore::secp256k1::Secp256k1;
    use dpp::dashcore::{key::KeyPair, signer, Network, PrivateKey};

    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contracts::dpns_contract;
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::document::DocumentV0Setters;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};

    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::identity::{Identity, IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::{Identifier, IdentityPublicKey};
    use dpp::serialization::{PlatformSerializable, Signable};

    use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
    use dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use dpp::state_transition::{StateTransition, StateTransitionLike};
    use dpp::tests::fixtures::{get_dashpay_contract_fixture, instant_asset_lock_proof_fixture};
    use dpp::version::PlatformVersion;
    use dpp::NativeBlsModule;

    use dpp::identity::contract_bounds::ContractBounds::SingleContractDocumentType;
    use dpp::platform_value::Bytes32;
    use dpp::system_data_contracts::dashpay_contract;
    use dpp::system_data_contracts::SystemDataContract::Dashpay;
    use platform_version::TryIntoPlatformVersioned;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    // This test needs to be finished, but is still useful for debugging
    #[test]
    #[ignore]
    fn verify_check_tx_on_data_contract_create() {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();

        let state = platform.state.read().unwrap();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version = PlatformVersion::get(protocol_version).unwrap();

        let tx: Vec<u8> = vec![
            0, 0, 0, 104, 37, 39, 102, 34, 99, 205, 58, 189, 155, 27, 93, 128, 49, 86, 24, 164, 86,
            171, 102, 203, 151, 25, 88, 2, 9, 48, 215, 150, 16, 127, 114, 0, 0, 0, 0, 0, 1, 0, 0,
            1, 14, 28, 76, 152, 45, 91, 51, 175, 52, 203, 177, 193, 171, 28, 8, 215, 207, 185, 149,
            86, 192, 251, 146, 195, 126, 232, 156, 190, 183, 97, 59, 20, 0, 1, 4, 110, 111, 116,
            101, 22, 3, 18, 4, 116, 121, 112, 101, 18, 6, 111, 98, 106, 101, 99, 116, 18, 10, 112,
            114, 111, 112, 101, 114, 116, 105, 101, 115, 22, 1, 18, 7, 109, 101, 115, 115, 97, 103,
            101, 22, 1, 18, 4, 116, 121, 112, 101, 18, 6, 115, 116, 114, 105, 110, 103, 18, 20, 97,
            100, 100, 105, 116, 105, 111, 110, 97, 108, 80, 114, 111, 112, 101, 114, 116, 105, 101,
            115, 19, 0, 116, 200, 180, 23, 82, 251, 127, 70, 3, 242, 82, 189, 127, 198, 107, 151,
            75, 27, 64, 150, 39, 22, 110, 95, 101, 7, 155, 2, 98, 160, 95, 223, 2, 65, 32, 202, 64,
            174, 15, 169, 140, 53, 129, 120, 106, 230, 25, 0, 70, 207, 222, 171, 52, 147, 135, 100,
            195, 27, 202, 108, 185, 188, 243, 196, 149, 82, 46, 55, 224, 244, 182, 158, 107, 149,
            217, 221, 43, 251, 104, 84, 78, 35, 20, 237, 188, 237, 240, 216, 62, 79, 208, 96, 149,
            116, 62, 82, 187, 135, 219,
        ];
        let state_transitions = StateTransition::deserialize_many(&vec![tx.clone()])
            .expect("expected a state transition");
        let state_transition = state_transitions.first().unwrap();
        let StateTransition::DataContractCreate(contract_create) = state_transition else {
            panic!("expecting a data contract create");
        };

        let identifier = contract_create.owner_id();

        let mut identity =
            Identity::random_identity(3, Some(50), platform_version).expect("got an identity");

        identity.set_id(identifier);

        platform
            .drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create structure");

        platform
            .drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity");

        let transaction = platform.drive.grove.start_transaction();

        let check_result = platform.check_tx(&tx).expect("expected to check tx");

        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![tx],
                &state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");
    }

    #[test]
    fn data_contract_create_check_tx() {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();
        let state = platform.state.read().unwrap();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version = PlatformVersion::get(protocol_version).unwrap();

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(1),
            platform_version,
        )
        .expect("expected to get key pair");

        platform
            .drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create state structure");
        let identity: Identity = IdentityV0 {
            id: Identifier::new([
                158, 113, 180, 126, 91, 83, 62, 44, 83, 54, 97, 88, 240, 215, 84, 139, 167, 156,
                166, 203, 222, 4, 64, 31, 215, 199, 149, 151, 190, 246, 251, 44,
            ]),
            public_keys: BTreeMap::from([(1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let dashpay = get_dashpay_contract_fixture(Some(identity.id()), protocol_version);
        let mut create_contract_state_transition: StateTransition = dashpay
            .try_into_platform_versioned(platform_version)
            .expect("expected a state transition");
        create_contract_state_transition
            .sign(&key, private_key.as_slice(), &NativeBlsModule)
            .expect("expected to sign transition");
        let serialized = create_contract_state_transition
            .serialize_to_bytes()
            .expect("serialized state transition");
        platform
            .drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity");

        let validation_result = platform
            .check_tx(serialized.as_slice())
            .expect("expected to check tx");

        assert!(validation_result.errors.is_empty());
    }

    #[test]
    fn document_update_check_tx() {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();

        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version().unwrap();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(19),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_key(key.clone(), private_key.clone());

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                identity.clone(),
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                platform_version,
            )
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let dashpay =
            get_dashpay_contract_fixture(Some(identity.id()), platform_version.protocol_version);
        let dashpay_contract = dashpay.data_contract().clone();
        let mut create_contract_state_transition: StateTransition = dashpay
            .try_into_platform_versioned(platform_version)
            .expect("expected a state transition");
        create_contract_state_transition
            .sign(&key, private_key.as_slice(), &NativeBlsModule)
            .expect("expected to sign transition");
        let data_contract_create_serialized_transition = create_contract_state_transition
            .serialize_to_bytes()
            .expect("expected data contract create serialized state transition");

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identifier,
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition =
            DocumentsBatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        platform
            .drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create state structure");

        let transaction = platform.drive.grove.start_transaction();

        let validation_result = platform
            .execute_tx(
                identity_create_serialized_transition,
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute identity_create tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        let validation_result = platform
            .execute_tx(
                data_contract_create_serialized_transition,
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute data_contract_create tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));
        let validation_result = platform
            .execute_tx(
                documents_batch_create_serialized_transition,
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute document_create tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let validation_result = platform
            .check_tx(documents_batch_update_serialized_transition.as_slice())
            .expect("expected to check tx");

        assert!(validation_result.errors.is_empty());
    }

    #[test]
    fn identity_top_up_check_tx() {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();

        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version().unwrap();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(19),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_key(key.clone(), private_key.clone());

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                identity.clone(),
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                platform_version,
            )
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        platform
            .drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create state structure");

        let transaction = platform.drive.grove.start_transaction();

        let validation_result = platform
            .execute_tx(
                identity_create_serialized_transition,
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute identity_create tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof_top_up = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identity_top_up_transition: StateTransition =
            IdentityTopUpTransition::try_from_identity(
                identity.clone(),
                asset_lock_proof_top_up,
                pk.as_slice(),
                &NativeBlsModule,
                platform_version,
                None,
            )
            .expect("expected an identity create transition");

        let identity_top_up_serialized_transition = identity_top_up_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let validation_result = platform
            .check_tx(identity_top_up_serialized_transition.as_slice())
            .expect("expected to check tx");

        assert!(validation_result.errors.is_empty());

        let transaction = platform.drive.grove.start_transaction();

        let validation_result = platform
            .execute_tx(
                identity_top_up_serialized_transition,
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute identity top up tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn identity_cant_double_top_up() {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();

        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version().unwrap();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(19),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_key(key.clone(), private_key.clone());

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                identity.clone(),
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                platform_version,
            )
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        platform
            .drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create state structure");

        let transaction = platform.drive.grove.start_transaction();

        let validation_result = platform
            .execute_tx(
                identity_create_serialized_transition,
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute identity_create tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof_top_up = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identity_top_up_transition: StateTransition =
            IdentityTopUpTransition::try_from_identity(
                identity.clone(),
                asset_lock_proof_top_up,
                pk.as_slice(),
                &NativeBlsModule,
                platform_version,
                None,
            )
            .expect("expected an identity create transition");

        let identity_top_up_serialized_transition = identity_top_up_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let validation_result = platform
            .check_tx(identity_top_up_serialized_transition.as_slice())
            .expect("expected to check tx");

        assert!(validation_result.errors.is_empty());

        let transaction = platform.drive.grove.start_transaction();

        let validation_result = platform
            .execute_tx(
                identity_top_up_serialized_transition.clone(),
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute identity top up tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let validation_result = platform
            .check_tx(identity_top_up_serialized_transition.as_slice())
            .expect("expected to check tx");

        assert!(matches!(
            validation_result.errors.first().expect("expected an error"),
            ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutPointAlreadyExistsError(_)
            )
        ));
    }

    #[test]
    fn identity_top_up_with_unknown_identity_doesnt_panic() {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();

        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version().unwrap();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(19),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_key(key.clone(), private_key.clone());

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        platform
            .drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create state structure");

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof_top_up = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identity_top_up_transition: StateTransition =
            IdentityTopUpTransition::try_from_identity(
                identity.clone(),
                asset_lock_proof_top_up,
                pk.as_slice(),
                &NativeBlsModule,
                platform_version,
                None,
            )
            .expect("expected an identity create transition");

        let identity_top_up_serialized_transition = identity_top_up_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let validation_result = platform
            .check_tx(identity_top_up_serialized_transition.as_slice())
            .expect("expected to check tx");

        // This errors because we never created the identity

        assert!(matches!(
            validation_result.errors.first().expect("expected an error"),
            ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(_))
        ));
    }

    #[test]
    fn identity_cant_create_with_used_outpoint() {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();

        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version().unwrap();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(19),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_key(key.clone(), private_key.clone());

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                identity.clone(),
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                platform_version,
            )
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        platform
            .drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create state structure");

        let transaction = platform.drive.grove.start_transaction();

        let validation_result = platform
            .execute_tx(
                identity_create_serialized_transition,
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute identity_create tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof_top_up = instant_asset_lock_proof_fixture(Some(
            PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap(),
        ));

        let identity_top_up_transition: StateTransition =
            IdentityTopUpTransition::try_from_identity(
                identity.clone(),
                asset_lock_proof_top_up.clone(),
                pk.as_slice(),
                &NativeBlsModule,
                platform_version,
                None,
            )
            .expect("expected an identity create transition");

        let identity_top_up_serialized_transition = identity_top_up_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let validation_result = platform
            .check_tx(identity_top_up_serialized_transition.as_slice())
            .expect("expected to check tx");

        assert!(validation_result.errors.is_empty());

        let transaction = platform.drive.grove.start_transaction();

        let validation_result = platform
            .execute_tx(
                identity_top_up_serialized_transition.clone(),
                &BlockInfo::default(),
                &transaction,
            )
            .expect("expected to execute identity top up tx");
        assert!(matches!(validation_result, SuccessfulPaidExecution(..)));

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // At this point we try creating a new identity with a used asset lock

        let mut signer = SimpleSigner::default();

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(50),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_key(key.clone(), private_key.clone());

        let identifier = asset_lock_proof_top_up
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                identity,
                asset_lock_proof_top_up,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                platform_version,
            )
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let validation_result = platform
            .check_tx(identity_create_serialized_transition.as_slice())
            .expect("expected to check tx");

        assert!(matches!(
            validation_result.errors.first().expect("expected an error"),
            ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutPointAlreadyExistsError(_)
            )
        ));
    }

    #[test]
    fn identity_update_with_non_master_key_check_tx() {
        let mut config = PlatformConfig::default();

        let mut rng = StdRng::seed_from_u64(1);

        let secp = Secp256k1::new();

        let master_key_pair = KeyPair::new(&secp, &mut rng);

        let _master_secret_key = master_key_pair.secret_key();

        let master_public_key = master_key_pair.public_key();

        config.abci.keys.dpns_master_public_key = master_public_key.serialize().to_vec();

        let high_key_pair = KeyPair::new(&secp, &mut rng);

        let high_secret_key = high_key_pair.secret_key();

        let high_public_key = high_key_pair.public_key();

        config.abci.keys.dpns_second_public_key = high_public_key.serialize().to_vec();

        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_mock_rpc();

        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version().unwrap();

        let genesis_time = 0;

        let system_identity_public_keys_v0: SystemIdentityPublicKeysV0 =
            platform.config.abci.keys.clone().into();

        platform
            .create_genesis_state(
                genesis_time,
                system_identity_public_keys_v0.into(),
                None,
                platform_version,
            )
            .expect("expected to create genesis state");

        let new_key_pair = KeyPair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };

        let signable_bytes = new_key
            .signable_bytes()
            .expect("expected to get signable bytes");
        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: dpns_contract::OWNER_ID_BYTES.into(),
            revision: 0,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            public_keys_disabled_at: None,
            signature_public_key_id: 1,
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let signature = signer::sign(
            &update_transition
                .signable_bytes()
                .expect("expected signable bytes"),
            &high_secret_key.secret_bytes(),
        )
        .expect("expected to sign");

        update_transition.set_signature(signature.to_vec().into());

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let validation_result = platform
            .check_tx(update_transition_bytes.as_slice())
            .expect("expected to execute identity top up tx");

        // Only master keys can sign an update

        validation_result.errors.first().expect("expected an error");
    }

    #[test]
    fn identity_update_with_encryption_key_check_tx() {
        let mut config = PlatformConfig::default();

        let mut rng = StdRng::seed_from_u64(1);

        let secp = Secp256k1::new();

        let master_key_pair = KeyPair::new(&secp, &mut rng);

        let master_secret_key = master_key_pair.secret_key();

        let master_public_key = master_key_pair.public_key();

        config.abci.keys.dashpay_master_public_key = master_public_key.serialize().to_vec();

        let high_key_pair = KeyPair::new(&secp, &mut rng);

        let high_secret_key = high_key_pair.secret_key();

        let high_public_key = high_key_pair.public_key();

        config.abci.keys.dashpay_second_public_key = high_public_key.serialize().to_vec();

        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_mock_rpc();

        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version().unwrap();

        let genesis_time = 0;

        let system_identity_public_keys_v0: SystemIdentityPublicKeysV0 =
            platform.config.abci.keys.clone().into();

        platform
            .create_genesis_state(
                genesis_time,
                system_identity_public_keys_v0.into(),
                None,
                platform_version,
            )
            .expect("expected to create genesis state");

        let new_key_pair = KeyPair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: ECDSA_SECP256K1,
            read_only: true,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(SingleContractDocumentType {
                id: Dashpay.id(),
                document_type_name: "contactRequest".to_string(),
            }),
        };

        let signable_bytes = new_key
            .signable_bytes()
            .expect("expected to get signable bytes");

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: dashpay_contract::OWNER_ID_BYTES.into(),
            revision: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            public_keys_disabled_at: None,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: dashpay_contract::OWNER_ID_BYTES.into(),
            revision: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            public_keys_disabled_at: None,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let signature = signer::sign(&signable_bytes, &master_secret_key.secret_bytes())
            .expect("expected to sign");

        update_transition.set_signature(signature.to_vec().into());

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let validation_result = platform
            .check_tx(update_transition_bytes.as_slice())
            .expect("expected to execute identity top up tx");

        // we won't have enough funds

        validation_result.errors.first().expect("expected an error");
    }
}
