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
        _tx: TransactionArg,
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
            0 => self.transform_into_action_v0(),
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
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::platform_value::{Bytes32, Value};
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;

    mod vote_tests {
        use std::collections::BTreeMap;
        use std::sync::Arc;
        use arc_swap::Guard;
        use rand::Rng;
        use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse};
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
        use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
        use super::*;
        use dpp::document::Document;
        use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        use dpp::fee::Credits;
        use dpp::identifier::Identifier;
        use dpp::identity::{IdentityPublicKey, IdentityV0};
        use dpp::prelude::{DataContract, Identity};
        use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
        use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
        use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
        use dpp::util::hash::hash_double;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
        use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use dpp::voting::vote_polls::VotePoll;
        use dpp::voting::votes::resource_vote::ResourceVote;
        use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dpp::voting::votes::Vote;
        use drive::drive::object_size_info::DataContractResolvedInfo;
        use drive::drive::votes::resolve_contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
        use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally;
        use drive::query::vote_poll_vote_state_query::ResolvedContestedDocumentVotePollDriveQuery;
        use simple_signer::signer::SimpleSigner;
        use crate::platform_types::platform_state::PlatformState;
        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::TempPlatform;

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
            platform_state: &Guard<Arc<PlatformState>>,
            seed: u64,
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
            document_1.set("label", "quantum".into());
            document_1.set("normalizedLabel", "quantum".into());
            document_1.set("records.dashUniqueIdentityId", document_1.owner_id().into());
            document_1.set("subdomainRules.allowSubdomains", false.into());

            document_2.set("parentDomainName", "dash".into());
            document_2.set("normalizedParentDomainName", "dash".into());
            document_2.set("label", "quantum".into());
            document_2.set("normalizedLabel", "quantum".into());
            document_2.set("records.dashUniqueIdentityId", document_2.owner_id().into());
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
                    platform_state,
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
                    vote_poll: ContestedDocumentResourceVotePollWithContractInfo {
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

            let (root_hash, contenders) = resolved_contested_document_vote_poll_drive_query
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

            (identity_1, identity_2, dpns_contract)
        }

        #[test]
        fn test_masternode_voting() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (contender_1, contender_2, dpns_contract) =
                create_dpns_name_contest(&mut platform, &platform_state, 7, platform_version);

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

            assert_eq!(processing_result.valid_count(), 2);
        }
    }
}
