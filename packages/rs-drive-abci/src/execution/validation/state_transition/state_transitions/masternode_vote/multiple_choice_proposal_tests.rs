//! Multiple choice proposals (protocol version 15): a `multipleChoiceProposal` of the decision
//! proposals contract is a question, and each `choice` entered for it is an answer. The choices
//! of one proposal contend on the contested `byProposal` index, a vote among documents: each
//! choice is a contender of its own, named by its document id, so an identity enters as many
//! as it wants. The choice with the most votes is kept, and a lock means none of them. Anyone
//! enters choices for an `open` proposal, and only its owner for one that is not. The owner
//! may edit the question's text for two hours after creating it.
//!
//! A proposal holds no description of its own. It points at a document saying what is being
//! decided, of the contract and the document type it names: a moderated type of a contract
//! whose moderators the masternodes elect, holding text a moderator's removal takes away.

use crate::execution::validation::state_transition::state_transitions::tests::{
    setup_identity, setup_masternode_voting_identity,
};
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{
    get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0,
};
use dapi_grpc::platform::v0::{
    get_contested_resource_vote_state_request, get_contested_resource_vote_state_response,
    GetContestedResourceVoteStateRequest,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
    ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
};
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::DocumentReferenceKind;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::DataContract;
use dpp::decision_proposal::DECISION_PROPOSALS_CONTRACT_ID;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, TimestampMillis};
use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;
use dpp::platform_value::{platform_value, Bytes32, Identifier, Value};
use dpp::prelude::IdentityNonce;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::system_data_contracts::decision_proposals_contract::v1::document_types::{
    choice, multiple_choice_proposal,
};
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::document::ContestWindows;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryExecutionResult, ContestedDocumentVotePollDriveQueryResultType,
    ResolvedContestedDocumentVotePollDriveQuery,
};
use drive::query::DriveDocumentQuery;
use drive::util::object_size_info::DataContractResolvedInfo;
use drive::util::storage_flags::StorageFlags;
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type IdentityInfo = (Identity, SimpleSigner, IdentityPublicKey);

/// An identity writing to the decision proposals contract, with the identity contract nonce its
/// next transition uses.
struct Writer {
    info: IdentityInfo,
    next_nonce: IdentityNonce,
}

impl Writer {
    fn id(&self) -> Identifier {
        self.info.0.id()
    }
}

fn block(time_ms: TimestampMillis, height: u64) -> BlockInfo {
    BlockInfo {
        time_ms,
        height,
        core_height: 42,
        epoch: Default::default(),
    }
}

/// A document type of the description hosts whose documents moderators delete, a removal
/// taking their `text` with it.
const TEXT: &str = "text";
/// One whose removal record keeps the `text`.
const KEPT_TEXT: &str = "keptText";
/// One whose documents their owner deletes.
const PLAIN_TEXT: &str = "plainText";

/// A document type holding a `text`, with `extra` merged into its schema.
fn text_schema(extra: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "minLength": 1, "maxLength": 4096, "position": 0 },
        },
        "required": ["text"],
        "additionalProperties": false,
    });
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        schema_map.extend(extra_map);
    }
    schema
}

/// A contract hosting the texts proposals describe themselves with. Its moderators are a
/// team the masternodes elect, its owner until one is seated, when `elected`; its owner
/// alone otherwise. The same contract every time it is built.
fn description_host(elected: bool, platform_version: &PlatformVersion) -> DataContract {
    let owner_id = Identifier::new([if elected { 0xD5 } else { 0xD6 }; 32]);
    let mut contract =
        get_data_contract_fixture(Some(owner_id), 1, platform_version.protocol_version)
            .data_contract_owned();
    let moderators = if elected {
        ContractModerators::Elected(Box::new(ElectedModerators {
            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            challenge_cool_down: Some(1_209_600),
            moderated_document_types: [TEXT, KEPT_TEXT]
                .into_iter()
                .map(|name| {
                    (
                        name.to_string(),
                        BTreeSet::from([ModerationAbility::DeleteDocuments]),
                    )
                })
                .collect(),
            interim: InterimModerators::ContractOwner,
            election_delay: None,
            max_added_moderators: 0,
            owner_protected: false,
        }))
    } else {
        ContractModerators::ContractOwner
    };
    contract.set_config(contract.config().clone().with_moderation(Some(
        ContractModerationConfig {
            banlist: false,
            suspensions: false,
            warnings: false,
            moderators,
        },
    )));
    // After the config: a type moderators delete from needs the contract to be moderated
    for (name, schema) in [
        (
            TEXT,
            text_schema(platform_value!({
                "canBeDeleted": false,
                "moderatorAbilities": { "delete": true },
            })),
        ),
        (
            KEPT_TEXT,
            text_schema(platform_value!({
                "canBeDeleted": false,
                "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["text"] },
            })),
        ),
        (PLAIN_TEXT, text_schema(platform_value!({}))),
    ] {
        contract
            .set_document_schema(name, schema, true, &mut vec![], platform_version)
            .expect("expected to add the document type");
    }
    contract
}

