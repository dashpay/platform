mod action_validation;
mod advanced_structure;
mod data_triggers;
mod identity_contract_nonce;
mod state;
mod transformer;

use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::identity::PartialIdentity;
use dpp::prelude::*;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::documents_batch::advanced_structure::v0::DocumentsBatchStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::documents_batch::identity_contract_nonce::v0::DocumentsBatchStateTransitionIdentityContractNonceV0;
use crate::execution::validation::state_transition::documents_batch::state::v0::DocumentsBatchStateTransitionStateValidationV0;

use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionNonceValidationV0,
    StateTransitionStateValidationV0, StateTransitionStructureKnownInStateValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;

impl ValidationMode {
    /// Returns a bool on whether we should validate that documents are valid against the state
    pub fn should_validate_document_valid_against_state(&self) -> bool {
        match self {
            ValidationMode::CheckTx => false,
            ValidationMode::RecheckTx => false,
            ValidationMode::Validator => true,
            ValidationMode::NoValidation => false,
        }
    }
}

impl StateTransitionActionTransformerV0 for DocumentsBatchTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(&platform.into(), block_info, validation_mode, tx),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for DocumentsBatchTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .basic_structure
        {
            0 => {
                // There is nothing expensive here
                self.validate_base_structure(platform_version)
                    .map_err(Error::Protocol)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: base structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionNonceValidationV0 for DocumentsBatchTransition {
    fn validate_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .revision
        {
            0 => self.validate_identity_contract_nonces_v0(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: revision".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStructureKnownInStateValidationV0 for DocumentsBatchTransition {
    fn validate_advanced_structure_from_state(
        &self,
        action: &StateTransitionAction,
        identity: Option<&PartialIdentity>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .advanced_structure
        {
            0 => {
                let identity =
                    identity.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "The identity must be known on advanced structure validation",
                    )))?;
                let StateTransitionAction::DocumentsBatchAction(documents_batch_transition_action) =
                    action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a documents batch transition action",
                    )));
                };
                self.validate_advanced_structure_from_state_v0(
                    documents_batch_transition_action,
                    identity,
                    execution_context,
                    platform_version,
                )
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: advanced structure from state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn has_advanced_structure_validation_with_state(&self) -> bool {
        true
    }

    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool {
        true
    }
}

