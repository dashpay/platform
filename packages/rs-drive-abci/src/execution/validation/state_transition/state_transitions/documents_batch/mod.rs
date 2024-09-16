mod action_validation;
mod advanced_structure;
mod balance;
mod data_triggers;
mod identity_contract_nonce;
mod is_allowed;
mod state;
mod transformer;

use dpp::block::block_info::BlockInfo;
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
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

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
        block_info: &BlockInfo,
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
                    block_info,
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
    use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::document::transfer::Transferable;
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::fee::fee_result::BalanceChange;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::nft::TradeMode;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::{Bytes32, Value};
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
    use dpp::tests::json_document::json_document_to_contract;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::drive::document::query::QueryDocumentsWithFlagsOutcomeV0Methods;
    use drive::query::DriveDocumentQuery;
    use drive::util::storage_flags::StorageFlags;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;

    mod creation_tests {
        use rand::Rng;
        use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse};
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
        use super::*;
        use assert_matches::assert_matches;
        use rand::distributions::Standard;
        use dpp::consensus::basic::document::DocumentFieldMaxSizeExceededError;
        use dpp::consensus::ConsensusError;
        use dpp::consensus::basic::BasicError;
        use dpp::fee::fee_result::refunds::FeeRefunds;
        use dpp::fee::fee_result::FeeResult;
        use dpp::data_contract::accessors::v0::DataContractV0Setters;
        use dpp::data_contract::document_type::restricted_creation::CreationRestrictionMode;
        use dpp::document::Document;
        use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        use dpp::util::hash::hash_double;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
        use drive::util::object_size_info::DataContractResolvedInfo;
        use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
        use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally;
        use drive::query::vote_poll_vote_state_query::ResolvedContestedDocumentVotePollDriveQuery;
        use drive::util::test_helpers::setup_contract;
        use crate::execution::validation::state_transition::state_transitions::tests::{add_contender_to_dpns_name_contest, create_dpns_identity_name_contest, create_dpns_name_contest_give_key_info, perform_votes_multi};
        use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
        use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::PaidConsensusError;
        use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
        use dpp::consensus::state::state_error::StateError;

        #[test]
        fn test_document_creation() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_document_creation_should_fail_if_reusing_entropy() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                .expect("expected to commit transaction");

            // Now let's create a second document with the same entropy

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

            document.set("avatarUrl", "http://test.com/coy.jpg".into());

            let documents_batch_create_transition =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document,
                    profile,
                    entropy.0,
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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::DocumentAlreadyPresentError { .. }),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_document_creation_with_very_big_field() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let dashpay_contract_no_max_length = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/dashpay/dashpay-contract-no-max-length.json",
                None,
                None,
            );

            let dashpay_contract = dashpay_contract_no_max_length.clone();

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

            let max_field_size = platform_version.system_limits.max_field_value_size;
            let avatar_size = max_field_size + 1000;

            document.set(
                "avatar",
                Value::Bytes(
                    rng.sample_iter(Standard)
                        .take(avatar_size as usize)
                        .collect(),
                ),
            );

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
                    false,
                    None,
                )
                .expect("expected to process state transition");
            assert_eq!(
                processing_result.execution_results().first().unwrap(),
                &PaidConsensusError(
                    ConsensusError::BasicError(BasicError::DocumentFieldMaxSizeExceededError(
                        DocumentFieldMaxSizeExceededError::new(
                            "avatar".to_string(),
                            avatar_size as u64,
                            max_field_size as u64
                        )
                    )),
                    FeeResult {
                        storage_fee: 11556000,
                        processing_fee: 526140,
                        fee_refunds: FeeRefunds::default(),
                        removed_bytes_from_system: 0
                    }
                )
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_document_creation_on_contested_unique_index() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity_1, signer_1, key_1) =
                setup_identity(&mut platform, 958, dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, 93, dash_to_credits!(0.5));

            let dpns = platform.drive.cache.system_data_contracts.load_dpns();
            let dpns_contract = dpns.clone();

            let preorder = dpns_contract
                .document_type_for_name("preorder")
                .expect("expected a profile document type");

            assert!(!preorder.documents_mutable());
            assert!(preorder.documents_can_be_deleted());
            assert!(!preorder.documents_transferable().is_transferable());

            let domain = dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type");

            assert!(!domain.documents_mutable());
            // Deletion is disabled with data trigger
            assert!(domain.documents_can_be_deleted());
            assert!(domain.documents_transferable().is_transferable());

            let entropy = Bytes32::random_with_rng(&mut rng);

            let mut preorder_document_1 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut preorder_document_2 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_1 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_2 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            document_1.set("parentDomainName", "dash".into());
            document_1.set("normalizedParentDomainName", "dash".into());
            document_1.set("label", "quantum".into());
            document_1.set("normalizedLabel", "quantum".into());
            document_1.set("records.identity", document_1.owner_id().into());
            document_1.set("subdomainRules.allowSubdomains", false.into());

            document_2.set("parentDomainName", "dash".into());
            document_2.set("normalizedParentDomainName", "dash".into());
            document_2.set("label", "quantum".into());
            document_2.set("normalizedLabel", "quantum".into());
            document_2.set("records.identity", document_2.owner_id().into());
            document_2.set("subdomainRules.allowSubdomains", false.into());

            let salt_1: [u8; 32] = rng.gen();
            let salt_2: [u8; 32] = rng.gen();

            let mut salted_domain_buffer_1: Vec<u8> = vec![];
            salted_domain_buffer_1.extend(salt_1);
            salted_domain_buffer_1.extend("quantum.dash".as_bytes());

            let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

            let mut salted_domain_buffer_2: Vec<u8> = vec![];
            salted_domain_buffer_2.extend(salt_2);
            salted_domain_buffer_2.extend("quantum.dash".as_bytes());

            let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

            preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
            preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());

            document_1.set("preorderSalt", salt_1.into());
            document_2.set("preorderSalt", salt_2.into());

            let documents_batch_create_preorder_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_1,
                    preorder,
                    entropy.0,
                    &key_1,
                    2,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_1 =
                documents_batch_create_preorder_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_preorder_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_2,
                    preorder,
                    entropy.0,
                    &key_2,
                    2,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_2 =
                documents_batch_create_preorder_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_1,
                    domain,
                    entropy.0,
                    &key_1,
                    3,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_1 =
                documents_batch_create_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_2,
                    domain,
                    entropy.0,
                    &key_2,
                    3,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_2 =
                documents_batch_create_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_preorder_transition_1.clone(),
                        documents_batch_create_serialized_preorder_transition_2.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 2);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_transition_1.clone(),
                        documents_batch_create_serialized_transition_2.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 2);

            // Now let's run a query for the vote totals

            let config = bincode::config::standard()
                .with_big_endian()
                .with_no_limit();

            let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
                .expect("expected to encode the word dash");

            let quantum_encoded =
                bincode::encode_to_vec(Value::Text("quantum".to_string()), config)
                    .expect("expected to encode the word quantum");

            let index_name = "parentNameAndLabel".to_string();

            let query_validation_result = platform
                .query_contested_resource_vote_state(
                    GetContestedResourceVoteStateRequest {
                        version: Some(get_contested_resource_vote_state_request::Version::V0(
                            GetContestedResourceVoteStateRequestV0 {
                                contract_id: dpns_contract.id().to_vec(),
                                document_type_name: domain.name().clone(),
                                index_name: index_name.clone(),
                                index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                                result_type: ResultType::DocumentsAndVoteTally as i32,
                                allow_include_locked_and_abstaining_vote_tally: false,
                                start_at_identifier_info: None,
                                count: None,
                                prove: false,
                            },
                        )),
                    },
                    &platform_state,
                    platform_version,
                )
                .expect("expected to execute query")
                .into_data()
                .expect("expected query to be valid");

            let get_contested_resource_vote_state_response::Version::V0(
                GetContestedResourceVoteStateResponseV0 {
                    metadata: _,
                    result,
                },
            ) = query_validation_result.version.expect("expected a version");

            let Some(
                get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                    get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                        contenders,
                        abstain_vote_tally,
                        lock_vote_tally,
                        finished_vote_info,
                    },
                ),
            ) = result
            else {
                panic!("expected contenders")
            };

            assert_eq!(abstain_vote_tally, None);

            assert_eq!(lock_vote_tally, None);

            assert_eq!(finished_vote_info, None);

            assert_eq!(contenders.len(), 2);

            let first_contender = contenders.first().unwrap();

            let second_contender = contenders.last().unwrap();

            let first_contender_document = Document::from_bytes(
                first_contender
                    .document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            let second_contender_document = Document::from_bytes(
                second_contender
                    .document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            assert_ne!(first_contender_document, second_contender_document);

            assert_eq!(first_contender.identifier, identity_1.id().to_vec());

            assert_eq!(second_contender.identifier, identity_2.id().to_vec());

            assert_eq!(first_contender.vote_count, Some(0));

            assert_eq!(second_contender.vote_count, Some(0));

            let GetContestedResourceVoteStateResponse { version } = platform
                .query_contested_resource_vote_state(
                    GetContestedResourceVoteStateRequest {
                        version: Some(get_contested_resource_vote_state_request::Version::V0(
                            GetContestedResourceVoteStateRequestV0 {
                                contract_id: dpns_contract.id().to_vec(),
                                document_type_name: domain.name().clone(),
                                index_name: "parentNameAndLabel".to_string(),
                                index_values: vec![dash_encoded, quantum_encoded],
                                result_type: ResultType::DocumentsAndVoteTally as i32,
                                allow_include_locked_and_abstaining_vote_tally: true,
                                start_at_identifier_info: None,
                                count: None,
                                prove: true,
                            },
                        )),
                    },
                    &platform_state,
                    platform_version,
                )
                .expect("expected to execute query")
                .into_data()
                .expect("expected query to be valid");

            let get_contested_resource_vote_state_response::Version::V0(
                GetContestedResourceVoteStateResponseV0 {
                    metadata: _,
                    result,
                },
            ) = version.expect("expected a version");

            let Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)) = result
            else {
                panic!("expected contenders")
            };

            let resolved_contested_document_vote_poll_drive_query =
                ResolvedContestedDocumentVotePollDriveQuery {
                    vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                        contract: DataContractResolvedInfo::BorrowedDataContract(&dpns_contract),
                        document_type_name: domain.name().clone(),
                        index_name: index_name.clone(),
                        index_values: vec![
                            Value::Text("dash".to_string()),
                            Value::Text("quantum".to_string()),
                        ],
                    },
                    result_type: DocumentsAndVoteTally,
                    offset: None,
                    limit: None,
                    start_at: None,
                    allow_include_locked_and_abstaining_vote_tally: true,
                };

            let (_root_hash, result) = resolved_contested_document_vote_poll_drive_query
                .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
                .expect("expected to verify proof");

            let contenders = result.contenders;
            assert_eq!(contenders.len(), 2);

            let first_contender = contenders.first().unwrap();

            let second_contender = contenders.last().unwrap();

            let first_contender_document = Document::from_bytes(
                first_contender
                    .serialized_document()
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            let second_contender_document = Document::from_bytes(
                second_contender
                    .serialized_document()
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            assert_ne!(first_contender_document, second_contender_document);

            assert_eq!(first_contender.identity_id(), identity_1.id());

            assert_eq!(second_contender.identity_id(), identity_2.id());

            assert_eq!(first_contender.vote_tally(), Some(0));

            assert_eq!(second_contender.vote_tally(), Some(0));
        }

        #[test]
        fn test_document_creation_on_contested_unique_index_should_fail_if_reusing_entropy() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity_1, signer_1, key_1) =
                setup_identity(&mut platform, 958, dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, 93, dash_to_credits!(0.5));

            let dpns = platform.drive.cache.system_data_contracts.load_dpns();
            let dpns_contract = dpns.clone();

            let preorder = dpns_contract
                .document_type_for_name("preorder")
                .expect("expected a profile document type");

            assert!(!preorder.documents_mutable());
            assert!(preorder.documents_can_be_deleted());
            assert!(!preorder.documents_transferable().is_transferable());

            let domain = dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type");

            assert!(!domain.documents_mutable());
            // Deletion is disabled with data trigger
            assert!(domain.documents_can_be_deleted());
            assert!(domain.documents_transferable().is_transferable());

            let entropy = Bytes32::random_with_rng(&mut rng);

            let mut preorder_document_1 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut preorder_document_2 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let new_entropy = Bytes32::random_with_rng(&mut rng);

            let mut preorder_document_3_on_identity_1 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    new_entropy, //change entropy here
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_1 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_2 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_3_on_identity_1 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy, //same entropy
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            document_1.set("parentDomainName", "dash".into());
            document_1.set("normalizedParentDomainName", "dash".into());
            document_1.set("label", "quantum".into());
            document_1.set("normalizedLabel", "quantum".into());
            document_1.set("records.identity", document_1.owner_id().into());
            document_1.set("subdomainRules.allowSubdomains", false.into());

            document_2.set("parentDomainName", "dash".into());
            document_2.set("normalizedParentDomainName", "dash".into());
            document_2.set("label", "quantum".into());
            document_2.set("normalizedLabel", "quantum".into());
            document_2.set("records.identity", document_2.owner_id().into());
            document_2.set("subdomainRules.allowSubdomains", false.into());

            document_3_on_identity_1.set("parentDomainName", "dash".into());
            document_3_on_identity_1.set("normalizedParentDomainName", "dash".into());
            document_3_on_identity_1.set("label", "cry".into());
            document_3_on_identity_1.set("normalizedLabel", "cry".into());
            document_3_on_identity_1.set(
                "records.identity",
                document_3_on_identity_1.owner_id().into(),
            );
            document_3_on_identity_1.set("subdomainRules.allowSubdomains", false.into());

            let salt_1: [u8; 32] = rng.gen();
            let salt_2: [u8; 32] = rng.gen();
            let salt_3: [u8; 32] = rng.gen();

            let mut salted_domain_buffer_1: Vec<u8> = vec![];
            salted_domain_buffer_1.extend(salt_1);
            salted_domain_buffer_1.extend("quantum.dash".as_bytes());

            let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

            let mut salted_domain_buffer_2: Vec<u8> = vec![];
            salted_domain_buffer_2.extend(salt_2);
            salted_domain_buffer_2.extend("quantum.dash".as_bytes());

            let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

            let mut salted_domain_buffer_3: Vec<u8> = vec![];
            salted_domain_buffer_3.extend(salt_3);
            salted_domain_buffer_3.extend("cry.dash".as_bytes());

            let salted_domain_hash_3 = hash_double(salted_domain_buffer_3);

            preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
            preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());
            preorder_document_3_on_identity_1.set("saltedDomainHash", salted_domain_hash_3.into());

            document_1.set("preorderSalt", salt_1.into());
            document_2.set("preorderSalt", salt_2.into());
            document_3_on_identity_1.set("preorderSalt", salt_3.into());

            let documents_batch_create_preorder_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_1,
                    preorder,
                    entropy.0,
                    &key_1,
                    2,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_1 =
                documents_batch_create_preorder_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_preorder_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_2,
                    preorder,
                    entropy.0,
                    &key_2,
                    2,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_2 =
                documents_batch_create_preorder_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_preorder_transition_3 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_3_on_identity_1,
                    preorder,
                    new_entropy.0,
                    &key_1,
                    3,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_3 =
                documents_batch_create_preorder_transition_3
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_1,
                    domain,
                    entropy.0,
                    &key_1,
                    4,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_1 =
                documents_batch_create_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_2,
                    domain,
                    entropy.0,
                    &key_2,
                    3,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_2 =
                documents_batch_create_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_3 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_3_on_identity_1,
                    domain,
                    entropy.0,
                    &key_1,
                    5,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_3 =
                documents_batch_create_transition_3
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_preorder_transition_1.clone(),
                        documents_batch_create_serialized_preorder_transition_2.clone(),
                        documents_batch_create_serialized_preorder_transition_3.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 3);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_transition_1.clone(),
                        documents_batch_create_serialized_transition_2.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 2);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_create_serialized_transition_3.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(
                        StateError::DocumentContestDocumentWithSameIdAlreadyPresentError { .. }
                    ),
                    _
                )]
            );

            // Now let's run a query for the vote totals

            let config = bincode::config::standard()
                .with_big_endian()
                .with_no_limit();

            let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
                .expect("expected to encode the word dash");

            let quantum_encoded =
                bincode::encode_to_vec(Value::Text("quantum".to_string()), config)
                    .expect("expected to encode the word quantum");

            let index_name = "parentNameAndLabel".to_string();

            let query_validation_result = platform
                .query_contested_resource_vote_state(
                    GetContestedResourceVoteStateRequest {
                        version: Some(get_contested_resource_vote_state_request::Version::V0(
                            GetContestedResourceVoteStateRequestV0 {
                                contract_id: dpns_contract.id().to_vec(),
                                document_type_name: domain.name().clone(),
                                index_name: index_name.clone(),
                                index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                                result_type: ResultType::DocumentsAndVoteTally as i32,
                                allow_include_locked_and_abstaining_vote_tally: false,
                                start_at_identifier_info: None,
                                count: None,
                                prove: false,
                            },
                        )),
                    },
                    &platform_state,
                    platform_version,
                )
                .expect("expected to execute query")
                .into_data()
                .expect("expected query to be valid");

            let get_contested_resource_vote_state_response::Version::V0(
                GetContestedResourceVoteStateResponseV0 {
                    metadata: _,
                    result,
                },
            ) = query_validation_result.version.expect("expected a version");

            let Some(
                get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                    get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                        contenders,
                        abstain_vote_tally,
                        lock_vote_tally,
                        finished_vote_info,
                    },
                ),
            ) = result
            else {
                panic!("expected contenders")
            };

            assert_eq!(abstain_vote_tally, None);

            assert_eq!(lock_vote_tally, None);

            assert_eq!(finished_vote_info, None);

            assert_eq!(contenders.len(), 2);

            let first_contender = contenders.first().unwrap();

            let second_contender = contenders.last().unwrap();

            let first_contender_document = Document::from_bytes(
                first_contender
                    .document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            let second_contender_document = Document::from_bytes(
                second_contender
                    .document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            assert_ne!(first_contender_document, second_contender_document);

            assert_eq!(first_contender.identifier, identity_1.id().to_vec());

            assert_eq!(second_contender.identifier, identity_2.id().to_vec());

            assert_eq!(first_contender.vote_count, Some(0));

            assert_eq!(second_contender.vote_count, Some(0));

            let GetContestedResourceVoteStateResponse { version } = platform
                .query_contested_resource_vote_state(
                    GetContestedResourceVoteStateRequest {
                        version: Some(get_contested_resource_vote_state_request::Version::V0(
                            GetContestedResourceVoteStateRequestV0 {
                                contract_id: dpns_contract.id().to_vec(),
                                document_type_name: domain.name().clone(),
                                index_name: "parentNameAndLabel".to_string(),
                                index_values: vec![dash_encoded, quantum_encoded],
                                result_type: ResultType::DocumentsAndVoteTally as i32,
                                allow_include_locked_and_abstaining_vote_tally: true,
                                start_at_identifier_info: None,
                                count: None,
                                prove: true,
                            },
                        )),
                    },
                    &platform_state,
                    platform_version,
                )
                .expect("expected to execute query")
                .into_data()
                .expect("expected query to be valid");

            let get_contested_resource_vote_state_response::Version::V0(
                GetContestedResourceVoteStateResponseV0 {
                    metadata: _,
                    result,
                },
            ) = version.expect("expected a version");

            let Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)) = result
            else {
                panic!("expected contenders")
            };

            let resolved_contested_document_vote_poll_drive_query =
                ResolvedContestedDocumentVotePollDriveQuery {
                    vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                        contract: DataContractResolvedInfo::BorrowedDataContract(&dpns_contract),
                        document_type_name: domain.name().clone(),
                        index_name: index_name.clone(),
                        index_values: vec![
                            Value::Text("dash".to_string()),
                            Value::Text("quantum".to_string()),
                        ],
                    },
                    result_type: DocumentsAndVoteTally,
                    offset: None,
                    limit: None,
                    start_at: None,
                    allow_include_locked_and_abstaining_vote_tally: true,
                };

            let (_root_hash, result) = resolved_contested_document_vote_poll_drive_query
                .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
                .expect("expected to verify proof");

            let contenders = result.contenders;
            assert_eq!(contenders.len(), 2);

            let first_contender = contenders.first().unwrap();

            let second_contender = contenders.last().unwrap();

            let first_contender_document = Document::from_bytes(
                first_contender
                    .serialized_document()
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            let second_contender_document = Document::from_bytes(
                second_contender
                    .serialized_document()
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            assert_ne!(first_contender_document, second_contender_document);

            assert_eq!(first_contender.identity_id(), identity_1.id());

            assert_eq!(second_contender.identity_id(), identity_2.id());

            assert_eq!(first_contender.vote_tally(), Some(0));

            assert_eq!(second_contender.vote_tally(), Some(0));
        }

        #[test]
        fn test_that_a_contested_document_can_not_be_added_to_after_a_week() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (contender_1, contender_2, dpns_contract) = create_dpns_identity_name_contest(
                &mut platform,
                &platform_state,
                7,
                "quantum",
                platform_version,
            );

            perform_votes_multi(
                &mut platform,
                dpns_contract.as_ref(),
                vec![
                    (TowardsIdentity(contender_1.id()), 50),
                    (TowardsIdentity(contender_2.id()), 5),
                    (ResourceVoteChoice::Abstain, 10),
                    (ResourceVoteChoice::Lock, 3),
                ],
                "quantum",
                10,
                None,
                platform_version,
            );

            let max_join_time = platform_version
                .dpp
                .validation
                .voting
                .allow_other_contenders_time_testing_ms;

            fast_forward_to_block(&platform, max_join_time / 2, 900, 42, 0, false);

            let platform_state = platform.state.load();

            let _contender_3 = add_contender_to_dpns_name_contest(
                &mut platform,
                &platform_state,
                4,
                "quantum",
                None, // this should succeed, as we are under a week
                platform_version,
            );

            let time_now = platform_version
                .dpp
                .validation
                .voting
                .allow_other_contenders_time_testing_ms
                + 100;

            fast_forward_to_block(&platform, time_now, 900, 42, 0, false); //more than a week, less than 2 weeks

            let platform_state = platform.state.load();

            // We expect this to fail

            let time_started = 0;

            let extra_time_used = 3000; // add_contender_to_dpns_name_contest uses this extra time

            let expected_error_message = format!(
                "Document Contest for vote_poll ContestedDocumentResourceVotePoll {{ contract_id: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec, document_type_name: domain, index_name: parentNameAndLabel, index_values: [string dash, string quantum] }} is not joinable V0(ContestedDocumentVotePollStoredInfoV0 {{ finalized_events: [], vote_poll_status: Started(BlockInfo {{ time_ms: {}, height: 0, core_height: 0, epoch: 0 }}), locked_count: 0 }}), it started {} and it is now {}, and you can only join for {}",
                time_started + extra_time_used,
                time_started + extra_time_used,
                time_now + extra_time_used,
                max_join_time
            );

            let _contender_4 = add_contender_to_dpns_name_contest(
                &mut platform,
                &platform_state,
                9,
                "quantum",
                Some(expected_error_message.as_str()), // this should fail, as we are over a week
                platform_version,
            );
        }

        #[test]
        fn test_that_a_contest_can_not_be_joined_twice_by_the_same_identity() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (
                (
                    _contender_1,
                    contender_1_signer,
                    contender_1_key,
                    _preorder_document_1,
                    (mut document_1, _entropy),
                ),
                (_contender_2, _, _, _, _),
                dpns_contract,
            ) = create_dpns_name_contest_give_key_info(
                &mut platform,
                &platform_state,
                7,
                "quantum",
                platform_version,
            );

            let domain = dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type");

            let mut rng = StdRng::seed_from_u64(89);

            let different_entropy = Bytes32::random_with_rng(&mut rng);

            document_1.set_id(Document::generate_document_id_v0(
                dpns_contract.id_ref(),
                document_1.owner_id_ref(),
                domain.name(),
                different_entropy.as_slice(),
            ));

            let documents_batch_create_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_1,
                    domain,
                    different_entropy.0,
                    &contender_1_key,
                    4,
                    0,
                    &contender_1_signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_1 =
                documents_batch_create_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_create_serialized_transition_1.clone()],
                    &platform_state,
                    &BlockInfo::default_with_time(
                        &platform_state
                            .last_committed_block_time_ms()
                            .unwrap_or_default()
                            + 3000,
                    ),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let result = processing_result.into_execution_results().remove(0);

            let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result
            else {
                panic!("expected a paid consensus error");
            };
            assert_eq!(consensus_error.to_string(), "An Identity with the id BjNejy4r9QAvLHpQ9Yq6yRMgNymeGZ46d48fJxJbMrfW is already a contestant for the vote_poll ContestedDocumentResourceVotePoll { contract_id: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec, document_type_name: domain, index_name: parentNameAndLabel, index_values: [string dash, string quantum] }");
        }

        #[test]
        fn test_that_a_contested_document_can_not_be_added_if_we_are_locked() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (contender_1, contender_2, dpns_contract) = create_dpns_identity_name_contest(
                &mut platform,
                &platform_state,
                7,
                "quantum",
                platform_version,
            );

            perform_votes_multi(
                &mut platform,
                dpns_contract.as_ref(),
                vec![
                    (TowardsIdentity(contender_1.id()), 3),
                    (TowardsIdentity(contender_2.id()), 5),
                    (ResourceVoteChoice::Abstain, 8),
                    (ResourceVoteChoice::Lock, 10),
                ],
                "quantum",
                10,
                None,
                platform_version,
            );

            fast_forward_to_block(
                &platform,
                platform_version
                    .dpp
                    .validation
                    .voting
                    .allow_other_contenders_time_testing_ms
                    / 2,
                900,
                42,
                0,
                false,
            ); // a time when others can join

            let platform_state = platform.state.load();

            let _contender_3 = add_contender_to_dpns_name_contest(
                &mut platform,
                &platform_state,
                4,
                "quantum",
                None, // this should succeed, as we are under the `platform_version.dpp.validation.voting.allow_other_contenders_time_testing_ms`
                platform_version,
            );

            let time_after_distribution_limit = platform_version
                .dpp
                .voting_versions
                .default_vote_poll_time_duration_test_network_ms
                + 10_000; // add 10s (3 seconds is used by create_dpns_identity_name_contest)

            fast_forward_to_block(&platform, time_after_distribution_limit, 900, 42, 0, false); // after distribution

            let platform_state = platform.state.load();

            let transaction = platform.drive.grove.start_transaction();

            platform
                .check_for_ended_vote_polls(
                    &platform_state,
                    &platform_state,
                    &BlockInfo {
                        time_ms: time_after_distribution_limit,
                        height: 900,
                        core_height: 42,
                        epoch: Default::default(),
                    },
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to check for ended vote polls");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let platform_state = platform.state.load();

            // We expect this to fail

            let expected_error_message = format!(
                "Document Contest for vote_poll ContestedDocumentResourceVotePoll {{ contract_id: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec, document_type_name: domain, index_name: parentNameAndLabel, index_values: [string dash, string quantum] }} is currently already locked V0(ContestedDocumentVotePollStoredInfoV0 {{ finalized_events: [ContestedDocumentVotePollStoredInfoVoteEventV0 {{ resource_vote_choices: [FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: TowardsIdentity(BjNejy4r9QAvLHpQ9Yq6yRMgNymeGZ46d48fJxJbMrfW), voters: [2oGomAQc47V9h3mkpyHUPbF74gT2AmoYKg1oSb94Rbwm:1, 4iroeiNBeBYZetCt21kW7FGyczE8WqoqzZ48YAHwyV7R:1, Cdf8V4KGHHd395x5xPJPPrzTKwmp5MqbuszSE2iMzzeP:1] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: TowardsIdentity(FiLk5pGtspYtF65PKsQq3YFr1DEiXPHTZeKjusT6DuqN), voters: [] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: TowardsIdentity(Fv8S6kTbNrRqKC7PR7XcRUoPR59bxNhhggg5mRaNN6ow), voters: [4MK8GWEWX1PturUqjZJefdE4WGrUqz1UQZnbK17ENkeA:1, 5gRudU7b4n8LYkNvhZomv6FtMrP7gvaTvRrHKfaTS22K:1, AfzQBrdwzDuTVdXrMWqQyVvXRWqPMDVjA76hViuGLh6W:1, E75wdFZB22P1uW1wJBJGPgXZuZKLotK7YmbH5wUk5msH:1, G3ZfS2v39x6FuLGnnJ1RNQyy4zn4Wb64KiGAjqj39wUu:1] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: Abstain, voters: [5Ur8tDxJnatfUd9gcVFDde7ptHydujZzJLNTxa6aMYYy:1, 93Gsg14oT9K4FLYmC7N26uS4g5b7JcM1GwGEDeJCCBPJ:1, 96eX4PTjbXRuGHuMzwXdptWFtHcboXbtevk51Jd73pP7:1, AE9xm2mbemDeMxPUzyt35Agq1axRxggVfV4DRLAZp7Qt:1, FbLyu5d7JxEsvSsujj7Wopg57Wrvz9HH3UULCusKpBnF:1, GsubMWb3LH1skUJrcxTmZ7wus1habJcbpb8su8yBVqFY:1, H9UrL7aWaxDmXhqeGMJy7LrGdT2wWb45mc7kQYsoqwuf:1, Hv88mzPZVKq2fnjoUqK56vjzkcmqRHpWE1ME4z1MXDrw:1] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: Lock, voters: [F1oA8iAoyJ8dgCAi2GSPqcNhp9xEuAqhP47yXBDw5QR:1, 2YSjsJUp74MJpm12rdn8wyPR5MY3c322pV8E8siw989u:1, 3fQrmN4PWhthUFnCFTaJqbT2PPGf7MytAyik4eY1DP8V:1, 7r7gnAiZunVLjtSd5ky4yvPpnWTFYbJuQAapg8kDCeNK:1, 86TUE89xNkBDcmshXRD198xjAvMmKecvHbwo6i83AmqA:1, 97iYr4cirPdG176kqa5nvJWT9tsnqxHmENfRnZUgM6SC:1, 99nKfYZL4spsTe9p9pPNhc1JWv9yq4CbPPMPm87a5sgn:1, BYAqFxCVwMKrw5YAQMCFQGiAF2v3YhKRm2EdGfgkYN9G:1, CGKeK3AfdZUxXF3qH9zxp5MR7Z4WvDVqMrU5wjMKqT5C:1, HRPPEX4mdoZAMkg6NLJUgDzN4pSTpiDXEAGcR5JBdiXX:1] }}], start_block: BlockInfo {{ time_ms: 3000, height: 0, core_height: 0, epoch: 0 }}, finalization_block: BlockInfo {{ time_ms: {}, height: 900, core_height: 42, epoch: 0 }}, winner: Locked }}], vote_poll_status: Locked, locked_count: 1 }}), unlocking is possible by paying 400000000000 credits",
                time_after_distribution_limit
            );

            let _contender_4 = add_contender_to_dpns_name_contest(
                &mut platform,
                &platform_state,
                9,
                "quantum",
                Some(expected_error_message.as_str()), // this should fail, as it is locked
                platform_version,
            );
        }

        #[test]
        fn test_document_creation_on_restricted_document_type_that_only_allows_contract_owner_to_create(
        ) {
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (another_identity, another_identity_signer, another_identity_key) =
                setup_identity(&mut platform, 450, dash_to_credits!(0.1));

            let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase-creation-restricted-to-owner.json";

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // let's construct the grovedb structure for the card game data contract
            let mut contract = json_document_to_contract(card_game_path, true, platform_version)
                .expect("expected to get data contract");

            contract.set_owner_id(identity.id());

            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            let card_document_type = contract
                .document_type_for_name("card")
                .expect("expected a profile document type");

            assert_eq!(
                card_document_type.creation_restriction_mode(),
                CreationRestrictionMode::OwnerOnly
            );

            let mut rng = StdRng::seed_from_u64(433);

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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // There is no issue because the creator of the contract made the document

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Now let's try for another identity

            let mut document = card_document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    another_identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            document.set("attack", 8.into());
            document.set("defense", 2.into());

            let documents_batch_create_transition =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document.clone(),
                    card_document_type,
                    entropy.0,
                    &another_identity_key,
                    2,
                    0,
                    &another_identity_signer,
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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // There is no issue because the creator of the contract made the document

            assert_eq!(processing_result.invalid_paid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let result = processing_result.into_execution_results().remove(0);

            let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result
            else {
                panic!("expected a paid consensus error");
            };
            assert_eq!(consensus_error.to_string(), "Document Creation on 86LHvdC1Tqx5P97LQUSibGFqf2vnKFpB6VkqQ7oso86e:card is not allowed because of the document type's creation restriction mode Owner Only");
        }
    }

    mod replacement_tests {
        use super::*;
        use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
        use dpp::identifier::Identifier;
        use dpp::prelude::IdentityNonce;
        use std::collections::BTreeMap;

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                    false,
                    None,
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 1341740);

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        fn perform_document_replace_on_profile_after_epoch_change(
            original_name: &str,
            new_names: Vec<(&str, StorageFlags)>,
        ) {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

            document.set("displayName", original_name.into());
            document.set("avatarUrl", "http://test.com/bob.jpg".into());

            let documents_batch_create_transition =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document.clone(),
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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            for (i, (new_name, mut expected_flags)) in new_names.into_iter().enumerate() {
                document.increment_revision().unwrap();
                document.set("displayName", new_name.into());

                fast_forward_to_block(
                    &platform,
                    500_000_000 + i as u64 * 1000,
                    900 + i as u64,
                    42,
                    1 + i as u16,
                    true,
                ); //less than a week

                let documents_batch_update_transition =
                    DocumentsBatchTransition::new_document_replacement_transition_from_document(
                        document.clone(),
                        profile,
                        &key,
                        3 + i as IdentityNonce,
                        0,
                        &signer,
                        platform_version,
                        None,
                        None,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let documents_batch_update_serialized_transition =
                    documents_batch_update_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let platform_state = platform.state.load();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_update_serialized_transition.clone()],
                        &platform_state,
                        platform_state.last_block_info(),
                        &transaction,
                        platform_version,
                        false,
                        None,
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

                assert_eq!(
                    processing_result.valid_count(),
                    1,
                    "{:?}",
                    processing_result.execution_results()
                );

                let drive_query = DriveDocumentQuery::new_primary_key_single_item_query(
                    &dashpay,
                    profile,
                    document.id(),
                );

                let mut documents = platform
                    .drive
                    .query_documents_with_flags(drive_query, None, false, None, None)
                    .expect("expected to get back documents")
                    .documents_owned();

                let (_first_document, storage_flags) = documents.remove(0);

                let storage_flags = storage_flags.expect("expected storage flags");

                expected_flags.set_owner_id(identity.id().to_buffer());

                assert_eq!(storage_flags, expected_flags);
            }

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size() {
            perform_document_replace_on_profile_after_epoch_change(
                "Sam",
                vec![(
                    "Samuel",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                )],
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable_different_epoch_smaller_size() {
            perform_document_replace_on_profile_after_epoch_change(
                "Sam",
                vec![(
                    "S",
                    StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
                )],
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable_different_epoch_same_size() {
            perform_document_replace_on_profile_after_epoch_change(
                "Sam",
                vec![(
                    "Max",
                    StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
                )],
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size(
        ) {
            perform_document_replace_on_profile_after_epoch_change(
                "Sam",
                vec![
                    (
                        "Samuel",
                        StorageFlags::MultiEpochOwned(
                            0,
                            BTreeMap::from([(1, 6)]),
                            Identifier::default().to_buffer(),
                        ),
                    ),
                    (
                        "SamuelW",
                        StorageFlags::MultiEpochOwned(
                            0,
                            BTreeMap::from([(1, 6), (2, 4)]),
                            Identifier::default().to_buffer(),
                        ),
                    ),
                ],
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size_by_3_bytes(
        ) {
            perform_document_replace_on_profile_after_epoch_change(
                "Sam",
                vec![
                    (
                        "Samuel",
                        StorageFlags::MultiEpochOwned(
                            0,
                            BTreeMap::from([(1, 6)]),
                            Identifier::default().to_buffer(),
                        ),
                    ),
                    (
                        "SamuelWes",
                        StorageFlags::MultiEpochOwned(
                            0,
                            BTreeMap::from([(1, 6), (2, 6)]),
                            Identifier::default().to_buffer(),
                        ),
                    ),
                ],
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_smaller_size(
        ) {
            // In this case we start with the size Samuell Base epoch 0 epoch 1 added 7 bytes
            // Then we try to update it to         Sami    Base epoch 2
            // Epoch 1 added 7 bytes is itself 3 bytes
            // Sami is 3 bytes less than Samuell
            // First iteration will say we should remove 6 bytes
            // We need to start by calculating the cost of the original storage flags, in this case 5 bytes
            // Then we need to calculate the cost of the new storage flags, in this case 2 bytes
            // We should do the difference, then apply that difference in the combination function
            perform_document_replace_on_profile_after_epoch_change(
                "Sam",
                vec![
                    (
                        "Samuell",
                        StorageFlags::MultiEpochOwned(
                            0,
                            BTreeMap::from([(1, 7)]),
                            Identifier::default().to_buffer(),
                        ),
                    ),
                    (
                        "Sami",
                        StorageFlags::MultiEpochOwned(
                            0,
                            BTreeMap::from([(1, 4)]),
                            Identifier::default().to_buffer(),
                        ),
                    ),
                ],
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_back_to_original(
        ) {
            perform_document_replace_on_profile_after_epoch_change(
                "Sam",
                vec![
                    (
                        "Samuel",
                        StorageFlags::MultiEpochOwned(
                            0,
                            BTreeMap::from([(1, 6)]),
                            Identifier::default().to_buffer(),
                        ),
                    ),
                    (
                        "Sam",
                        StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
                    ),
                ],
            );
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_not_mutable() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(437);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

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
                    false,
                    None,
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 41880);
        }

        #[test]
        fn test_document_replace_on_document_type_that_is_not_mutable_but_is_transferable() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_transfer_only(Transferable::Always);

            let mut rng = StdRng::seed_from_u64(435);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, _, _) = setup_identity(&mut platform, 452, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", receiver.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_transfer_serialized_transition =
                documents_batch_transfer_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);

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
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 516040);
        }

        #[test]
        fn test_double_document_replace() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

            let mut altered_document_2 = altered_document.clone();

            altered_document_2.increment_revision().unwrap();
            altered_document_2.set("displayName", "Ody".into());
            altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

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
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let receiver_documents_sql_string = "select * from profile".to_string();

            let query_documents = DriveDocumentQuery::from_sql_expr(
                receiver_documents_sql_string.as_str(),
                &dashpay,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

            let documents_batch_update_transition_1 =
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

            let documents_batch_update_serialized_transition_1 =
                documents_batch_update_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_update_transition_2 =
                DocumentsBatchTransition::new_document_replacement_transition_from_document(
                    altered_document_2,
                    profile,
                    &key,
                    4,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_update_serialized_transition_2 =
                documents_batch_update_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_update_serialized_transition_1.clone(),
                        documents_batch_update_serialized_transition_2.clone(),
                    ],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            assert_eq!(processing_result.valid_count(), 2);

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_double_document_replace_different_height_same_epoch() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

            let mut altered_document_2 = altered_document.clone();

            altered_document_2.increment_revision().unwrap();
            altered_document_2.set("displayName", "Ody".into());
            altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

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
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let receiver_documents_sql_string = "select * from profile".to_string();

            let query_documents = DriveDocumentQuery::from_sql_expr(
                receiver_documents_sql_string.as_str(),
                &dashpay,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

            fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

            let platform_state = platform.state.load();

            let documents_batch_update_transition_1 =
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

            let documents_batch_update_serialized_transition_1 =
                documents_batch_update_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_update_transition_2 =
                DocumentsBatchTransition::new_document_replacement_transition_from_document(
                    altered_document_2,
                    profile,
                    &key,
                    4,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_update_serialized_transition_2 =
                documents_batch_update_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition_1.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] displayName:string Samuel publicMessage:string 8XG7KBGNvm2  ");

            fast_forward_to_block(&platform, 1_600_000_000, 902, 44, 1, false); //next epoch

            let platform_state = platform.state.load();

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition_2.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_double_document_replace_no_change_different_height_same_epoch() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

            let mut altered_document_2 = altered_document.clone();

            altered_document_2.increment_revision().unwrap();

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
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let receiver_documents_sql_string = "select * from profile".to_string();

            let query_documents = DriveDocumentQuery::from_sql_expr(
                receiver_documents_sql_string.as_str(),
                &dashpay,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

            fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

            let platform_state = platform.state.load();

            let documents_batch_update_transition_1 =
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

            let documents_batch_update_serialized_transition_1 =
                documents_batch_update_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_update_transition_2 =
                DocumentsBatchTransition::new_document_replacement_transition_from_document(
                    altered_document_2,
                    profile,
                    &key,
                    4,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_update_serialized_transition_2 =
                documents_batch_update_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition_1.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

            fast_forward_to_block(&platform, 1_600_000_000, 902, 44, 1, false); //next epoch

            let platform_state = platform.state.load();

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition_2.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_double_document_replace_different_height_different_epoch() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

            let mut altered_document_2 = altered_document.clone();

            altered_document_2.increment_revision().unwrap();
            altered_document_2.set("displayName", "Ody".into());
            altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

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
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let receiver_documents_sql_string = "select * from profile".to_string();

            let query_documents = DriveDocumentQuery::from_sql_expr(
                receiver_documents_sql_string.as_str(),
                &dashpay,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

            fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

            let platform_state = platform.state.load();

            let documents_batch_update_transition_1 =
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

            let documents_batch_update_serialized_transition_1 =
                documents_batch_update_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_update_transition_2 =
                DocumentsBatchTransition::new_document_replacement_transition_from_document(
                    altered_document_2,
                    profile,
                    &key,
                    4,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_update_serialized_transition_2 =
                documents_batch_update_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition_1.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] displayName:string Samuel publicMessage:string 8XG7KBGNvm2  ");

            fast_forward_to_block(&platform, 1_600_000_000, 905, 44, 2, true); //next epoch

            let platform_state = platform.state.load();

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition_2.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            let query_sender_results = platform
                .drive
                .query_documents(query_documents.clone(), None, false, None, None)
                .expect("expected query result");

            let document = query_sender_results
                .documents()
                .first()
                .expect("expected a document");

            assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
    }

    mod deletion_tests {
        use super::*;

        #[test]
        fn test_document_delete_on_document_type_that_is_mutable_and_can_be_deleted() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
            let dashpay_contract = dashpay.clone();

            let profile = dashpay_contract
                .document_type_for_name("profile")
                .expect("expected a profile document type");

            assert!(profile.documents_mutable());

            assert!(profile.documents_can_be_deleted());

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
                    false,
                    None,
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 1609340);

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_document_delete_on_document_type_that_is_mutable_and_can_not_be_deleted() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let contract_path = "tests/supporting_files/contract/dashpay/dashpay-contract-contact-request-mutable-and-can-not-be-deleted.json";

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // let's construct the grovedb structure for the card game data contract
            let dashpay_contract = json_document_to_contract(contract_path, true, platform_version)
                .expect("expected to get data contract");
            platform
                .drive
                .apply_contract(
                    &dashpay_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            let mut rng = StdRng::seed_from_u64(437);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

            let contact_request_document_type = dashpay_contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type");

            assert!(contact_request_document_type.documents_mutable());

            assert!(!contact_request_document_type.documents_can_be_deleted());

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
                    false,
                    None,
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

            let documents_batch_deletion_serialized_transition =
                documents_batch_deletion_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);
        }

        #[test]
        fn test_document_delete_on_document_type_that_is_not_mutable_and_can_be_deleted() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let contract_path = "tests/supporting_files/contract/dashpay/dashpay-contract-contact-request-not-mutable-and-can-be-deleted.json";

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // let's construct the grovedb structure for the card game data contract
            let dashpay_contract = json_document_to_contract(contract_path, true, platform_version)
                .expect("expected to get data contract");
            platform
                .drive
                .apply_contract(
                    &dashpay_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            let mut rng = StdRng::seed_from_u64(437);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

            let contact_request_document_type = dashpay_contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type");

            assert!(!contact_request_document_type.documents_mutable());

            assert!(contact_request_document_type.documents_can_be_deleted());

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
                    false,
                    None,
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

            let documents_batch_deletion_serialized_transition =
                documents_batch_deletion_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 2762400);

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_document_delete_on_document_type_that_is_not_mutable_and_can_not_be_deleted() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(437);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

            let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
            let dashpay_contract = dashpay.clone();

            let contact_request_document_type = dashpay_contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type");

            assert!(!contact_request_document_type.documents_mutable());

            assert!(!contact_request_document_type.documents_can_be_deleted());

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
                    false,
                    None,
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

            let documents_batch_deletion_serialized_transition =
                documents_batch_deletion_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);
        }

        #[test]
        fn test_document_delete_that_does_not_yet_exist() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 516040);
        }
    }

    mod transfer_tests {
        use super::*;

        #[test]
        fn test_document_transfer_on_document_type_that_is_transferable_that_has_no_owner_indices()
        {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-all-transferable-no-owner-indexes.json";

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // let's construct the grovedb structure for the card game data contract
            let contract = json_document_to_contract(card_game_path, true, platform_version)
                .expect("expected to get data contract");
            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

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

            let documents_batch_transfer_serialized_transition =
                documents_batch_transfer_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().storage_fee, 0); // There is no storage fee, as there are no indexes that will change

            assert_eq!(processing_result.aggregated_fees().processing_fee, 1985420);

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_document_transfer_on_document_type_that_is_transferable() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_transfer_only(Transferable::Always);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", receiver.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_transfer_serialized_transition =
                documents_batch_transfer_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().storage_fee, 37341000); // 1383 bytes added

            // todo: we should expect these numbers to be closer

            assert_eq!(
                processing_result
                    .aggregated_fees()
                    .fee_refunds
                    .calculate_refunds_amount_for_identity(identity.id()),
                Some(14992395)
            );

            assert_eq!(processing_result.aggregated_fees().processing_fee, 3369260);

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

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        #[test]
        fn test_document_transfer_on_document_type_that_is_not_transferable() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_transfer_only(Transferable::Never);

            let mut rng = StdRng::seed_from_u64(435);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, _, _) = setup_identity(&mut platform, 452, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", receiver.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_transfer_serialized_transition =
                documents_batch_transfer_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);

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
                .with_crypto_card_game_transfer_only(Transferable::Never);

            let mut rng = StdRng::seed_from_u64(435);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, _, _) = setup_identity(&mut platform, 452, dash_to_credits!(0.1));

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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", receiver.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_transfer_serialized_transition =
                documents_batch_transfer_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 36200);

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
                .with_crypto_card_game_transfer_only(Transferable::Always);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, recipient_signer, recipient_key) =
                setup_identity(&mut platform, 450, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", receiver.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_transfer_serialized_transition =
                documents_batch_transfer_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 3730120);

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

            let documents_batch_deletion_serialized_transition =
                documents_batch_deletion_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 571240);

            let issues = platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
                .expect("expected to have no issues");

            assert_eq!(
                issues.len(),
                0,
                "issues are {}",
                issues
                    .iter()
                    .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
    }

    mod nft_tests {
        use super::*;
        #[test]
        fn test_document_set_price_on_document_without_ability_to_purchase() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_transfer_only(Transferable::Always);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
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

            // We expect the sender to have 1 document, and the receiver to have none
            assert_eq!(query_sender_results.documents().len(), 1);

            document.set_revision(Some(2));

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.1),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let result = processing_result.into_execution_results().remove(0);

            let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result
            else {
                panic!("expected a paid consensus error");
            };
            assert_eq!(consensus_error.to_string(), "Document transition action card is in trade mode No Trading that does not support the seller setting the price is not supported");
        }

        #[test]
        fn test_document_set_price() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_nft(TradeMode::DirectPurchase);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", receiver.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.1),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 2473880);

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

            // The sender document should have the desired price

            let document = query_sender_results.documents().first().unwrap();

            let price: Credits = document
                .properties()
                .get_integer("$price")
                .expect("expected to get back price");

            assert_eq!(dash_to_credits!(0.1), price);

            assert_eq!(document.revision(), Some(2));
        }

        #[test]
        fn test_document_set_price_and_purchase() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_nft(TradeMode::DirectPurchase);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (purchaser, recipient_signer, recipient_key) =
                setup_identity(&mut platform, 450, dash_to_credits!(1.0));

            let seller_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected that identity exists");

            assert_eq!(seller_balance, dash_to_credits!(0.1));

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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            assert_eq!(
                processing_result
                    .aggregated_fees()
                    .clone()
                    .into_balance_change(identity.id())
                    .change(),
                &BalanceChange::RemoveFromBalance {
                    required_removed_balance: 123579000,
                    desired_removed_balance: 126435860,
                }
            );

            let original_creation_cost = 126435860;

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let seller_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected that identity exists");

            // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
            assert_eq!(
                seller_balance,
                dash_to_credits!(0.1) - original_creation_cost
            );

            let sender_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", identity.id());

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", purchaser.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.1),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().storage_fee, 216000); // we added 8 bytes for the price

            assert_eq!(
                processing_result
                    .aggregated_fees()
                    .fee_refunds
                    .calculate_refunds_amount_for_identity(identity.id()),
                None
            );

            assert_eq!(processing_result.aggregated_fees().processing_fee, 2473880);

            let seller_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected that identity exists");

            // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
            assert_eq!(
                seller_balance,
                dash_to_credits!(0.1) - original_creation_cost - 2689880
            );

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

            // We expect the sender to still have their document, and the receiver to have none
            assert_eq!(query_sender_results.documents().len(), 1);

            assert_eq!(query_receiver_results.documents().len(), 0);

            // The sender document should have the desired price

            let mut document = query_sender_results.documents_owned().remove(0);

            let price: Credits = document
                .properties()
                .get_integer("$price")
                .expect("expected to get back price");

            assert_eq!(dash_to_credits!(0.1), price);

            // At this point we want to have the receiver purchase the document

            document.set_revision(Some(3));

            let documents_batch_purchase_transition =
                DocumentsBatchTransition::new_document_purchase_transition_from_document(
                    document.clone(),
                    card_document_type,
                    purchaser.id(),
                    dash_to_credits!(0.1), //same price as requested
                    &recipient_key,
                    1, // 1 because he's never done anything
                    0,
                    &recipient_signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the purchase");

            let documents_batch_purchase_serialized_transition =
                documents_batch_purchase_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_purchase_serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(50000000),
                    &transaction,
                    platform_version,
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().storage_fee, 64611000);

            assert_eq!(processing_result.aggregated_fees().processing_fee, 4080480);

            assert_eq!(
                processing_result
                    .aggregated_fees()
                    .fee_refunds
                    .calculate_refunds_amount_for_identity(identity.id()),
                Some(22704503)
            );

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

            let seller_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected that identity exists");

            // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
            assert_eq!(
                seller_balance,
                dash_to_credits!(0.2) - original_creation_cost + 20014623
            );

            let buyers_balance = platform
                .drive
                .fetch_identity_balance(purchaser.id().to_buffer(), None, platform_version)
                .expect("expected to get purchaser balance")
                .expect("expected that purchaser exists");

            // the buyer payed 0.1, but also storage and processing fees
            assert_eq!(buyers_balance, dash_to_credits!(0.9) - 68691480);
        }

        #[test]
        fn test_document_set_price_and_try_purchase_at_different_amount() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_nft(TradeMode::DirectPurchase);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (purchaser, recipient_signer, recipient_key) =
                setup_identity(&mut platform, 450, dash_to_credits!(1.0));

            let seller_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected that identity exists");

            assert_eq!(seller_balance, dash_to_credits!(0.1));

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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            document.set_revision(Some(2));

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.5),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 1);

            // At this point we want to have the receiver purchase the document

            document.set_revision(Some(3));

            let documents_batch_purchase_transition =
                DocumentsBatchTransition::new_document_purchase_transition_from_document(
                    document.clone(),
                    card_document_type,
                    purchaser.id(),
                    dash_to_credits!(0.35), //different than requested price
                    &recipient_key,
                    1, // 1 because he's never done anything
                    0,
                    &recipient_signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the purchase");

            let documents_batch_purchase_serialized_transition =
                documents_batch_purchase_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_purchase_serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(50000000),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.invalid_paid_count(), 1);

            let result = processing_result.into_execution_results().remove(0);

            let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result
            else {
                panic!("expected a paid consensus error");
            };
            assert_eq!(consensus_error.to_string(), "5rJccTdtJfg6AxSKyrptWUug3PWjveEitTTLqBn9wHdk document can not be purchased for 35000000000, it's sale price is 50000000000 (in credits)");
        }

        #[test]
        fn test_document_set_price_and_purchase_from_ones_self() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_nft(TradeMode::DirectPurchase);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

            let seller_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected that identity exists");

            assert_eq!(seller_balance, dash_to_credits!(0.5));

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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            document.set_revision(Some(2));

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.1),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 1);

            // At this point we want to have the receiver purchase the document

            document.set_revision(Some(3));

            let documents_batch_purchase_transition =
                DocumentsBatchTransition::new_document_purchase_transition_from_document(
                    document.clone(),
                    card_document_type,
                    identity.id(),
                    dash_to_credits!(0.1), //same price as requested
                    &key,
                    1, // 1 because he's never done anything
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the purchase");

            let documents_batch_purchase_serialized_transition =
                documents_batch_purchase_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_purchase_serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(50000000),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.invalid_paid_count(), 1);

            let result = processing_result.into_execution_results().remove(0);

            let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result
            else {
                panic!("expected a paid consensus error");
            };
            assert_eq!(consensus_error.to_string(), "Document transition action on document type: card identity trying to purchase a document that is already owned by the purchaser is not supported");
        }

        #[test]
        fn test_document_set_price_and_purchase_then_try_buy_back() {
            // In this test we try to buy back a document after it has been sold

            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_nft(TradeMode::DirectPurchase);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (purchaser, recipient_signer, recipient_key) =
                setup_identity(&mut platform, 450, dash_to_credits!(1.0));

            let seller_balance = platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected that identity exists");

            assert_eq!(seller_balance, dash_to_credits!(0.1));

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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            document.set_revision(Some(2));

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.1),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 1);

            // At this point we want to have the receiver purchase the document

            document.set_revision(Some(3));

            let documents_batch_purchase_transition =
                DocumentsBatchTransition::new_document_purchase_transition_from_document(
                    document.clone(),
                    card_document_type,
                    purchaser.id(),
                    dash_to_credits!(0.1), //same price as requested
                    &recipient_key,
                    1, // 1 because he's never done anything
                    0,
                    &recipient_signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the purchase");

            let documents_batch_purchase_serialized_transition =
                documents_batch_purchase_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_purchase_serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(50000000),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 1);

            // Let's verify some stuff

            let sender_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", identity.id());

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", purchaser.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            // We expect the sender to still have their document, and the receiver to have none
            assert_eq!(query_sender_results.documents().len(), 0);

            assert_eq!(query_receiver_results.documents().len(), 1);

            // The sender document should have the desired price

            let mut document = query_receiver_results.documents_owned().remove(0);

            let price: Option<Credits> = document
                .properties()
                .get_optional_integer("$price")
                .expect("expected to get back price");

            assert_eq!(price, None);

            assert_eq!(document.owner_id(), purchaser.id());

            // At this point we want to have the sender to try to buy back the document

            document.set_revision(Some(4));

            let documents_batch_purchase_transition =
                DocumentsBatchTransition::new_document_purchase_transition_from_document(
                    document.clone(),
                    card_document_type,
                    identity.id(),
                    dash_to_credits!(0.1), //same price as old requested
                    &key,
                    4, // 1 because he's never done anything
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the purchase");

            let documents_batch_purchase_serialized_transition =
                documents_batch_purchase_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_purchase_serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(50000000),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.invalid_paid_count(), 1);

            let result = processing_result.into_execution_results().remove(0);

            let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result
            else {
                panic!("expected a paid consensus error");
            };
            assert_eq!(
                consensus_error.to_string(),
                "5rJccTdtJfg6AxSKyrptWUug3PWjveEitTTLqBn9wHdk document not for sale"
            );
        }

        #[test]
        fn test_document_set_price_and_purchase_with_enough_credits_to_buy_but_not_enough_to_pay_for_processing(
        ) {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_nft(TradeMode::DirectPurchase);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (receiver, recipient_signer, recipient_key) =
                setup_identity(&mut platform, 450, dash_to_credits!(0.1));

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
                    false,
                    None,
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

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let receiver_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", receiver.id());

            let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
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

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.1),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 2473880);

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

            // We expect the sender to still have their document, and the receiver to have none
            assert_eq!(query_sender_results.documents().len(), 1);

            assert_eq!(query_receiver_results.documents().len(), 0);

            // The sender document should have the desired price

            let mut document = query_sender_results.documents_owned().remove(0);

            let price: Credits = document
                .properties()
                .get_integer("$price")
                .expect("expected to get back price");

            assert_eq!(dash_to_credits!(0.1), price);

            // At this point we want to have the receiver purchase the document

            document.set_revision(Some(3));

            let documents_batch_purchase_transition =
                DocumentsBatchTransition::new_document_purchase_transition_from_document(
                    document.clone(),
                    card_document_type,
                    receiver.id(),
                    dash_to_credits!(0.1), //same price as requested
                    &recipient_key,
                    1, // 1 because he's never done anything
                    0,
                    &recipient_signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the purchase");

            let documents_batch_purchase_serialized_transition =
                documents_batch_purchase_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_purchase_serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(50000000),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // nothing can go through because the purchaser doesn't have enough balance

            assert_eq!(processing_result.invalid_paid_count(), 0);

            assert_eq!(processing_result.invalid_unpaid_count(), 1);

            assert_eq!(processing_result.valid_count(), 0);

            assert_eq!(processing_result.aggregated_fees().processing_fee, 0);
        }

        #[test]
        fn test_document_set_price_on_not_owned_document() {
            let platform_version = PlatformVersion::latest();
            let (mut platform, contract) = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure()
                .with_crypto_card_game_nft(TradeMode::DirectPurchase);

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

            let (other_identity, other_identity_signer, other_identity_key) =
                setup_identity(&mut platform, 450, dash_to_credits!(0.1));

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
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            document.set_revision(Some(2));

            document.set_owner_id(other_identity.id()); // we do this to trick the system

            let documents_batch_update_price_transition =
                DocumentsBatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    card_document_type,
                    dash_to_credits!(0.1),
                    &other_identity_key,
                    1,
                    0,
                    &other_identity_signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition for the update price");

            let documents_batch_transfer_serialized_transition =
                documents_batch_update_price_transition
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
                    false,
                    None,
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

            assert_eq!(processing_result.aggregated_fees().processing_fee, 36200);

            let sender_documents_sql_string =
                format!("select * from card where $ownerId == '{}'", identity.id());

            let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
                sender_documents_sql_string.as_str(),
                &contract,
                Some(&platform.config.drive),
            )
            .expect("expected document query");

            let query_sender_results = platform
                .drive
                .query_documents(query_sender_identity_documents, None, false, None, None)
                .expect("expected query result");

            // The sender document should not have the desired price

            let document = query_sender_results.documents().first().unwrap();

            assert_eq!(
                document
                    .properties()
                    .get_optional_integer::<u64>("$price")
                    .expect("expected None"),
                None
            );
        }
    }

    mod dpns_tests {
        use super::*;
        use crate::execution::validation::state_transition::tests::setup_identity;
        use crate::test::helpers::setup::TestPlatformBuilder;
        use dpp::dash_to_credits;
        use dpp::data_contract::document_type::random_document::{
            DocumentFieldFillSize, DocumentFieldFillType,
        };
        use dpp::platform_value::Bytes32;
        use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
        use dpp::util::hash::hash_double;
        use drive::query::{InternalClauses, OrderClause, WhereClause, WhereOperator};
        use drive::util::test_helpers::setup_contract;
        use indexmap::IndexMap;
        use platform_version::version::PlatformVersion;
        use rand::prelude::StdRng;
        use rand::Rng;
        use std::collections::BTreeMap;

        #[test]
        fn test_dpns_contract_references_with_no_contested_unique_index() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity_1, signer_1, key_1) =
                setup_identity(&mut platform, 958, dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, 93, dash_to_credits!(0.5));

            let (identity_3, signer_3, key_3) =
                setup_identity(&mut platform, 98, dash_to_credits!(0.5));

            let dashpay_contract = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
                None,
                None,
            );

            let card_game = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json",
                None,
                None,
            );

            let dpns_contract = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-contract-id.json",
                None,
                None,
            );

            let preorder = dpns_contract
                .document_type_for_name("preorder")
                .expect("expected a profile document type");

            assert!(!preorder.documents_mutable());
            assert!(preorder.documents_can_be_deleted());
            assert!(!preorder.documents_transferable().is_transferable());

            let domain = dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type");

            assert!(!domain.documents_mutable());
            // Deletion is disabled with data trigger
            assert!(domain.documents_can_be_deleted());
            assert!(domain.documents_transferable().is_transferable());

            let entropy = Bytes32::random_with_rng(&mut rng);

            let mut preorder_document_1 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut preorder_document_2 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut preorder_document_3 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_3.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_1 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_2 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_3 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_3.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            document_1.set("parentDomainName", "dash".into());
            document_1.set("normalizedParentDomainName", "dash".into());
            document_1.set("label", "quantum123".into());
            document_1.set("normalizedLabel", "quantum123".into());
            document_1.set("records.contract", dashpay_contract.id().into());
            document_1.set("subdomainRules.allowSubdomains", false.into());

            document_2.set("parentDomainName", "dash".into());
            document_2.set("normalizedParentDomainName", "dash".into());
            document_2.set("label", "van89".into());
            document_2.set("normalizedLabel", "van89".into());
            document_2.set("records.contract", card_game.id().into());
            document_2.set("subdomainRules.allowSubdomains", false.into());

            document_3.set("parentDomainName", "dash".into());
            document_3.set("normalizedParentDomainName", "dash".into());
            document_3.set("label", "jazz65".into());
            document_3.set("normalizedLabel", "jazz65".into());
            document_3.set("records.identity", document_3.owner_id().into());
            document_3.set("subdomainRules.allowSubdomains", false.into());

            let salt_1: [u8; 32] = rng.gen();
            let salt_2: [u8; 32] = rng.gen();
            let salt_3: [u8; 32] = rng.gen();

            let mut salted_domain_buffer_1: Vec<u8> = vec![];
            salted_domain_buffer_1.extend(salt_1);
            salted_domain_buffer_1.extend("quantum123.dash".as_bytes());

            let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

            let mut salted_domain_buffer_2: Vec<u8> = vec![];
            salted_domain_buffer_2.extend(salt_2);
            salted_domain_buffer_2.extend("van89.dash".as_bytes());

            let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

            let mut salted_domain_buffer_3: Vec<u8> = vec![];
            salted_domain_buffer_3.extend(salt_3);
            salted_domain_buffer_3.extend("jazz65.dash".as_bytes());

            let salted_domain_hash_3 = hash_double(salted_domain_buffer_3);

            preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
            preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());
            preorder_document_3.set("saltedDomainHash", salted_domain_hash_3.into());

            document_1.set("preorderSalt", salt_1.into());
            document_2.set("preorderSalt", salt_2.into());
            document_3.set("preorderSalt", salt_3.into());

            let documents_batch_create_preorder_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_1,
                    preorder,
                    entropy.0,
                    &key_1,
                    2,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_1 =
                documents_batch_create_preorder_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_preorder_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_2,
                    preorder,
                    entropy.0,
                    &key_2,
                    2,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_2 =
                documents_batch_create_preorder_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_preorder_transition_3 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_3,
                    preorder,
                    entropy.0,
                    &key_3,
                    2,
                    0,
                    &signer_3,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_3 =
                documents_batch_create_preorder_transition_3
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_1,
                    domain,
                    entropy.0,
                    &key_1,
                    3,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_1 =
                documents_batch_create_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_2,
                    domain,
                    entropy.0,
                    &key_2,
                    3,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_2 =
                documents_batch_create_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_3 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_3.clone(),
                    domain,
                    entropy.0,
                    &key_3,
                    3,
                    0,
                    &signer_3,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_3 =
                documents_batch_create_transition_3
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_preorder_transition_1.clone(),
                        documents_batch_create_serialized_preorder_transition_2.clone(),
                        documents_batch_create_serialized_preorder_transition_3.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 3);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_transition_1.clone(),
                        documents_batch_create_serialized_transition_2.clone(),
                        documents_batch_create_serialized_transition_3.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 3);

            let mut order_by = IndexMap::new();

            order_by.insert(
                "records.identity".to_string(),
                OrderClause {
                    field: "records.identity".to_string(),
                    ascending: true,
                },
            );

            let drive_query = DriveDocumentQuery {
                contract: &dpns_contract,
                document_type: domain,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: None,
                    in_clause: None,
                    range_clause: Some(WhereClause {
                        field: "records.identity".to_string(),
                        operator: WhereOperator::LessThanOrEquals,
                        value: Value::Bytes32([255; 32]),
                    }),
                    equal_clauses: Default::default(),
                },
                offset: None,
                limit: None,
                order_by,
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };

            let documents = platform
                .drive
                .query_documents(drive_query, None, false, None, None)
                .expect("expected to get back documents")
                .documents_owned();

            let transient_fields = domain
                .transient_fields()
                .iter()
                .map(|a| a.as_str())
                .collect();

            assert!(documents
                .get(0)
                .expect("expected a document")
                .is_equal_ignoring_time_based_fields(
                    &document_3,
                    Some(transient_fields),
                    platform_version
                )
                .expect("expected to run is equal"));

            let drive_query = DriveDocumentQuery {
                contract: &dpns_contract,
                document_type: domain,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: None,
                    in_clause: None,
                    range_clause: None,
                    equal_clauses: BTreeMap::from([(
                        "records.identity".to_string(),
                        WhereClause {
                            field: "records.identity".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Null,
                        },
                    )]),
                },
                offset: None,
                limit: None,
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };

            let documents = platform
                .drive
                .query_documents(drive_query, None, false, None, None)
                .expect("expected to get back documents")
                .documents_owned();

            // This is normal because we set that we could not query on null
            assert_eq!(documents.len(), 0);
        }

        #[test]
        fn test_dpns_contract_references_with_no_contested_unique_index_null_searchable_true() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity_1, signer_1, key_1) =
                setup_identity(&mut platform, 958, dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, 93, dash_to_credits!(0.5));

            let (identity_3, signer_3, key_3) =
                setup_identity(&mut platform, 98, dash_to_credits!(0.5));

            let dashpay_contract = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
                None,
                None,
            );

            let card_game = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json",
                None,
                None,
            );

            let dpns_contract = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-contract-id-null-searchable-true.json",
                None,
                None,
            );

            let preorder = dpns_contract
                .document_type_for_name("preorder")
                .expect("expected a profile document type");

            assert!(!preorder.documents_mutable());
            assert!(preorder.documents_can_be_deleted());
            assert!(!preorder.documents_transferable().is_transferable());

            let domain = dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type");

            assert!(!domain.documents_mutable());
            // Deletion is disabled with data trigger
            assert!(domain.documents_can_be_deleted());
            assert!(domain.documents_transferable().is_transferable());

            let entropy = Bytes32::random_with_rng(&mut rng);

            let mut preorder_document_1 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut preorder_document_2 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut preorder_document_3 = preorder
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_3.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_1 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_1.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_2 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_2.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            let mut document_3 = domain
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity_3.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");

            document_1.set("parentDomainName", "dash".into());
            document_1.set("normalizedParentDomainName", "dash".into());
            document_1.set("label", "quantum123".into());
            document_1.set("normalizedLabel", "quantum123".into());
            document_1.set("records.contract", dashpay_contract.id().into());
            document_1.set("subdomainRules.allowSubdomains", false.into());

            document_2.set("parentDomainName", "dash".into());
            document_2.set("normalizedParentDomainName", "dash".into());
            document_2.set("label", "van89".into());
            document_2.set("normalizedLabel", "van89".into());
            document_2.set("records.contract", card_game.id().into());
            document_2.set("subdomainRules.allowSubdomains", false.into());

            document_3.set("parentDomainName", "dash".into());
            document_3.set("normalizedParentDomainName", "dash".into());
            document_3.set("label", "jazz65".into());
            document_3.set("normalizedLabel", "jazz65".into());
            document_3.set("records.identity", document_3.owner_id().into());
            document_3.set("subdomainRules.allowSubdomains", false.into());

            let salt_1: [u8; 32] = rng.gen();
            let salt_2: [u8; 32] = rng.gen();
            let salt_3: [u8; 32] = rng.gen();

            let mut salted_domain_buffer_1: Vec<u8> = vec![];
            salted_domain_buffer_1.extend(salt_1);
            salted_domain_buffer_1.extend("quantum123.dash".as_bytes());

            let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

            let mut salted_domain_buffer_2: Vec<u8> = vec![];
            salted_domain_buffer_2.extend(salt_2);
            salted_domain_buffer_2.extend("van89.dash".as_bytes());

            let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

            let mut salted_domain_buffer_3: Vec<u8> = vec![];
            salted_domain_buffer_3.extend(salt_3);
            salted_domain_buffer_3.extend("jazz65.dash".as_bytes());

            let salted_domain_hash_3 = hash_double(salted_domain_buffer_3);

            preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
            preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());
            preorder_document_3.set("saltedDomainHash", salted_domain_hash_3.into());

            document_1.set("preorderSalt", salt_1.into());
            document_2.set("preorderSalt", salt_2.into());
            document_3.set("preorderSalt", salt_3.into());

            let documents_batch_create_preorder_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_1,
                    preorder,
                    entropy.0,
                    &key_1,
                    2,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_1 =
                documents_batch_create_preorder_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_preorder_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_2,
                    preorder,
                    entropy.0,
                    &key_2,
                    2,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_2 =
                documents_batch_create_preorder_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_preorder_transition_3 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    preorder_document_3,
                    preorder,
                    entropy.0,
                    &key_3,
                    2,
                    0,
                    &signer_3,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_preorder_transition_3 =
                documents_batch_create_preorder_transition_3
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_1 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_1,
                    domain,
                    entropy.0,
                    &key_1,
                    3,
                    0,
                    &signer_1,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_1 =
                documents_batch_create_transition_1
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_2 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_2,
                    domain,
                    entropy.0,
                    &key_2,
                    3,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_2 =
                documents_batch_create_transition_2
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let documents_batch_create_transition_3 =
                DocumentsBatchTransition::new_document_creation_transition_from_document(
                    document_3.clone(),
                    domain,
                    entropy.0,
                    &key_3,
                    3,
                    0,
                    &signer_3,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition_3 =
                documents_batch_create_transition_3
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_preorder_transition_1.clone(),
                        documents_batch_create_serialized_preorder_transition_2.clone(),
                        documents_batch_create_serialized_preorder_transition_3.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 3);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_transition_1.clone(),
                        documents_batch_create_serialized_transition_2.clone(),
                        documents_batch_create_serialized_transition_3.clone(),
                    ],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.valid_count(), 3);

            let mut order_by = IndexMap::new();

            order_by.insert(
                "records.identity".to_string(),
                OrderClause {
                    field: "records.identity".to_string(),
                    ascending: true,
                },
            );

            let drive_query = DriveDocumentQuery {
                contract: &dpns_contract,
                document_type: domain,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: None,
                    in_clause: None,
                    range_clause: Some(WhereClause {
                        field: "records.identity".to_string(),
                        operator: WhereOperator::LessThanOrEquals,
                        value: Value::Bytes32([255; 32]),
                    }),
                    equal_clauses: Default::default(),
                },
                offset: None,
                limit: None,
                order_by,
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };

            let documents = platform
                .drive
                .query_documents(drive_query, None, false, None, None)
                .expect("expected to get back documents")
                .documents_owned();

            // here we will get all 3 documents
            assert_eq!(documents.len(), 3);

            let drive_query = DriveDocumentQuery {
                contract: &dpns_contract,
                document_type: domain,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: None,
                    in_clause: None,
                    range_clause: None,
                    equal_clauses: BTreeMap::from([(
                        "records.identity".to_string(),
                        WhereClause {
                            field: "records.identity".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Null,
                        },
                    )]),
                },
                offset: None,
                limit: None,
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };

            let documents = platform
                .drive
                .query_documents(drive_query, None, false, None, None)
                .expect("expected to get back documents")
                .documents_owned();

            assert_eq!(documents.len(), 2);
        }
    }
}