fn setup() -> (
    TempPlatform<MockCoreRPCLike>,
    &'static PlatformVersion,
    Arc<DataContract>,
    StdRng,
) {
    let platform_version = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    for elected in [true, false] {
        platform
            .drive
            .apply_contract(
                &description_host(elected, platform_version),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the description host");
    }
    let contract = platform
        .drive
        .cache
        .system_data_contracts
        .load_decision_proposals(platform_version)
        .expect("expected the decision proposals contract at genesis");
    (
        platform,
        platform_version,
        contract,
        StdRng::seed_from_u64(0x3C40_1CE5),
    )
}

fn writer(platform: &mut TempPlatform<MockCoreRPCLike>, rng: &mut StdRng) -> Writer {
    Writer {
        info: setup_identity(platform, rng.gen(), dash_to_credits!(1.0)),
        next_nonce: 1,
    }
}

/// The vote among the choices entered for `proposal_id`.
fn choice_poll(proposal_id: Identifier) -> ContestedDocumentResourceVotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: DECISION_PROPOSALS_CONTRACT_ID,
        document_type_name: choice::NAME.to_string(),
        index_name: choice::indexes::BY_PROPOSAL.to_string(),
        index_values: vec![Value::Identifier(proposal_id.to_buffer())],
    }
}

/// A serialized create of a `document_type_name` document of the decision proposals contract
/// holding `properties`, signed by `writer`, and the id of the document it creates. A choice
/// states the contest fund, which the builder reads from its contested index.
async fn create_transition(
    contract: &DataContract,
    writer: &mut Writer,
    document_type_name: &str,
    properties: BTreeMap<String, Value>,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (Vec<u8>, Identifier) {
    let document_type = contract
        .document_type_for_name(document_type_name)
        .expect("expected the document type");
    let entropy = Bytes32::random_with_rng(rng);
    let nonce = writer.next_nonce;
    writer.next_nonce += 1;
    let mut document: Document = DocumentV0 {
        owner_id: writer.id(),
        properties,
        ..Default::default()
    }
    .into();
    document
        .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
        .expect("expected to set the document id");
    let id = document.id();
    let (_, signer, key) = &writer.info;
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        document_type,
        entropy.0,
        key,
        nonce,
        0,
        None,
        signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to create the batch transition");
    (
        transition
            .serialize_to_bytes()
            .expect("expected to serialize the batch transition"),
        id,
    )
}

/// Processes one transition in a block at `time_ms` and returns its execution result.
fn process(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> StateTransitionExecutionResult {
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[transition],
            &platform_state,
            &block(time_ms, 1),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the state transition");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");
    processing_result.into_execution_results().remove(0)
}

fn process_valid(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) {
    match process(platform, transition, time_ms, platform_version) {
        StateTransitionExecutionResult::SuccessfulExecution { .. } => {}
        other => panic!("expected the transition to pass, got {other:?}"),
    }
}

/// Processes one transition that must be refused, paid, and returns why.
fn process_refused(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> ConsensusError {
    match process(platform, transition, time_ms, platform_version) {
        StateTransitionExecutionResult::PaidConsensusError { error, .. } => error,
        other => panic!("expected a paid refusal, got {other:?}"),
    }
}

/// `proposer` asks `question` at `time_ms`, open to anyone's choices; returns the proposal's id.
async fn ask(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    proposer: &mut Writer,
    question: &str,
    time_ms: TimestampMillis,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Identifier {
    ask_stating(
        platform,
        contract,
        proposer,
        question,
        true,
        time_ms,
        rng,
        platform_version,
    )
    .await
}

/// `proposer` asks `question` at `time_ms`, stating whether it is `open`; returns the
/// proposal's id.
#[allow(clippy::too_many_arguments)]
async fn ask_stating(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    proposer: &mut Writer,
    question: &str,
    open: bool,
    time_ms: TimestampMillis,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Identifier {
    // The proposer first writes what is being decided where moderators can take it down
    let host = description_host(true, platform_version);
    let description_id = describe(
        platform,
        &host,
        proposer,
        TEXT,
        time_ms,
        rng,
        platform_version,
    )
    .await;
    let (transition, id) = create_transition(
        contract,
        proposer,
        multiple_choice_proposal::NAME,
        question_properties(question, open, host.id(), TEXT, description_id),
        rng,
        platform_version,
    )
    .await;
    process_valid(platform, transition, time_ms, platform_version);
    id
}

/// `writer` writes a text of `document_type_name` in `host` at `time_ms`; returns its id.
async fn describe(
    platform: &TempPlatform<MockCoreRPCLike>,
    host: &DataContract,
    writer: &mut Writer,
    document_type_name: &str,
    time_ms: TimestampMillis,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Identifier {
    let (transition, id) = create_transition(
        host,
        writer,
        document_type_name,
        BTreeMap::from([(
            "text".to_string(),
            Value::Text("Enter the answer you would have the network give.".to_string()),
        )]),
        rng,
        platform_version,
    )
    .await;
    process_valid(platform, transition, time_ms, platform_version);
    id
}

/// The properties of a multiple choice proposal asking `question`, described by the document
/// `description_id` of `description_document_type` in the contract `description_contract_id`.
fn question_properties(
    question: &str,
    open: bool,
    description_contract_id: Identifier,
    description_document_type: &str,
    description_id: Identifier,
) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            multiple_choice_proposal::properties::TITLE.to_string(),
            Value::Text(question.to_string()),
        ),
        (
            multiple_choice_proposal::properties::DESCRIPTION_ID.to_string(),
            Value::Identifier(description_id.to_buffer()),
        ),
        (
            multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID.to_string(),
            Value::Identifier(description_contract_id.to_buffer()),
        ),
        (
            multiple_choice_proposal::properties::DESCRIPTION_DOCUMENT_TYPE.to_string(),
            Value::Text(description_document_type.to_string()),
        ),
        (
            multiple_choice_proposal::properties::OPEN.to_string(),
            Value::Bool(open),
        ),
    ])
}

/// A serialized create of `writer`'s choice `answer` for `proposal_id`, and the choice's id.
async fn choice_transition(
    contract: &DataContract,
    writer: &mut Writer,
    proposal_id: Identifier,
    answer: &str,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (Vec<u8>, Identifier) {
    create_transition(
        contract,
        writer,
        choice::NAME,
        BTreeMap::from([
            (
                choice::properties::PROPOSAL_ID.to_string(),
                Value::Identifier(proposal_id.to_buffer()),
            ),
            (
                choice::properties::TITLE.to_string(),
                Value::Text(answer.to_string()),
            ),
        ]),
        rng,
        platform_version,
    )
    .await
}

/// `writer` enters `answer` for `proposal_id` at `time_ms`; returns the choice's id.
#[allow(clippy::too_many_arguments)]
async fn enter(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    writer: &mut Writer,
    proposal_id: Identifier,
    answer: &str,
    time_ms: TimestampMillis,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Identifier {
    let (transition, id) =
        choice_transition(contract, writer, proposal_id, answer, rng, platform_version).await;
    process_valid(platform, transition, time_ms, platform_version);
    id
}

/// A masternode votes `vote_choice` in `poll` at `time_ms`, and the vote passes.
async fn vote(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    poll: &ContestedDocumentResourceVotePoll,
    vote_choice: ResourceVoteChoice,
    masternode_seed: u64,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) {
    match cast_vote(
        platform,
        poll,
        vote_choice,
        masternode_seed,
        time_ms,
        platform_version,
    )
    .await
    {
        StateTransitionExecutionResult::SuccessfulExecution { .. } => {}
        other => panic!("expected the vote to pass, got {other:?}"),
    }
}

/// A masternode votes `vote_choice` in `poll` at `time_ms`; returns what came of the vote.
async fn cast_vote(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    poll: &ContestedDocumentResourceVotePoll,
    vote_choice: ResourceVoteChoice,
    masternode_seed: u64,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> StateTransitionExecutionResult {
    let (pro_tx_hash, _, signer, voting_key) =
        setup_masternode_voting_identity(platform, masternode_seed, platform_version);
    let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::ContestedDocumentResourceVotePoll(poll.clone()),
        resource_vote_choice: vote_choice,
    }));
    let transition = MasternodeVoteTransition::try_from_vote_with_signer(
        vote,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
        None,
    )
    .await
    .expect("expected to make the vote")
    .serialize_to_bytes()
    .expect("expected to serialize the vote");
    process(platform, transition, time_ms, platform_version)
}

/// Ends every poll due at `time_ms`, as the block at that time would.
fn end_polls_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    time_ms: TimestampMillis,
    height: u64,
    platform_version: &PlatformVersion,
) {
    let mut platform_state = (**platform.state.load()).clone();
    let block_info = block(time_ms, height);
    platform_state.set_last_committed_block_info(Some(
        ExtendedBlockInfoV0 {
            basic_info: block_info,
            app_hash: platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap(),
            quorum_hash: [0u8; 32],
            block_id_hash: [0u8; 32],
            proposer_pro_tx_hash: [0u8; 32],
            signature: [0u8; 96],
            round: 0,
        }
        .into(),
    ));
    platform.state.store(Arc::new(platform_state));
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform
        .check_for_ended_vote_polls(
            &platform_state,
            &platform_state,
            &block_info,
            Some(&transaction),
            platform_version,
        )
        .expect("ending the polls due must never fail");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");
}