impl StateTransitionStateValidationV0 for DocumentsBatchTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .state
        {
            0 => {
                let action =
                    action.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "documents batch structure validation should have an action",
                    )))?;
                let StateTransitionAction::DocumentsBatchAction(documents_batch_transition_action) =
                    action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a documents batch transition action",
                    )));
                };
                self.validate_state_v0(
                    documents_batch_transition_action,
                    &platform.into(),
                    epoch,
                    execution_context,
                    tx,
                    platform_version,
                )
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::document::transfer::Transferable;
    use dpp::document::DocumentV0Setters;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
    use dpp::platform_value::{Bytes32, Value};
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::query::DriveQuery;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    fn setup_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
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

        signer.add_key(master_key.clone(), master_private_key.clone());

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_key(critical_public_key.clone(), private_key.clone());

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
            ]),
            balance: 10000000000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

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

        (identity, signer, critical_public_key)
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        assert!(profile.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
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
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_update_transition =
            DocumentsBatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 5074230);
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_not_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let (other_identity, ..) = setup_identity(&mut platform, 495);

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .expect("expected a profile document type");

        assert!(!contact_request_document_type.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = contact_request_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set(
            "toUserId",
            Value::Identifier(other_identity.id().to_buffer()),
        );
        document.set("recipientKeyIndex", Value::U32(1));
        document.set("senderKeyIndex", Value::U32(1));
        document.set("accountReference", Value::U32(0));
        document.set("coreHeightCreatedAt", Value::U32(5));

        let mut altered_document = document.clone();

        altered_document.set_revision(Some(1));
        altered_document.set("senderKeyIndex", Value::U32(2));

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document,
                contact_request_document_type,
                entropy.0,
                &key,
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_update_transition =
            DocumentsBatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                contact_request_document_type,
                &key,
                3,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 102690);
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_not_mutable_but_is_transferable() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(435);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let (receiver, _, _) = setup_identity(&mut platform, 452);

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(
                query_sender_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(
                query_receiver_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        // We expect the sender to have 1 document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        document.set("attack", 6.into());
        document.set("defense", 0.into());

        let documents_batch_transfer_transition =
            DocumentsBatchTransition::new_document_replacement_transition_from_document(
                document,
                card_document_type,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1253950);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);
    }

    #[test]
    fn test_document_replace_that_does_not_yet_exist() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
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

        let documents_batch_update_transition =
            DocumentsBatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1244470);
    }

    #[test]
    fn test_document_delete_on_document_type_that_is_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        assert!(profile.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
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
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_deletion_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 5588830);
    }

    #[test]
    fn test_document_delete_on_document_type_that_is_not_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let (other_identity, ..) = setup_identity(&mut platform, 495);

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .expect("expected a profile document type");

        assert!(!contact_request_document_type.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = contact_request_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set(
            "toUserId",
            Value::Identifier(other_identity.id().to_buffer()),
        );
        document.set("recipientKeyIndex", Value::U32(1));
        document.set("senderKeyIndex", Value::U32(1));
        document.set("accountReference", Value::U32(0));
        document.set("coreHeightCreatedAt", Value::U32(5));

        let mut altered_document = document.clone();

        altered_document.set_revision(Some(1));
        altered_document.set("senderKeyIndex", Value::U32(2));

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document,
                contact_request_document_type,
                entropy.0,
                &key,
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_deletion_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                contact_request_document_type,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_deletion_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_deletion_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1507670);
    }

    #[test]
    fn test_document_delete_that_does_not_yet_exist() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
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

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_delete_serialized_transition = documents_batch_delete_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_delete_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1244470);
    }

    #[test]
    fn test_document_transfer_on_document_type_that_is_transferable() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let (receiver, _, _) = setup_identity(&mut platform, 450);

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(!card_document_type.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(
                query_sender_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(
                query_receiver_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        // We expect the sender to have 1 document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            DocumentsBatchTransition::new_document_transfer_transition_from_document(
                document,
                card_document_type,
                receiver.id(),
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 9350380);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to have no documents, and the receiver to have 1
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);
    }

    #[test]
    fn test_document_transfer_on_document_type_that_is_not_transferable() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game(Transferable::Never);

        let mut rng = StdRng::seed_from_u64(435);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let (receiver, _, _) = setup_identity(&mut platform, 452);

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(
                query_sender_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(
                query_receiver_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        // We expect the sender to have 1 document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            DocumentsBatchTransition::new_document_transfer_transition_from_document(
                document,
                card_document_type,
                receiver.id(),
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1253950);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);
    }

    #[test]
    fn test_document_transfer_that_does_not_yet_exist() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game(Transferable::Never);

        let mut rng = StdRng::seed_from_u64(435);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let (receiver, _, _) = setup_identity(&mut platform, 452);

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(
                query_sender_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(
                query_receiver_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        // We expect the sender to have 0 documents, and the receiver to also have none
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            DocumentsBatchTransition::new_document_transfer_transition_from_document(
                document,
                card_document_type,
                receiver.id(),
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 25090);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to still have no document, and the receiver to have none as well
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 0);
    }

    #[test]
    fn test_document_delete_after_transfer() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958);

        let (receiver, recipient_signer, recipient_key) = setup_identity(&mut platform, 450);

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(!card_document_type.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
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

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(
                query_sender_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(
                query_receiver_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        // We expect the sender to have 1 document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            DocumentsBatchTransition::new_document_transfer_transition_from_document(
                document.clone(),
                card_document_type,
                receiver.id(),
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default_with_time(50000000),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 10277100);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to have no documents, and the receiver to have 1
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);

        // Now let's try to delete the transferred document

        document.set_owner_id(receiver.id());

        let documents_batch_deletion_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                card_document_type,
                &recipient_key,
                2,
                0,
                &recipient_signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_deletion_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_deletion_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1107610);
    }
}
