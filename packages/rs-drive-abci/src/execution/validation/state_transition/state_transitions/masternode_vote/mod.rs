mod balance;
mod nonce;
mod state;
mod structure;

use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::masternode_vote::state::v0::MasternodeVoteStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionStateValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for MasternodeVoteTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .masternode_vote_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "masternode votes state transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for MasternodeVoteTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        _epoch: &Epoch,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .masternode_vote_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "masternode votes state transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::platform_value::{Bytes32, Value};
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, get_vote_polls_by_end_date_request, get_vote_polls_by_end_date_response, GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse, GetVotePollsByEndDateRequest, GetVotePollsByEndDateResponse};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
    use dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::GetVotePollsByEndDateRequestV0;
    use dapi_grpc::platform::v0::get_vote_polls_by_end_date_response::{get_vote_polls_by_end_date_response_v0, GetVotePollsByEndDateResponseV0};
    use dapi_grpc::platform::v0::get_vote_polls_by_end_date_response::get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamp;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use dpp::voting::votes::Vote;
    use drive::drive::object_size_info::DataContractResolvedInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
    use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally;
    use drive::query::vote_poll_vote_state_query::ResolvedContestedDocumentVotePollDriveQuery;

    mod vote_tests {
        use std::collections::BTreeMap;
        use std::sync::Arc;
        use arc_swap::Guard;
        use rand::Rng;
        use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, get_contested_resources_request, get_contested_resources_response, get_vote_polls_by_end_date_request, get_vote_polls_by_end_date_response, GetContestedResourcesRequest, GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse, GetVotePollsByEndDateRequest, GetVotePollsByEndDateResponse};
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
        use dapi_grpc::platform::v0::get_contested_resources_request::GetContestedResourcesRequestV0;
        use dapi_grpc::platform::v0::get_contested_resources_response::{get_contested_resources_response_v0, GetContestedResourcesResponseV0};
        use dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::GetVotePollsByEndDateRequestV0;
        use dapi_grpc::platform::v0::get_vote_polls_by_end_date_response::{get_vote_polls_by_end_date_response_v0, GetVotePollsByEndDateResponseV0};
        use dapi_grpc::platform::v0::get_vote_polls_by_end_date_response::get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamp;
        use super::*;
        use dpp::document::Document;
        use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        use dpp::identifier::Identifier;
        use dpp::identity::{IdentityPublicKey, IdentityV0};
        use dpp::prelude::{DataContract, Identity};
        use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
        use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
        use dpp::util::hash::hash_double;
        use dpp::util::strings::convert_to_homograph_safe_chars;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
        use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use dpp::voting::vote_polls::VotePoll;
        use dpp::voting::votes::resource_vote::ResourceVote;
        use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dpp::voting::votes::Vote;
        use drive::drive::object_size_info::DataContractResolvedInfo;
        use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
        use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally;
        use drive::query::vote_poll_vote_state_query::ResolvedContestedDocumentVotePollDriveQuery;
        use drive::query::vote_polls_by_document_type_query::ResolvedVotePollsByDocumentTypeQuery;
        use simple_signer::signer::SimpleSigner;
        use crate::platform_types::platform_state::PlatformState;
        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::TempPlatform;
        use dpp::serialization::PlatformDeserializable;
        use drive::query::VotePollsByEndDateDriveQuery;
        use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
        use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
        use dpp::platform_value::IdentifierBytes32;
        use dpp::platform_value::Value::Text;

        fn setup_masternode_identity(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            seed: u64,
            platform_version: &PlatformVersion,
        ) -> (Identity, SimpleSigner, IdentityPublicKey) {
            let mut signer = SimpleSigner::default();

            let mut rng = StdRng::seed_from_u64(seed);

            let (voting_key, voting_private_key) =
                IdentityPublicKey::random_voting_key_with_rng(0, &mut rng, platform_version)
                    .expect("expected to get key pair");

            signer.add_key(voting_key.clone(), voting_private_key.clone());

            let identity: Identity = IdentityV0 {
                id: Identifier::random_with_rng(&mut rng),
                public_keys: BTreeMap::from([(0, voting_key.clone())]),
                balance: 0,
                revision: 0,
            }
            .into();

            // We just add this identity to the system first

            platform
                .drive
                .add_new_identity(
                    identity.clone(),
                    true,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add a new identity");

            (identity, signer, voting_key)
        }

        fn create_dpns_name_contest(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            platform_state: &PlatformState,
            seed: u64,
            name: &str,
            platform_version: &PlatformVersion,
        ) -> (Identity, Identity, Arc<DataContract>) {
            let mut rng = StdRng::seed_from_u64(seed);

            let (identity_1, signer_1, key_1) =
                setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

            // Flip them if needed so identity 1 id is always smaller than identity 2 id
            let (identity_1, identity_2, signer_1, signer_2, key_1, key_2) =
                if identity_1.id() < identity_2.id() {
                    (identity_1, identity_2, signer_1, signer_2, key_1, key_2)
                } else {
                    (identity_2, identity_1, signer_2, signer_1, key_2, key_1)
                };

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
            assert!(!domain.documents_can_be_deleted());
            assert!(!domain.documents_transferable().is_transferable());

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
            document_1.set("label", name.into());
            document_1.set(
                "normalizedLabel",
                convert_to_homograph_safe_chars(name).into(),
            );
            document_1.set("records.dashUniqueIdentityId", document_1.owner_id().into());
            document_1.set("subdomainRules.allowSubdomains", false.into());

            document_2.set("parentDomainName", "dash".into());
            document_2.set("normalizedParentDomainName", "dash".into());
            document_2.set("label", name.into());
            document_2.set(
                "normalizedLabel",
                convert_to_homograph_safe_chars(name).into(),
            );
            document_2.set("records.dashUniqueIdentityId", document_2.owner_id().into());
            document_2.set("subdomainRules.allowSubdomains", false.into());

            let salt_1: [u8; 32] = rng.gen();
            let salt_2: [u8; 32] = rng.gen();

            let mut salted_domain_buffer_1: Vec<u8> = vec![];
            salted_domain_buffer_1.extend(salt_1);
            salted_domain_buffer_1
                .extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());

            let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

            let mut salted_domain_buffer_2: Vec<u8> = vec![];
            salted_domain_buffer_2.extend(salt_2);
            salted_domain_buffer_2
                .extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());

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
                    platform_state,
                    &BlockInfo::default_with_time(
                        platform_state
                            .last_committed_block_time_ms()
                            .unwrap_or_default()
                            + 3000,
                    ),
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

            assert_eq!(processing_result.valid_count(), 2);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![
                        documents_batch_create_serialized_transition_1.clone(),
                        documents_batch_create_serialized_transition_2.clone(),
                    ],
                    platform_state,
                    &BlockInfo::default_with_time(
                        platform_state
                            .last_committed_block_time_ms()
                            .unwrap_or_default()
                            + 3000,
                    ),
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

            assert_eq!(processing_result.valid_count(), 2);
            (identity_1, identity_2, dpns_contract)
        }

        fn verify_dpns_name_contest(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            platform_state: &Guard<Arc<PlatformState>>,
            dpns_contract: &DataContract,
            identity_1: &Identity,
            identity_2: &Identity,
            name: &str,
            platform_version: &PlatformVersion,
        ) {
            // Now let's run a query for the vote totals

            let domain = dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type");

            let config = bincode::config::standard()
                .with_big_endian()
                .with_no_limit();

            let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
                .expect("expected to encode the word dash");

            let quantum_encoded =
                bincode::encode_to_vec(Value::Text(convert_to_homograph_safe_chars(name)), config)
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
                                start_at_identifier_info: None,
                                count: None,
                                order_ascending: true,
                                prove: false,
                            },
                        )),
                    },
                    platform_state,
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
                    },
                ),
            ) = result
            else {
                panic!("expected contenders")
            };

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
                                start_at_identifier_info: None,
                                count: None,
                                order_ascending: true,
                                prove: true,
                            },
                        )),
                    },
                    platform_state,
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
                        contract: DataContractResolvedInfo::BorrowedDataContract(dpns_contract),
                        document_type_name: domain.name().clone(),
                        index_name: index_name.clone(),
                        index_values: vec![
                            Value::Text("dash".to_string()),
                            Value::Text(convert_to_homograph_safe_chars(name)),
                        ],
                    },
                    result_type: DocumentsAndVoteTally,
                    offset: None,
                    limit: None,
                    start_at: None,
                    order_ascending: true,
                };

            let (_, contenders) = resolved_contested_document_vote_poll_drive_query
                .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
                .expect("expected to verify proof");

            assert_eq!(contenders.len(), 2);

            let first_contender = contenders.first().unwrap();

            let second_contender = contenders.last().unwrap();

            let first_contender_document = Document::from_bytes(
                first_contender
                    .serialized_document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            let second_contender_document = Document::from_bytes(
                second_contender
                    .serialized_document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            assert_ne!(first_contender_document, second_contender_document);

            assert_eq!(first_contender.identity_id, identity_1.id());

            assert_eq!(second_contender.identity_id, identity_2.id());

            assert_eq!(first_contender.vote_tally, Some(0));

            assert_eq!(second_contender.vote_tally, Some(0));
        }

        mod contests_requests {
            use super::*;
            #[test]
            fn test_contests_request() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity_1, identity_2, dpns_contract) = create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                verify_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    dpns_contract.as_ref(),
                    &identity_1,
                    &identity_2,
                    "quantum",
                    platform_version,
                );

                let (identity_3, identity_4, dpns_contract) = create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "cooldog",
                    platform_version,
                );

                verify_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    dpns_contract.as_ref(),
                    &identity_3,
                    &identity_4,
                    "cooldog",
                    platform_version,
                );

                let domain = dpns_contract
                    .document_type_for_name("domain")
                    .expect("expected a profile document type");

                let index_name = "parentNameAndLabel".to_string();

                let config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();

                let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
                    .expect("expected to encode value");

                let query_validation_result = platform
                    .query_contested_resources(
                        GetContestedResourcesRequest {
                            version: Some(get_contested_resources_request::Version::V0(
                                GetContestedResourcesRequestV0 {
                                    contract_id: dpns_contract.id().to_vec(),
                                    document_type_name: domain.name().clone(),
                                    index_name: index_name.clone(),
                                    start_index_values: vec![dash_encoded.clone()],
                                    end_index_values: vec![],
                                    start_at_value_info: None,
                                    count: None,
                                    order_ascending: true,
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

                let get_contested_resources_response::Version::V0(
                    GetContestedResourcesResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = query_validation_result.version.expect("expected a version");

                let Some(get_contested_resources_response_v0::Result::ContestedResourceValues(
                    get_contested_resources_response_v0::ContestedResourceValues {
                        contested_resource_values,
                    },
                )) = result
                else {
                    panic!("expected contested resources")
                };

                assert_eq!(contested_resource_values.len(), 2);

                let query_validation_result = platform
                    .query_contested_resources(
                        GetContestedResourcesRequest {
                            version: Some(get_contested_resources_request::Version::V0(
                                GetContestedResourcesRequestV0 {
                                    contract_id: dpns_contract.id().to_vec(),
                                    document_type_name: domain.name().clone(),
                                    index_name: index_name.clone(),
                                    start_index_values: vec![dash_encoded],
                                    end_index_values: vec![],
                                    start_at_value_info: None,
                                    count: None,
                                    order_ascending: true,
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

                let get_contested_resources_response::Version::V0(
                    GetContestedResourcesResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = query_validation_result.version.expect("expected a version");

                let Some(get_contested_resources_response_v0::Result::Proof(proof)) = result else {
                    panic!("expected proof")
                };

                let resolved_contested_document_vote_poll_drive_query =
                    ResolvedVotePollsByDocumentTypeQuery {
                        contract: DataContractResolvedInfo::BorrowedDataContract(
                            dpns_contract.as_ref(),
                        ),
                        document_type_name: domain.name(),
                        index_name: &index_name,
                        start_index_values: &vec!["dash".into()],
                        end_index_values: &vec![],
                        limit: None,
                        order_ascending: true,
                        start_at_value: &None,
                    };

                let (_, contests) = resolved_contested_document_vote_poll_drive_query
                    .verify_contests_proof(proof.grovedb_proof.as_ref(), platform_version)
                    .expect("expected to verify proof");

                assert_eq!(contests.len(), 2);
            }
        }

        #[test]
        fn test_masternode_voting_and_vote_state_request() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (contender_1, contender_2, dpns_contract) = create_dpns_name_contest(
                &mut platform,
                &platform_state,
                7,
                "quantum",
                platform_version,
            );

            let (masternode_1, signer_1, voting_key_1) =
                setup_masternode_identity(&mut platform, 29, platform_version);

            // Let's vote for contender 1

            let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                    ContestedDocumentResourceVotePoll {
                        contract_id: dpns_contract.id(),
                        document_type_name: "domain".to_string(),
                        index_name: "parentNameAndLabel".to_string(),
                        index_values: vec![
                            Value::Text("dash".to_string()),
                            Value::Text("quantum".to_string()),
                        ],
                    },
                ),
                resource_vote_choice: TowardsIdentity(contender_1.id()),
            }));

            let masternode_vote_transition = MasternodeVoteTransition::try_from_vote_with_signer(
                vote,
                &signer_1,
                masternode_1.id(),
                &voting_key_1,
                1,
                platform_version,
                None,
            )
            .expect("expected to make transition vote");

            let masternode_vote_serialized_transition = masternode_vote_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![masternode_vote_serialized_transition.clone()],
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

            assert_eq!(processing_result.valid_count(), 1);

            // Now let's run a query for the vote totals

            let domain = dpns_contract
                .document_type_for_name("domain")
                .expect("expected a profile document type");

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
                                start_at_identifier_info: None,
                                count: None,
                                order_ascending: true,
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
                    },
                ),
            ) = result
            else {
                panic!("expected contenders")
            };

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

            assert_eq!(first_contender.identifier, contender_1.id().to_vec());

            assert_eq!(second_contender.identifier, contender_2.id().to_vec());

            assert_eq!(first_contender.vote_count, Some(1));

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
                                start_at_identifier_info: None,
                                count: None,
                                order_ascending: true,
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
                    order_ascending: true,
                };

            let (_, contenders) = resolved_contested_document_vote_poll_drive_query
                .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
                .expect("expected to verify proof");

            assert_eq!(contenders.len(), 2);

            let first_contender = contenders.first().unwrap();

            let second_contender = contenders.last().unwrap();

            let first_contender_document = Document::from_bytes(
                first_contender
                    .serialized_document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            let second_contender_document = Document::from_bytes(
                second_contender
                    .serialized_document
                    .as_ref()
                    .expect("expected a document")
                    .as_slice(),
                domain,
                platform_version,
            )
            .expect("expected to get document");

            assert_ne!(first_contender_document, second_contender_document);

            assert_eq!(first_contender.identity_id, contender_1.id());

            assert_eq!(second_contender.identity_id, contender_2.id());

            assert_eq!(first_contender.vote_tally, Some(1));

            assert_eq!(second_contender.vote_tally, Some(0));
        }

        mod end_date_query {
            use super::*;
            use dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::get_vote_polls_by_end_date_request_v0;

            #[test]
            fn test_not_proved_end_date_query_request() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: None,
                                    end_time_info: None,
                                    limit: None,
                                    offset: None,
                                    ascending: true,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::VotePollsByTimestamps(
                    get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamps {
                        vote_polls_by_timestamps,
                        finished_results,
                    },
                )) = result
                else {
                    panic!("expected contenders")
                };

                assert!(finished_results);

                let serialized_contested_vote_poll_bytes = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 7, 113, 117,
                    97, 110, 116, 117, 109,
                ];

                // The timestamp is 0 because there were no blocks
                assert_eq!(
                    vote_polls_by_timestamps,
                    vec![SerializedVotePollsByTimestamp {
                        timestamp: 1_209_603_000, // in ms, 2 weeks after Jan 1 1970
                        serialized_vote_polls: vec![serialized_contested_vote_poll_bytes.clone()]
                    }]
                );

                // Let's try deserializing

                let vote_poll = VotePoll::deserialize_from_bytes(
                    serialized_contested_vote_poll_bytes.as_slice(),
                )
                .expect("expected to deserialize");

                assert_eq!(
                    vote_poll,
                    VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: Identifier(IdentifierBytes32([
                                230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123,
                                91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83,
                                197, 49, 85
                            ])),
                            document_type_name: "domain".to_string(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![
                                Text("dash".to_string()),
                                Text("quantum".to_string())
                            ]
                        }
                    )
                );
            }

            #[test]
            fn test_proved_end_date_query_request() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: None,
                                    end_time_info: None,
                                    limit: None,
                                    offset: None,
                                    ascending: true,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::Proof(proof)) = result
                else {
                    panic!("expected contenders")
                };

                let vote_poll_by_end_date_query = VotePollsByEndDateDriveQuery {
                    start_time: None,
                    end_time: None,
                    offset: None,
                    limit: None,
                    order_ascending: true,
                };

                let (_, vote_polls_by_timestamps) = vote_poll_by_end_date_query
                    .verify_vote_polls_by_end_date_proof(
                        proof.grovedb_proof.as_ref(),
                        platform_version,
                    )
                    .expect("expected to verify proof");

                assert_eq!(
                    vote_polls_by_timestamps,
                    BTreeMap::from([(
                        1_209_603_000,
                        vec![VotePoll::ContestedDocumentResourceVotePoll(
                            ContestedDocumentResourceVotePoll {
                                contract_id: Identifier(IdentifierBytes32([
                                    230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222,
                                    123, 91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34,
                                    191, 83, 197, 49, 85
                                ])),
                                document_type_name: "domain".to_string(),
                                index_name: "parentNameAndLabel".to_string(),
                                index_values: vec![
                                    Text("dash".to_string()),
                                    Text("quantum".to_string())
                                ]
                            }
                        )]
                    )])
                );
            }

            #[test]
            fn test_not_proved_end_date_query_multiple_contests() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();
                let mut platform_state = (**platform_state).clone();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 500000,
                            height: 100,
                            core_height: 42,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create two new contenders, but we are on the same contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "quantum",
                    platform_version,
                );

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    9,
                    "coolio",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: None,
                                    end_time_info: None,
                                    limit: None,
                                    offset: None,
                                    ascending: true,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::VotePollsByTimestamps(
                    get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamps {
                        vote_polls_by_timestamps,
                        finished_results,
                    },
                )) = result
                else {
                    panic!("expected contenders")
                };

                assert!(finished_results);

                let serialized_contested_vote_poll_bytes_1 = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 7, 113, 117,
                    97, 110, 116, 117, 109,
                ];

                let serialized_contested_vote_poll_bytes_2 = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 6, 99, 48, 48,
                    49, 49, 48,
                ];

                // The timestamp is 0 because there were no blocks
                assert_eq!(
                    vote_polls_by_timestamps,
                    vec![
                        SerializedVotePollsByTimestamp {
                            timestamp: 1_209_603_000, // in ms, 2 weeks after Jan 1 1970 + 3 seconds (chosen block time in test)
                            serialized_vote_polls: vec![
                                serialized_contested_vote_poll_bytes_1.clone()
                            ]
                        },
                        SerializedVotePollsByTimestamp {
                            timestamp: 1_210_103_000, // in ms, 500 s after Jan 1 1970 + 3 seconds (chosen block time in test)
                            serialized_vote_polls: vec![
                                serialized_contested_vote_poll_bytes_2.clone()
                            ]
                        },
                    ]
                );

                // Let's try deserializing

                let vote_poll_1 = VotePoll::deserialize_from_bytes(
                    serialized_contested_vote_poll_bytes_1.as_slice(),
                )
                .expect("expected to deserialize");

                assert_eq!(
                    vote_poll_1,
                    VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: Identifier(IdentifierBytes32([
                                230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123,
                                91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83,
                                197, 49, 85
                            ])),
                            document_type_name: "domain".to_string(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![
                                Text("dash".to_string()),
                                Text("quantum".to_string())
                            ]
                        }
                    )
                );

                // Let's try deserializing

                let vote_poll_2 = VotePoll::deserialize_from_bytes(
                    serialized_contested_vote_poll_bytes_2.as_slice(),
                )
                .expect("expected to deserialize");

                assert_eq!(
                    vote_poll_2,
                    VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: Identifier(IdentifierBytes32([
                                230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123,
                                91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83,
                                197, 49, 85
                            ])),
                            document_type_name: "domain".to_string(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![
                                Text("dash".to_string()),
                                Text("c00110".to_string())
                            ]
                        }
                    )
                );
            }

            #[test]
            fn test_proved_end_date_query_multiple_contests() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();
                let mut platform_state = (**platform_state).clone();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 500000,
                            height: 100,
                            core_height: 42,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create two new contenders, but we are on the same contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "quantum",
                    platform_version,
                );

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    9,
                    "coolio",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: None,
                                    end_time_info: None,
                                    limit: None,
                                    offset: None,
                                    ascending: true,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::Proof(proof)) = result
                else {
                    panic!("expected contenders")
                };

                let vote_poll_by_end_date_query = VotePollsByEndDateDriveQuery {
                    start_time: None,
                    end_time: None,
                    offset: None,
                    limit: None,
                    order_ascending: true,
                };

                let (_, vote_polls_by_timestamps) = vote_poll_by_end_date_query
                    .verify_vote_polls_by_end_date_proof(
                        proof.grovedb_proof.as_ref(),
                        platform_version,
                    )
                    .expect("expected to verify proof");

                assert_eq!(
                    vote_polls_by_timestamps,
                    BTreeMap::from([
                        (
                            1_209_603_000,
                            vec![VotePoll::ContestedDocumentResourceVotePoll(
                                ContestedDocumentResourceVotePoll {
                                    contract_id: Identifier(IdentifierBytes32([
                                        230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109,
                                        222, 123, 91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33,
                                        246, 34, 191, 83, 197, 49, 85
                                    ])),
                                    document_type_name: "domain".to_string(),
                                    index_name: "parentNameAndLabel".to_string(),
                                    index_values: vec![
                                        Text("dash".to_string()),
                                        Text("quantum".to_string())
                                    ]
                                }
                            )]
                        ),
                        (
                            1_210_103_000,
                            vec![VotePoll::ContestedDocumentResourceVotePoll(
                                ContestedDocumentResourceVotePoll {
                                    contract_id: Identifier(IdentifierBytes32([
                                        230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109,
                                        222, 123, 91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33,
                                        246, 34, 191, 83, 197, 49, 85
                                    ])),
                                    document_type_name: "domain".to_string(),
                                    index_name: "parentNameAndLabel".to_string(),
                                    index_values: vec![
                                        Text("dash".to_string()),
                                        Text("c00110".to_string())
                                    ]
                                }
                            )]
                        )
                    ])
                );
            }

            #[test]
            fn test_not_proved_end_date_query_multiple_contests_with_start_at() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();
                let mut platform_state = (**platform_state).clone();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 500000,
                            height: 100,
                            core_height: 42,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create two new contenders, but we are on the same contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "quantum",
                    platform_version,
                );

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    9,
                    "coolio",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 900000,
                            height: 150,
                            core_height: 45,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    10,
                    "crazyman",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::StartAtTimeInfo {
                                            start_time_ms: 1_209_603_000,
                                            start_time_included: false,
                                        },
                                    ),
                                    end_time_info: None,
                                    limit: None,
                                    offset: None,
                                    ascending: true,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::VotePollsByTimestamps(
                    get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamps {
                        vote_polls_by_timestamps,
                        finished_results,
                    },
                )) = result
                else {
                    panic!("expected contenders")
                };

                assert!(finished_results);

                let serialized_contested_vote_poll_bytes_2 = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 6, 99, 48, 48,
                    49, 49, 48,
                ];

                let serialized_contested_vote_poll_bytes_3 = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 8, 99, 114,
                    97, 122, 121, 109, 97, 110,
                ];

                assert_eq!(
                    vote_polls_by_timestamps,
                    vec![
                        SerializedVotePollsByTimestamp {
                            timestamp: 1_210_103_000, // in ms, 500 s after Jan 1 1970 + 3 seconds (chosen block time in test)
                            serialized_vote_polls: vec![
                                serialized_contested_vote_poll_bytes_2.clone()
                            ]
                        },
                        SerializedVotePollsByTimestamp {
                            timestamp: 1_210_503_000, // in ms, 900 s after Jan 1 1970 + 3 seconds (chosen block time in test)
                            serialized_vote_polls: vec![
                                serialized_contested_vote_poll_bytes_3.clone()
                            ]
                        },
                    ]
                );
            }

            #[test]
            fn test_not_proved_end_date_query_multiple_contests_with_end_at() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();
                let mut platform_state = (**platform_state).clone();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 500000,
                            height: 100,
                            core_height: 42,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create two new contenders, but we are on the same contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "quantum",
                    platform_version,
                );

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    9,
                    "coolio",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 900000,
                            height: 150,
                            core_height: 45,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    10,
                    "crazyman",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::StartAtTimeInfo {
                                            start_time_ms: 1_209_603_000,
                                            start_time_included: false,
                                        },
                                    ),
                                    end_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::EndAtTimeInfo {
                                            end_time_ms: 1_210_500_000,
                                            end_time_included: true,
                                        },
                                    ),
                                    limit: None,
                                    offset: None,
                                    ascending: true,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::VotePollsByTimestamps(
                    get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamps {
                        vote_polls_by_timestamps,
                        finished_results,
                    },
                )) = result
                else {
                    panic!("expected contenders")
                };

                assert!(finished_results);

                let serialized_contested_vote_poll_bytes_2 = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 6, 99, 48, 48,
                    49, 49, 48,
                ];

                assert_eq!(
                    vote_polls_by_timestamps,
                    vec![SerializedVotePollsByTimestamp {
                        timestamp: 1_210_103_000, // in ms, 500 s after Jan 1 1970 + 3 seconds (chosen block time in test)
                        serialized_vote_polls: vec![serialized_contested_vote_poll_bytes_2.clone()]
                    },]
                );
            }

            #[test]
            fn test_not_proved_end_date_query_multiple_contests_with_end_at_before_start_at() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();
                let mut platform_state = (**platform_state).clone();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 500000,
                            height: 100,
                            core_height: 42,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create two new contenders, but we are on the same contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "quantum",
                    platform_version,
                );

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    9,
                    "coolio",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 900000,
                            height: 150,
                            core_height: 45,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    10,
                    "crazyman",
                    platform_version,
                );

                platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::StartAtTimeInfo {
                                            start_time_ms: 1_209_603_000,
                                            start_time_included: true,
                                        },
                                    ),
                                    end_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::EndAtTimeInfo {
                                            end_time_ms: 1_209_601_000,
                                            end_time_included: true,
                                        },
                                    ),
                                    limit: None,
                                    offset: None,
                                    ascending: true,
                                    prove: false,
                                },
                            )),
                        },
                        &platform_state,
                        platform_version,
                    )
                    .expect("expected to execute query")
                    .into_data()
                    .expect_err("expected query to be invalid");

                platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::StartAtTimeInfo {
                                            start_time_ms: 1_209_603_000,
                                            start_time_included: true,
                                        },
                                    ),
                                    end_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::EndAtTimeInfo {
                                            end_time_ms: 1_209_603_000,
                                            end_time_included: false,
                                        },
                                    ),
                                    limit: None,
                                    offset: None,
                                    ascending: true,
                                    prove: false,
                                },
                            )),
                        },
                        &platform_state,
                        platform_version,
                    )
                    .expect("expected to execute query")
                    .into_data()
                    .expect_err("expected query to be invalid");
            }

            #[test]
            fn test_not_proved_end_date_query_multiple_contests_with_start_at_ascending_false() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();
                let mut platform_state = (**platform_state).clone();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 500000,
                            height: 100,
                            core_height: 42,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create two new contenders, but we are on the same contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "quantum",
                    platform_version,
                );

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    9,
                    "coolio",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 900000,
                            height: 150,
                            core_height: 45,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    10,
                    "crazyman",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: Some(
                                        get_vote_polls_by_end_date_request_v0::StartAtTimeInfo {
                                            start_time_ms: 1_209_603_000,
                                            start_time_included: false,
                                        },
                                    ),
                                    end_time_info: None,
                                    limit: None,
                                    offset: None,
                                    ascending: false,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::VotePollsByTimestamps(
                    get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamps {
                        vote_polls_by_timestamps,
                        finished_results,
                    },
                )) = result
                else {
                    panic!("expected contenders")
                };

                assert!(finished_results);

                let serialized_contested_vote_poll_bytes_2 = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 6, 99, 48, 48,
                    49, 49, 48,
                ];

                let serialized_contested_vote_poll_bytes_3 = vec![
                    0, 230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126,
                    10, 29, 113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 6, 100,
                    111, 109, 97, 105, 110, 18, 112, 97, 114, 101, 110, 116, 78, 97, 109, 101, 65,
                    110, 100, 76, 97, 98, 101, 108, 2, 18, 4, 100, 97, 115, 104, 18, 8, 99, 114,
                    97, 122, 121, 109, 97, 110,
                ];

                assert_eq!(
                    vote_polls_by_timestamps,
                    vec![
                        SerializedVotePollsByTimestamp {
                            timestamp: 1_210_503_000, // in ms, 900 s after Jan 1 1970 + 3 seconds (chosen block time in test)
                            serialized_vote_polls: vec![
                                serialized_contested_vote_poll_bytes_3.clone()
                            ]
                        },
                        SerializedVotePollsByTimestamp {
                            timestamp: 1_210_103_000, // in ms, 500 s after Jan 1 1970 + 3 seconds (chosen block time in test)
                            serialized_vote_polls: vec![
                                serialized_contested_vote_poll_bytes_2.clone()
                            ]
                        },
                    ]
                );
            }

            #[test]
            fn test_proved_end_date_query_multiple_contests_with_start_at() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();
                let mut platform_state = (**platform_state).clone();

                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    7,
                    "quantum",
                    platform_version,
                );

                platform_state.set_last_committed_block_info(Some(
                    ExtendedBlockInfoV0 {
                        basic_info: BlockInfo {
                            time_ms: 500000,
                            height: 100,
                            core_height: 42,
                            epoch: Default::default(),
                        },
                        app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                        quorum_hash: [0u8; 32],
                        block_id_hash: [0u8; 32],
                        signature: [0u8; 96],
                        round: 0,
                    }
                    .into(),
                ));

                // we create two new contenders, but we are on the same contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    8,
                    "quantum",
                    platform_version,
                );

                // we create a new contest
                create_dpns_name_contest(
                    &mut platform,
                    &platform_state,
                    9,
                    "coolio",
                    platform_version,
                );

                let GetVotePollsByEndDateResponse { version } = platform
                    .query_vote_polls_by_end_date_query(
                        GetVotePollsByEndDateRequest {
                            version: Some(get_vote_polls_by_end_date_request::Version::V0(
                                GetVotePollsByEndDateRequestV0 {
                                    start_time_info: None,
                                    end_time_info: None,
                                    limit: None,
                                    offset: None,
                                    ascending: true,
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

                let get_vote_polls_by_end_date_response::Version::V0(
                    GetVotePollsByEndDateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = version.expect("expected a version");

                let Some(get_vote_polls_by_end_date_response_v0::Result::Proof(proof)) = result
                else {
                    panic!("expected contenders")
                };

                let vote_poll_by_end_date_query = VotePollsByEndDateDriveQuery {
                    start_time: None,
                    end_time: None,
                    offset: None,
                    limit: None,
                    order_ascending: true,
                };

                let (_, vote_polls_by_timestamps) = vote_poll_by_end_date_query
                    .verify_vote_polls_by_end_date_proof(
                        proof.grovedb_proof.as_ref(),
                        platform_version,
                    )
                    .expect("expected to verify proof");

                assert_eq!(
                    vote_polls_by_timestamps,
                    BTreeMap::from([
                        (
                            1_209_603_000,
                            vec![VotePoll::ContestedDocumentResourceVotePoll(
                                ContestedDocumentResourceVotePoll {
                                    contract_id: Identifier(IdentifierBytes32([
                                        230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109,
                                        222, 123, 91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33,
                                        246, 34, 191, 83, 197, 49, 85
                                    ])),
                                    document_type_name: "domain".to_string(),
                                    index_name: "parentNameAndLabel".to_string(),
                                    index_values: vec![
                                        Text("dash".to_string()),
                                        Text("quantum".to_string())
                                    ]
                                }
                            )]
                        ),
                        (
                            1_210_103_000,
                            vec![VotePoll::ContestedDocumentResourceVotePoll(
                                ContestedDocumentResourceVotePoll {
                                    contract_id: Identifier(IdentifierBytes32([
                                        230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109,
                                        222, 123, 91, 126, 10, 29, 113, 42, 9, 196, 13, 87, 33,
                                        246, 34, 191, 83, 197, 49, 85
                                    ])),
                                    document_type_name: "domain".to_string(),
                                    index_name: "parentNameAndLabel".to_string(),
                                    index_values: vec![
                                        Text("dash".to_string()),
                                        Text("c00110".to_string())
                                    ]
                                }
                            )]
                        )
                    ])
                );
            }
        }
    }
}