/// Where the vote among the choices of a proposal stands; `None` before its first choice.
fn status(
    platform: &TempPlatform<MockCoreRPCLike>,
    poll: &ContestedDocumentResourceVotePoll,
    platform_version: &PlatformVersion,
) -> Option<ContestedDocumentVotePollStatus> {
    let resolved = poll
        .resolve(&platform.drive, None, platform_version)
        .expect("expected to resolve the contest");
    let (_, stored_info) = platform
        .drive
        .fetch_contested_document_vote_poll_stored_info(&resolved, None, None, platform_version)
        .expect("expected to read the contest");
    stored_info.map(|info| info.vote_poll_status())
}

fn prefunded_balance(
    platform: &TempPlatform<MockCoreRPCLike>,
    poll: &ContestedDocumentResourceVotePoll,
    platform_version: &PlatformVersion,
) -> Credits {
    platform
        .drive
        .fetch_prefunded_specialized_balance(
            poll.specialized_balance_id()
                .expect("expected the balance id")
                .to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to read the balance")
        .unwrap_or_default()
}

/// The `document_type_name` document stored under `document_id`.
fn stored_document(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    document_type_name: &str,
    document_id: Identifier,
) -> Option<Document> {
    let document_type = contract
        .document_type_for_name(document_type_name)
        .expect("expected the document type");
    platform
        .drive
        .query_documents(
            DriveDocumentQuery::new_primary_key_single_item_query(
                contract,
                document_type,
                document_id,
            ),
            None,
            false,
            None,
            None,
        )
        .expect("expected to query the document")
        .documents_owned()
        .pop()
}

/// The choice stored under `choice_id`, which only the choice a vote kept is.
fn stored_choice(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    choice_id: Identifier,
) -> Option<Document> {
    stored_document(platform, contract, choice::NAME, choice_id)
}

/// A serialized edit of the multiple choice proposal `question` by `proposer` (its owner)
/// setting `changes`, its revision bumped.
async fn edit_transition(
    contract: &DataContract,
    proposer: &mut Writer,
    mut question: Document,
    changes: &[(&str, Value)],
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    for (property, value) in changes {
        question.set(property, value.clone());
    }
    question.increment_revision().expect("revision increments");
    let nonce = proposer.next_nonce;
    proposer.next_nonce += 1;
    let (_, signer, key) = &proposer.info;
    BatchTransition::new_document_replacement_transition_from_document(
        question,
        contract
            .document_type_for_name(multiple_choice_proposal::NAME)
            .expect("expected the multiple choice proposal document type"),
        key,
        nonce,
        0,
        None,
        signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to build the edit")
    .serialize_to_bytes()
    .expect("expected to serialize the edit")
}

/// The vote among the choices of `proposal_id` as a client reads it: proved by the vote state
/// query and verified, the verified root being Drive's.
fn proved_vote_state(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    proposal_id: Identifier,
    platform_version: &PlatformVersion,
) -> ContestedDocumentVotePollDriveQueryExecutionResult {
    const LIMIT: u16 = 10;
    let index_value = Value::Identifier(proposal_id.to_buffer());
    let platform_state = platform.state.load();
    let response = platform
        .query_contested_resource_vote_state(
            GetContestedResourceVoteStateRequest {
                version: Some(get_contested_resource_vote_state_request::Version::V0(
                    GetContestedResourceVoteStateRequestV0 {
                        contract_id: contract.id().to_vec(),
                        document_type_name: choice::NAME.to_string(),
                        index_name: choice::indexes::BY_PROPOSAL.to_string(),
                        index_values: vec![bincode::encode_to_vec(
                            &index_value,
                            PLATFORM_BINCODE_CONFIG,
                        )
                        .expect("expected to encode the proposal id")],
                        result_type: ResultType::DocumentsAndVoteTally as i32,
                        allow_include_locked_and_abstaining_vote_tally: true,
                        start_at_identifier_info: None,
                        count: Some(LIMIT as u32),
                        prove: true,
                    },
                )),
            },
            &platform_state,
            platform_version,
        )
        .expect("expected to execute the query")
        .into_data()
        .expect("expected the query to be valid");
    let get_contested_resource_vote_state_response::Version::V0(
        GetContestedResourceVoteStateResponseV0 { result, .. },
    ) = response.version.expect("expected a version");
    let Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)) = result else {
        panic!("expected a proof")
    };
    let (root_hash, result) = ResolvedContestedDocumentVotePollDriveQuery {
        vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: DataContractResolvedInfo::BorrowedDataContract(contract),
            document_type_name: choice::NAME.to_string(),
            index_name: choice::indexes::BY_PROPOSAL.to_string(),
            index_values: vec![index_value],
        },
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
        offset: None,
        limit: Some(LIMIT),
        start_at: None,
        allow_include_locked_and_abstaining_vote_tally: true,
    }
    .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
    .expect("expected to verify the proof");
    assert_eq!(
        root_hash,
        platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected Drive's root"),
        "the proof verifies against Drive's root"
    );
    result
}

fn windows(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> ContestWindows {
    ContestWindows::generic(platform.config.network, platform_version)
}

#[tokio::test]
async fn should_keep_the_choice_the_masternodes_vote_for_most() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let poll_duration = windows(&platform, platform_version).poll_duration_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let mut bob = writer(&mut platform, &mut rng);
    let mut carol = writer(&mut platform, &mut rng);

    let proposal_id = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which block time should Platform target?",
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let poll = choice_poll(proposal_id);
    // A question alone opens no vote
    assert_eq!(status(&platform, &poll, platform_version), None);

    // Anyone enters a choice, the proposer included: the first opens the vote, and each
    // prefunds the masternode votes with the contest fund
    let start = 20_000;
    let alice_choice = enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "One second",
        start,
        &mut rng,
        platform_version,
    )
    .await;
    assert!(matches!(
        status(&platform, &poll, platform_version),
        Some(ContestedDocumentVotePollStatus::Started(_))
    ));
    let bob_choice = enter(
        &platform,
        &contract,
        &mut bob,
        proposal_id,
        "Two seconds",
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;
    let carol_choice = enter(
        &platform,
        &contract,
        &mut carol,
        proposal_id,
        "Three seconds",
        start + 120_000,
        &mut rng,
        platform_version,
    )
    .await;
    let proposer_choice = enter(
        &platform,
        &contract,
        &mut proposer,
        proposal_id,
        "Leave it as it is",
        start + 180_000,
        &mut rng,
        platform_version,
    )
    .await;
    let fund = platform_version
        .fee_version
        .vote_resolution_fund_fees
        .contested_document_vote_resolution_fund_required_amount;
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        4 * fund
    );
    // A choice is held by the vote, not stored, until the vote ends
    assert!(stored_choice(&platform, &contract, bob_choice).is_none());

    // A vote names a choice by its document id. Two masternodes for Bob's choice, one each for
    // Alice's and Carol's, one for none of them
    for (seed, vote_choice) in [
        (1, ResourceVoteChoice::TowardsIdentity(bob_choice)),
        (2, ResourceVoteChoice::TowardsIdentity(alice_choice)),
        (3, ResourceVoteChoice::TowardsIdentity(bob_choice)),
        (4, ResourceVoteChoice::TowardsIdentity(carol_choice)),
        (5, ResourceVoteChoice::Lock),
    ] {
        vote(
            &mut platform,
            &poll,
            vote_choice,
            seed,
            start + 200_000,
            platform_version,
        )
        .await;
    }

    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        Some(ContestedDocumentVotePollStatus::Awarded(bob_choice))
    );
    let kept = stored_choice(&platform, &contract, bob_choice).expect("Bob's choice is kept");
    assert_eq!(kept.owner_id(), bob.id());
    assert_eq!(
        kept.properties().get(choice::properties::TITLE),
        Some(&Value::Text("Two seconds".to_string()))
    );
    for other in [alice_choice, carol_choice, proposer_choice] {
        assert!(stored_choice(&platform, &contract, other).is_none());
    }
}

#[tokio::test]
async fn should_keep_no_choice_when_the_lock_votes_outnumber_every_choice() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let poll_duration = windows(&platform, platform_version).poll_duration_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let mut bob = writer(&mut platform, &mut rng);
    let proposal_id = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let poll = choice_poll(proposal_id);
    let start = 20_000;
    let alice_choice = enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        start,
        &mut rng,
        platform_version,
    )
    .await;
    let bob_choice = enter(
        &platform,
        &contract,
        &mut bob,
        proposal_id,
        "A fox",
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;

    for (seed, vote_choice) in [
        (1, ResourceVoteChoice::Lock),
        (2, ResourceVoteChoice::TowardsIdentity(alice_choice)),
        (3, ResourceVoteChoice::Lock),
    ] {
        vote(
            &mut platform,
            &poll,
            vote_choice,
            seed,
            start + 120_000,
            platform_version,
        )
        .await;
    }

    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        Some(ContestedDocumentVotePollStatus::Locked)
    );
    for entered in [alice_choice, bob_choice] {
        assert!(stored_choice(&platform, &contract, entered).is_none());
    }

    // The masternodes' answer stands: no choice can be entered for the proposal again
    let (again, _) = choice_transition(
        &contract,
        &mut proposer,
        proposal_id,
        "A heron",
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(
        &platform,
        again,
        start + poll_duration + 60_000,
        platform_version,
    );
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::DocumentContestCurrentlyLockedError(_))
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_keep_the_first_choice_entered_when_nobody_votes() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let poll_duration = windows(&platform, platform_version).poll_duration_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let mut bob = writer(&mut platform, &mut rng);
    let proposal_id = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let poll = choice_poll(proposal_id);
    let start = 20_000;
    let bob_choice = enter(
        &platform,
        &contract,
        &mut bob,
        proposal_id,
        "A fox",
        start,
        &mut rng,
        platform_version,
    )
    .await;
    let alice_choice = enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;

    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        Some(ContestedDocumentVotePollStatus::Awarded(bob_choice))
    );
    assert!(stored_choice(&platform, &contract, bob_choice).is_some());
    assert!(stored_choice(&platform, &contract, alice_choice).is_none());
}

#[tokio::test]
async fn should_let_an_identity_enter_several_choices_and_vote_on_each() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let poll_duration = windows(&platform, platform_version).poll_duration_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let mut bob = writer(&mut platform, &mut rng);
    let proposal_id = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let poll = choice_poll(proposal_id);
    let start = 20_000;
    let owl = enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        start,
        &mut rng,
        platform_version,
    )
    .await;
    let fox = enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "A fox",
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;
    let heron = enter(
        &platform,
        &contract,
        &mut bob,
        proposal_id,
        "A heron",
        start + 120_000,
        &mut rng,
        platform_version,
    )
    .await;
    // Each choice prefunds the votes, whoever entered it
    let fund = platform_version
        .fee_version
        .vote_resolution_fund_fees
        .contested_document_vote_resolution_fund_required_amount;
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        3 * fund
    );

    // A vote names a choice, not who entered it: Alice entered two, and her id names neither
    let refused = cast_vote(
        &mut platform,
        &poll,
        ResourceVoteChoice::TowardsIdentity(alice.id()),
        1,
        start + 180_000,
        platform_version,
    )
    .await;
    assert!(
        matches!(
            refused,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::VoteChoiceNotAllowedForVotePollError(_)
            ))
        ),
        "{refused:?}"
    );

    // Her two choices are voted on apart
    for (seed, vote_choice) in [
        (2, ResourceVoteChoice::TowardsIdentity(fox)),
        (3, ResourceVoteChoice::TowardsIdentity(owl)),
        (4, ResourceVoteChoice::TowardsIdentity(fox)),
        (5, ResourceVoteChoice::TowardsIdentity(heron)),
    ] {
        vote(
            &mut platform,
            &poll,
            vote_choice,
            seed,
            start + 200_000,
            platform_version,
        )
        .await;
    }

    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        Some(ContestedDocumentVotePollStatus::Awarded(fox))
    );
    // The choice kept is stored under the identity that entered it
    let kept = stored_choice(&platform, &contract, fox).expect("the fox is kept");
    assert_eq!(kept.owner_id(), alice.id());
    assert_eq!(
        kept.properties().get(choice::properties::TITLE),
        Some(&Value::Text("A fox".to_string()))
    );
    for other in [owl, heron] {
        assert!(stored_choice(&platform, &contract, other).is_none());
    }
}

#[tokio::test]
async fn should_prove_the_choices_of_one_identity_as_contenders_of_their_own() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let poll_duration = windows(&platform, platform_version).poll_duration_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let mut bob = writer(&mut platform, &mut rng);
    let proposal_id = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let poll = choice_poll(proposal_id);
    let start = 20_000;
    let owl = enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        start,
        &mut rng,
        platform_version,
    )
    .await;
    let fox = enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "A fox",
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;
    let heron = enter(
        &platform,
        &contract,
        &mut bob,
        proposal_id,
        "A heron",
        start + 120_000,
        &mut rng,
        platform_version,
    )
    .await;
    for (seed, vote_choice) in [
        (1, ResourceVoteChoice::TowardsIdentity(fox)),
        (2, ResourceVoteChoice::TowardsIdentity(owl)),
        (3, ResourceVoteChoice::TowardsIdentity(fox)),
        (4, ResourceVoteChoice::Lock),
    ] {
        vote(
            &mut platform,
            &poll,
            vote_choice,
            seed,
            start + 200_000,
            platform_version,
        )
        .await;
    }

    // While the vote runs, the proof holds a contender under each choice's document id, each
    // with its own tally, and the two Alice entered are both hers
    let running = proved_vote_state(&platform, &contract, proposal_id, platform_version);
    assert_eq!(running.winner, None);
    assert_eq!(running.locked_vote_tally, Some(1));
    let document_type = contract
        .document_type_for_name(choice::NAME)
        .expect("expected the choice document type");
    let proved: BTreeMap<Identifier, (Option<u32>, Identifier, Identifier)> = running
        .contenders
        .iter()
        .map(|contender| {
            let document = Document::from_bytes(
                contender
                    .serialized_document()
                    .as_ref()
                    .expect("the proof holds the contender's document"),
                document_type,
                platform_version,
            )
            .expect("expected to deserialize the choice");
            (
                contender.identity_id(),
                (contender.vote_tally(), document.id(), document.owner_id()),
            )
        })
        .collect();
    assert_eq!(
        proved,
        BTreeMap::from([
            (owl, (Some(1), owl, alice.id())),
            (fox, (Some(2), fox, alice.id())),
            (heron, (Some(0), heron, bob.id())),
        ])
    );

    // Once it has ended, the proof names the winning choice by its document id
    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    let ended = proved_vote_state(&platform, &contract, proposal_id, platform_version);
    assert!(
        matches!(
            ended.winner,
            Some((ContestedDocumentVotePollWinnerInfo::WonByIdentity(winner), _)) if winner == fox
        ),
        "{:?}",
        ended.winner
    );
    let kept = stored_choice(&platform, &contract, fox).expect("the fox is kept");
    assert_eq!(kept.owner_id(), alice.id());
}

#[tokio::test]
async fn should_refuse_a_choice_for_a_proposal_that_does_not_exist() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let mut alice = writer(&mut platform, &mut rng);
    // The id of a yes/no proposal names no multiple choice proposal either
    let (transition, _) = choice_transition(
        &contract,
        &mut alice,
        Identifier::new(rng.gen()),
        "An owl",
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, transition, 20_000, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_))
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_refuse_a_choice_entered_after_the_join_window() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let join_window = windows(&platform, platform_version).join_window_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let mut bob = writer(&mut platform, &mut rng);
    let proposal_id = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let start = 20_000;
    enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        start,
        &mut rng,
        platform_version,
    )
    .await;

    let (late, _) = choice_transition(
        &contract,
        &mut bob,
        proposal_id,
        "A fox",
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, late, start + join_window + 1, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::DocumentContestNotJoinableError(_))
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_run_one_vote_per_proposal() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let poll_duration = windows(&platform, platform_version).poll_duration_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let mut bob = writer(&mut platform, &mut rng);
    let mascot = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let color = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which color?",
        11_000,
        &mut rng,
        platform_version,
    )
    .await;

    // The same identities answer both questions, and each question has its own vote
    let start = 20_000;
    let mut entered = BTreeMap::new();
    for (index, (writer, proposal_id, answer)) in
        [(&mut alice, mascot, "An owl"), (&mut bob, mascot, "A fox")]
            .into_iter()
            .enumerate()
    {
        let id = enter(
            &platform,
            &contract,
            writer,
            proposal_id,
            answer,
            start + index as u64 * 1_000,
            &mut rng,
            platform_version,
        )
        .await;
        entered.insert(answer, id);
    }
    for (index, (writer, proposal_id, answer)) in
        [(&mut alice, color, "Blue"), (&mut bob, color, "Green")]
            .into_iter()
            .enumerate()
    {
        let id = enter(
            &platform,
            &contract,
            writer,
            proposal_id,
            answer,
            start + 10_000 + index as u64 * 1_000,
            &mut rng,
            platform_version,
        )
        .await;
        entered.insert(answer, id);
    }

    vote(
        &mut platform,
        &choice_poll(mascot),
        ResourceVoteChoice::TowardsIdentity(entered["An owl"]),
        1,
        start + 60_000,
        platform_version,
    )
    .await;
    vote(
        &mut platform,
        &choice_poll(color),
        ResourceVoteChoice::TowardsIdentity(entered["Green"]),
        2,
        start + 61_000,
        platform_version,
    )
    .await;

    // Both votes end within the poll duration of the later one; a block ends a bounded number
    // of polls, so two blocks see them both out
    end_polls_at(
        &platform,
        start + 20_000 + poll_duration,
        11,
        platform_version,
    );
    end_polls_at(
        &platform,
        start + 21_000 + poll_duration,
        12,
        platform_version,
    );
    assert_eq!(
        status(&platform, &choice_poll(mascot), platform_version),
        Some(ContestedDocumentVotePollStatus::Awarded(entered["An owl"]))
    );
    assert_eq!(
        status(&platform, &choice_poll(color), platform_version),
        Some(ContestedDocumentVotePollStatus::Awarded(entered["Green"]))
    );
    assert!(stored_choice(&platform, &contract, entered["An owl"]).is_some());
    assert!(stored_choice(&platform, &contract, entered["Green"]).is_some());
    assert!(stored_choice(&platform, &contract, entered["A fox"]).is_none());
    assert!(stored_choice(&platform, &contract, entered["Blue"]).is_none());
}

#[tokio::test]
async fn should_let_the_proposer_edit_the_question_for_two_hours() {
    const TWO_HOURS_MS: TimestampMillis = 7_200_000;
    let (mut platform, platform_version, contract, mut rng) = setup();
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let created_at = 10_000;
    let proposal_id = ask(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        created_at,
        &mut rng,
        platform_version,
    )
    .await;
    let question = |platform: &TempPlatform<MockCoreRPCLike>| {
        stored_document(
            platform,
            &contract,
            multiple_choice_proposal::NAME,
            proposal_id,
        )
        .expect("the question is stored")
    };

    // A choice entered meanwhile does not close the window
    enter(
        &platform,
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        created_at + 60_000,
        &mut rng,
        platform_version,
    )
    .await;

    // Another text the proposer wrote, to describe the question with instead
    let host = description_host(true, platform_version);
    let rewritten = describe(
        &platform,
        &host,
        &mut proposer,
        TEXT,
        created_at + 30_000,
        &mut rng,
        platform_version,
    )
    .await;
    let later_text = describe(
        &platform,
        &host,
        &mut proposer,
        TEXT,
        created_at + 30_000,
        &mut rng,
        platform_version,
    )
    .await;

    // The title, the description and the url may change until two hours after the creation
    let edit = edit_transition(
        &contract,
        &mut proposer,
        question(&platform),
        &[
            (
                multiple_choice_proposal::properties::TITLE,
                Value::Text("Which mascot for Platform?".to_string()),
            ),
            (
                multiple_choice_proposal::properties::DESCRIPTION_ID,
                Value::Identifier(rewritten.to_buffer()),
            ),
            (
                multiple_choice_proposal::properties::URL,
                Value::Text("https://example.org/mascot".to_string()),
            ),
        ],
        platform_version,
    )
    .await;
    process_valid(&platform, edit, created_at + 3_600_000, platform_version);
    let edited = question(&platform);
    assert_eq!(
        edited.get(multiple_choice_proposal::properties::TITLE),
        Some(&Value::Text("Which mascot for Platform?".to_string()))
    );
    assert_eq!(
        edited.get(multiple_choice_proposal::properties::URL),
        Some(&Value::Text("https://example.org/mascot".to_string()))
    );
    assert_eq!(
        edited.get(multiple_choice_proposal::properties::DESCRIPTION_ID),
        Some(&Value::Identifier(rewritten.to_buffer()))
    );

    // to the millisecond
    let edit = edit_transition(
        &contract,
        &mut proposer,
        edited,
        &[(
            multiple_choice_proposal::properties::TITLE,
            Value::Text("Which animal for Platform?".to_string()),
        )],
        platform_version,
    )
    .await;
    process_valid(&platform, edit, created_at + TWO_HOURS_MS, platform_version);

    // and no longer after that, whichever of them the edit changes
    for (property, value) in [
        (
            multiple_choice_proposal::properties::TITLE,
            Value::Text("Which bird?".to_string()),
        ),
        (
            multiple_choice_proposal::properties::DESCRIPTION_ID,
            Value::Identifier(later_text.to_buffer()),
        ),
        (
            multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID,
            Value::Identifier(description_host(false, platform_version).id().to_buffer()),
        ),
        (
            multiple_choice_proposal::properties::DESCRIPTION_DOCUMENT_TYPE,
            Value::Text(KEPT_TEXT.to_string()),
        ),
        (
            multiple_choice_proposal::properties::URL,
            Value::Text("https://example.org/bird".to_string()),
        ),
    ] {
        let edit = edit_transition(
            &contract,
            &mut proposer,
            question(&platform),
            &[(property, value)],
            platform_version,
        )
        .await;
        let error = process_refused(
            &platform,
            edit,
            created_at + TWO_HOURS_MS + 1,
            platform_version,
        );
        assert!(
            matches!(
                error,
                ConsensusError::StateError(StateError::DocumentImmutablePropertyChangedError(_))
            ),
            "{property}: {error:?}"
        );
    }
    assert_eq!(
        question(&platform).get(multiple_choice_proposal::properties::TITLE),
        Some(&Value::Text("Which animal for Platform?".to_string()))
    );
}

#[tokio::test]
async fn should_let_only_its_owner_enter_choices_for_a_proposal_that_is_not_open() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let poll_duration = windows(&platform, platform_version).poll_duration_ms;
    let mut proposer = writer(&mut platform, &mut rng);
    let mut alice = writer(&mut platform, &mut rng);
    let proposal_id = ask_stating(
        &platform,
        &contract,
        &mut proposer,
        "Which mascot?",
        false,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let poll = choice_poll(proposal_id);

    // Anyone else is refused, and opens no vote
    let (transition, _) = choice_transition(
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, transition, 20_000, platform_version);
    match &error {
        ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(e)) => {
            assert_eq!(
                e.constraint(),
                choice::rules::CLOSED_PROPOSAL_TAKES_ITS_OWNERS_CHOICES
            );
        }
        other => panic!("expected the proposal's rule to refuse it, got {other:?}"),
    }
    assert_eq!(status(&platform, &poll, platform_version), None);

    // The proposer supplies the choices, as many as it wants
    let start = 30_000;
    let mut choices = Vec::new();
    for (index, answer) in ["A fox", "A heron", "A lynx"].into_iter().enumerate() {
        choices.push(
            enter(
                &platform,
                &contract,
                &mut proposer,
                proposal_id,
                answer,
                start + index as u64 * 1_000,
                &mut rng,
                platform_version,
            )
            .await,
        );
    }
    assert!(matches!(
        status(&platform, &poll, platform_version),
        Some(ContestedDocumentVotePollStatus::Started(_))
    ));

    // and still nobody else joins them
    let (transition, _) = choice_transition(
        &contract,
        &mut alice,
        proposal_id,
        "An owl",
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, transition, 40_000, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(_))
        ),
        "{error:?}"
    );

    for (seed, vote_choice) in [
        (1, ResourceVoteChoice::TowardsIdentity(choices[1])),
        (2, ResourceVoteChoice::TowardsIdentity(choices[0])),
        (3, ResourceVoteChoice::TowardsIdentity(choices[1])),
    ] {
        vote(
            &mut platform,
            &poll,
            vote_choice,
            seed,
            start + 60_000,
            platform_version,
        )
        .await;
    }
    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        Some(ContestedDocumentVotePollStatus::Awarded(choices[1]))
    );
    let kept = stored_choice(&platform, &contract, choices[1]).expect("the heron is kept");
    assert_eq!(kept.owner_id(), proposer.id());
    assert_eq!(
        kept.properties().get(choice::properties::TITLE),
        Some(&Value::Text("A heron".to_string()))
    );
    for other in [choices[0], choices[2]] {
        assert!(stored_choice(&platform, &contract, other).is_none());
    }
}

#[tokio::test]
async fn should_never_let_a_proposal_change_whether_it_is_open() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let mut proposer = writer(&mut platform, &mut rng);
    let created_at = 10_000;
    // Within the two hours its text may still change in
    let edited_at = created_at + 60_000;
    for open in [true, false] {
        let proposal_id = ask_stating(
            &platform,
            &contract,
            &mut proposer,
            "Which mascot?",
            open,
            created_at,
            &mut rng,
            platform_version,
        )
        .await;
        let question = stored_document(
            &platform,
            &contract,
            multiple_choice_proposal::NAME,
            proposal_id,
        )
        .expect("the question is stored");
        let edit = edit_transition(
            &contract,
            &mut proposer,
            question,
            &[(
                multiple_choice_proposal::properties::OPEN,
                Value::Bool(!open),
            )],
            platform_version,
        )
        .await;
        let error = process_refused(&platform, edit, edited_at, platform_version);
        assert!(
            matches!(
                error,
                ConsensusError::StateError(StateError::DocumentImmutablePropertyChangedError(_))
            ),
            "{open} to {}: {error:?}",
            !open
        );
    }
}

/// What is being decided is said by a document the proposal points at. The contract it names
/// is one whose moderators the masternodes elect, the document type a moderated one whose
/// text a removal takes away, and the document is there.
#[tokio::test]
async fn should_take_the_description_from_a_moderated_text_of_an_elected_contract() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let mut proposer = writer(&mut platform, &mut rng);
    let elected = description_host(true, platform_version);
    let owner_moderated = description_host(false, platform_version);
    for (document_type_name, kind) in [
        (TEXT, DocumentReferenceKind::Moderated),
        (KEPT_TEXT, DocumentReferenceKind::Moderated),
        (PLAIN_TEXT, DocumentReferenceKind::Deletable),
    ] {
        assert_eq!(
            elected
                .document_type_for_name(document_type_name)
                .expect("expected the type")
                .document_reference_kind(),
            kind,
            "{document_type_name}"
        );
    }

    // A text in each place a proposal could point
    let mut texts = BTreeMap::new();
    for (host, document_type_name) in [
        (&elected, TEXT),
        (&elected, KEPT_TEXT),
        (&elected, PLAIN_TEXT),
        (&owner_moderated, TEXT),
    ] {
        let id = describe(
            &platform,
            host,
            &mut proposer,
            document_type_name,
            10_000,
            &mut rng,
            platform_version,
        )
        .await;
        texts.insert((host.id(), document_type_name), id);
    }
    let asking = |host: &DataContract, document_type_name: &'static str, id: Identifier| {
        question_properties("Which mascot?", true, host.id(), document_type_name, id)
    };
    let text_of = |host: &DataContract, document_type_name: &'static str| {
        texts[&(host.id(), document_type_name)]
    };

    // A moderated text of the elected contract
    let (accepted, _) = create_transition(
        &contract,
        &mut proposer,
        multiple_choice_proposal::NAME,
        asking(&elected, TEXT, text_of(&elected, TEXT)),
        &mut rng,
        platform_version,
    )
    .await;
    process_valid(&platform, accepted, 20_000, platform_version);

    // A type whose removal record keeps the text: taking the document down would leave it
    let (kept, _) = create_transition(
        &contract,
        &mut proposer,
        multiple_choice_proposal::NAME,
        asking(&elected, KEPT_TEXT, text_of(&elected, KEPT_TEXT)),
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, kept, 20_000, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedDocumentTypeRequirementNotMetError(_))
        ),
        "{error:?}"
    );

    // A type whose documents their owner deletes is not a moderated one
    let (plain, _) = create_transition(
        &contract,
        &mut proposer,
        multiple_choice_proposal::NAME,
        asking(&elected, PLAIN_TEXT, text_of(&elected, PLAIN_TEXT)),
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, plain, 20_000, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedDocumentTypeNotModeratedError(_))
        ),
        "{error:?}"
    );

    // A moderated text of a contract its owner moderates: the masternodes elect nobody there
    let (unelected, _) = create_transition(
        &contract,
        &mut proposer,
        multiple_choice_proposal::NAME,
        asking(&owner_moderated, TEXT, text_of(&owner_moderated, TEXT)),
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, unelected, 20_000, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedContractRequirementNotMetError(_))
        ),
        "{error:?}"
    );

    // A text that is not there, and a type the contract does not have
    let (missing, _) = create_transition(
        &contract,
        &mut proposer,
        multiple_choice_proposal::NAME,
        asking(&elected, TEXT, Identifier::new([7; 32])),
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, missing, 20_000, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_))
        ),
        "{error:?}"
    );
    let (no_type, _) = create_transition(
        &contract,
        &mut proposer,
        multiple_choice_proposal::NAME,
        asking(&elected, "essay", text_of(&elected, TEXT)),
        &mut rng,
        platform_version,
    )
    .await;
    let error = process_refused(&platform, no_type, 20_000, platform_version);
    assert!(
        matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedDocumentTypeNotFoundError(_))
        ),
        "{error:?}"
    );
}
