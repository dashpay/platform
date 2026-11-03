//! Decision proposals (protocol version 15): creating a `proposal` of the decision proposals
//! contract opens a yes/no vote poll with the proposal's rule, funded with the decision proposal
//! fund (0.5 Dash) its proposer pays. Masternodes vote on it like on any yes/no poll, and its
//! result is recorded when it closes; nothing acts on it.

use super::*;
use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::decision_proposal::{
    decision_proposal_required_fund, DecisionProposalVotePoll, DECISION_PROPOSAL_FUND_NAME,
    PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, SecurityLevel};
use dpp::platform_value::{Bytes32, Value};
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::{
    BatchTransition, BatchTransitionV0, DocumentCreateTransition,
};
use dpp::state_transition::StateTransition;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::query::DriveDocumentQuery;
use rand::prelude::StdRng;
use rand::SeedableRng;
use std::collections::BTreeMap;

/// A week, in milliseconds: the shortest a proposal may run.
const WEEK_MS: u64 = 604_800_000;

/// An identity making proposals, with the identity contract nonce its next transition uses.
struct Proposer {
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    next_nonce: IdentityNonce,
}

fn proposer(platform: &mut TempPlatform<MockCoreRPCLike>, seed: u64) -> Proposer {
    // Enough for a few proposals and their fees
    let (identity, signer, key) = setup_identity(platform, seed, 300_000_000_000);
    Proposer {
        identity,
        signer,
        key,
        next_nonce: 1,
    }
}

fn decision_proposals_contract(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> Arc<DataContract> {
    platform
        .drive
        .cache
        .system_data_contracts
        .load_decision_proposals(platform_version)
        .expect("expected the decision proposals contract at genesis")
}

/// A proposal ending at `end_date` that passes on strictly more than half of yes plus no, with
/// at least half the network's voting power cast.
fn proposal(title: &str, end_date: u64) -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("title".to_string(), Value::Text(title.to_string())),
        (
            "description".to_string(),
            Value::Text("Yes raises it, no keeps it.".to_string()),
        ),
        ("passPercent".to_string(), Value::U8(50)),
        ("minimumTurnoutPercent".to_string(), Value::U8(50)),
        ("endDate".to_string(), Value::U64(end_date)),
    ])
}

/// A serialized create of a proposal holding `properties`, signed by `proposer`, stating
/// `stated_fund` (the decision proposal fund when `None`), and the id of the proposal.
async fn proposal_transition(
    contract: &DataContract,
    proposer: &mut Proposer,
    properties: BTreeMap<String, Value>,
    stated_fund: Option<Option<(String, Credits)>>,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (Vec<u8>, Identifier) {
    let document_type = contract
        .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
        .expect("expected the proposal document type");
    let entropy = Bytes32::random_with_rng(rng);
    let nonce = proposer.next_nonce;
    proposer.next_nonce += 1;
    let mut document: Document = DocumentV0 {
        owner_id: proposer.identity.id(),
        properties,
        ..Default::default()
    }
    .into();
    document
        .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
        .expect("expected to set the document id");
    let id = document.id();
    let create_transition: DocumentCreateTransition = DocumentCreateTransitionV0 {
        base: DocumentBaseTransition::from_document(
            &document,
            document_type,
            None,
            nonce,
            platform_version,
            None,
        )
        .expect("expected a base transition"),
        entropy: entropy.0,
        data: document.properties_consumed(),
        prefunded_voting_balance: stated_fund.unwrap_or_else(|| {
            Some((
                DECISION_PROPOSAL_FUND_NAME.to_string(),
                decision_proposal_required_fund(platform_version),
            ))
        }),
    }
    .into();
    let batch: BatchTransition = BatchTransitionV0 {
        owner_id: proposer.identity.id(),
        transitions: vec![create_transition.into()],
        user_fee_increase: 0,
        signature_public_key_id: 0,
        signature: Default::default(),
    }
    .into();
    let mut transition: StateTransition = batch.into();
    transition
        .sign_external(
            &proposer.key,
            &proposer.signer,
            Some(|_, _| Ok(SecurityLevel::HIGH)),
        )
        .await
        .expect("expected to sign");
    (
        transition
            .serialize_to_bytes()
            .expect("expected to serialize the transition"),
        id,
    )
}

/// Processes one transition in a block at `time_ms` and returns its execution result.
fn process(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: u64,
    platform_version: &PlatformVersion,
) -> StateTransitionExecutionResult {
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[transition],
            &platform_state,
            &BlockInfo {
                time_ms,
                height: 1,
                core_height: 42,
                epoch: Default::default(),
            },
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
    time_ms: u64,
    platform_version: &PlatformVersion,
) -> FeeResult {
    match process(platform, transition, time_ms, platform_version) {
        SuccessfulExecution { fee_result, .. } => fee_result,
        other => panic!("expected the proposal to pass, got {other:?}"),
    }
}

fn process_refused(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: u64,
    platform_version: &PlatformVersion,
) -> ConsensusError {
    match process(platform, transition, time_ms, platform_version) {
        StateTransitionExecutionResult::PaidConsensusError { error, .. } => error,
        other => panic!("expected a paid refusal, got {other:?}"),
    }
}

/// A serialized edit of `proposal` by `proposer` (its owner) setting `changes`, its revision
/// bumped.
async fn edit_transition(
    contract: &DataContract,
    proposer: &mut Proposer,
    mut proposal: Document,
    changes: &[(&str, Value)],
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    for (property, value) in changes {
        proposal.set(property, value.clone());
    }
    proposal.increment_revision().expect("revision increments");
    let nonce = proposer.next_nonce;
    proposer.next_nonce += 1;
    BatchTransition::new_document_replacement_transition_from_document(
        proposal,
        contract
            .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("expected the proposal document type"),
        &proposer.key,
        nonce,
        0,
        None,
        &proposer.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to build the edit")
    .serialize_to_bytes()
    .expect("expected to serialize the edit")
}

/// The poll the proposal `document_id` holding `properties` opens.
fn proposal_poll(
    document_id: Identifier,
    properties: &BTreeMap<String, Value>,
    platform_version: &PlatformVersion,
) -> YesNoVotePoll {
    DecisionProposalVotePoll::from_proposal(document_id, properties, platform_version)
        .expect("active at 15")
        .into_data()
        .expect("a valid rule")
        .vote_poll
}

fn prefunded_balance(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &YesNoVotePoll,
    platform_version: &PlatformVersion,
) -> Option<Credits> {
    platform
        .drive
        .fetch_prefunded_specialized_balance(
            vote_poll
                .specialized_balance_id(platform_version)
                .expect("expected the balance id")
                .to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to read the balance")
}

fn identity_balance(
    platform: &TempPlatform<MockCoreRPCLike>,
    identity_id: Identifier,
    platform_version: &PlatformVersion,
) -> Credits {
    platform
        .drive
        .fetch_identity_balance(identity_id.to_buffer(), None, platform_version)
        .expect("expected to read the balance")
        .expect("expected the identity")
}

fn stored_proposal(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    document_id: Identifier,
) -> Option<Document> {
    let document_type = contract
        .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
        .expect("expected the proposal document type");
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
        .expect("expected to query the proposal")
        .documents_owned()
        .pop()
}

#[tokio::test]
async fn should_open_and_fund_the_poll_of_a_proposal_when_it_is_created() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5104);
    let mut proposer = proposer(&mut platform, 1);
    let balance_before = identity_balance(&platform, proposer.identity.id(), platform_version);

    let properties = proposal("Raise the limit?", END_DATE);
    let (transition, document_id) = proposal_transition(
        &contract,
        &mut proposer,
        properties.clone(),
        None,
        &mut rng,
        platform_version,
    )
    .await;
    // The mempool admits it at both levels, the fee estimate of the opening included
    for (level, verdict) in check_tx_verdicts(&platform, &transition) {
        assert_eq!(verdict, Ok(true), "check_tx {level:?} admits the proposal");
    }
    let fee_result = process_valid(&platform, transition, 0, platform_version);

    let fund = decision_proposal_required_fund(platform_version);
    assert_eq!(
        fund, 50_000_000_000,
        "the decision proposal fund is 0.5 Dash"
    );
    let vote_poll = proposal_poll(document_id, &properties, platform_version);
    // The poll is open, with the proposal's rule, from this block, until the proposal's end date
    let stored_info = poll_state(&platform, &vote_poll, platform_version)
        .stored_info
        .expect("expected the poll to be open");
    assert_eq!(stored_info.vote_poll(), &vote_poll);
    assert!(matches!(
        stored_info.status(),
        YesNoVotePollStatus::Started(block) if block.time_ms == 0
    ));
    let end_dates = vote_poll_end_dates(&platform.drive, platform_version);
    assert_eq!(
        end_dates.get(&END_DATE).map(|polls| polls.len()),
        Some(1),
        "the poll is listed under the proposal's end date"
    );
    // funded by the proposer with the decision proposal fund, besides the create's fees
    assert_eq!(
        prefunded_balance(&platform, &vote_poll, platform_version),
        Some(fund)
    );
    assert_eq!(
        balance_before - identity_balance(&platform, proposer.identity.id(), platform_version),
        fund + fee_result.total_base_fee()
    );
    // and the proposal is stored.
    assert!(stored_proposal(&platform, &contract, document_id).is_some());
}

#[tokio::test]
async fn should_refuse_a_proposal_stating_any_other_fund() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5105);
    let mut proposer = proposer(&mut platform, 2);
    let fund = decision_proposal_required_fund(platform_version);

    for stated_fund in [
        None,
        Some((DECISION_PROPOSAL_FUND_NAME.to_string(), fund - 1)),
        Some((DECISION_PROPOSAL_FUND_NAME.to_string(), fund + 1)),
        Some(("byEndDate".to_string(), fund)),
    ] {
        let properties = proposal("Raise the limit?", END_DATE);
        let (transition, document_id) = proposal_transition(
            &contract,
            &mut proposer,
            properties.clone(),
            Some(stated_fund.clone()),
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            check_tx_verdicts(&platform, &transition)[0].1,
            Ok(false),
            "check_tx refuses a proposal stating {stated_fund:?}"
        );
        let error = process_refused(&platform, transition, 0, platform_version);
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::DecisionProposalFundMismatchError(error))
                if error.document_id() == document_id
                    && error.required_fund() == fund
                    && error.stated_fund() == stated_fund.as_ref(),
            "a proposal stating {stated_fund:?} is refused"
        );
        let vote_poll = proposal_poll(document_id, &properties, platform_version);
        assert!(poll_state(&platform, &vote_poll, platform_version)
            .stored_info
            .is_none());
        assert_eq!(
            prefunded_balance(&platform, &vote_poll, platform_version),
            None
        );
    }
    assert!(vote_poll_end_dates(&platform.drive, platform_version).is_empty());
}

#[tokio::test]
async fn should_refuse_a_proposal_whose_rule_is_outside_the_limits() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5106);
    let mut proposer = proposer(&mut platform, 3);

    // Out of range: a pass threshold below 50 or above 80 percent, a turnout above half the
    // network, a rule left out. The contract's schema refuses each.
    let with = |property: &str, value: Option<u8>| {
        let mut properties = proposal("Out of range", END_DATE);
        match value {
            Some(value) => properties.insert(property.to_string(), Value::U8(value)),
            None => properties.remove(property),
        };
        properties
    };
    for (property, value) in [
        ("passPercent", Some(49)),
        ("passPercent", Some(81)),
        ("passPercent", None),
        ("minimumTurnoutPercent", Some(51)),
        ("minimumTurnoutPercent", None),
    ] {
        let (transition, document_id) = proposal_transition(
            &contract,
            &mut proposer,
            with(property, value),
            None,
            &mut rng,
            platform_version,
        )
        .await;
        assert_matches!(
            process_refused(&platform, transition, 0, platform_version),
            ConsensusError::BasicError(_),
            "{property} {value:?} is refused"
        );
        assert!(stored_proposal(&platform, &contract, document_id).is_none());
    }
    assert!(vote_poll_end_dates(&platform.drive, platform_version).is_empty());

    // At the limits: 50 and 80 percent to pass, no turnout and half the network.
    for (pass_percent, minimum_turnout_percent) in [(50, 50), (80, 0)] {
        let mut properties = with("passPercent", Some(pass_percent));
        properties.insert(
            "minimumTurnoutPercent".to_string(),
            Value::U8(minimum_turnout_percent),
        );
        let (transition, _) = proposal_transition(
            &contract,
            &mut proposer,
            properties,
            None,
            &mut rng,
            platform_version,
        )
        .await;
        process_valid(&platform, transition, 0, platform_version);
    }
}

#[tokio::test]
async fn should_hold_a_proposal_to_an_end_date_one_to_four_weeks_away() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5107);
    let mut proposer = proposer(&mut platform, 4);
    let block_time = 1_000;

    // The mempool refuses one ending far too soon, judged against the last block
    let (transition, _) = proposal_transition(
        &contract,
        &mut proposer,
        proposal("Far too soon", block_time + 1),
        None,
        &mut rng,
        platform_version,
    )
    .await;
    assert_eq!(
        check_tx_verdicts(&platform, &transition)[0].1,
        Ok(false),
        "check_tx refuses a proposal ending far too soon"
    );

    for end_date in [block_time + WEEK_MS - 1, block_time + 4 * WEEK_MS + 1] {
        let (transition, document_id) = proposal_transition(
            &contract,
            &mut proposer,
            proposal("Out of range", end_date),
            None,
            &mut rng,
            platform_version,
        )
        .await;
        let error = process_refused(&platform, transition, block_time, platform_version);
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::DecisionProposalEndDateOutOfRangeError(error))
                if error.document_id() == document_id
                    && error.end_date() == end_date
                    && error.earliest_end_date() == block_time + WEEK_MS
                    && error.latest_end_date() == block_time + 4 * WEEK_MS,
            "a proposal ending at {end_date} is refused"
        );
    }
    assert!(vote_poll_end_dates(&platform.drive, platform_version).is_empty());

    for end_date in [block_time + WEEK_MS, block_time + 4 * WEEK_MS] {
        let (transition, _) = proposal_transition(
            &contract,
            &mut proposer,
            proposal("In range", end_date),
            None,
            &mut rng,
            platform_version,
        )
        .await;
        process_valid(&platform, transition, block_time, platform_version);
    }
    let end_dates = vote_poll_end_dates(&platform.drive, platform_version);
    assert_eq!(
        end_dates.keys().copied().collect::<Vec<_>>(),
        vec![block_time + WEEK_MS, block_time + 4 * WEEK_MS]
    );
}

#[tokio::test]
async fn should_open_one_poll_per_proposal_even_for_the_same_question() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5108);
    let mut first = proposer(&mut platform, 5);
    let mut second = proposer(&mut platform, 6);

    let properties = proposal("Raise the limit?", END_DATE);
    let mut polls = vec![];
    for proposer in [&mut first, &mut second] {
        let (transition, document_id) = proposal_transition(
            &contract,
            proposer,
            properties.clone(),
            None,
            &mut rng,
            platform_version,
        )
        .await;
        process_valid(&platform, transition, 0, platform_version);
        polls.push(proposal_poll(document_id, &properties, platform_version));
    }
    assert_ne!(
        polls[0].unique_id(platform_version).expect("id"),
        polls[1].unique_id(platform_version).expect("id")
    );
    for vote_poll in &polls {
        assert!(poll_state(&platform, vote_poll, platform_version)
            .stored_info
            .is_some());
    }
}

#[tokio::test]
async fn should_record_the_result_of_a_proposal_voted_on_and_keep_the_proposal() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5109);
    let mut proposer = proposer(&mut platform, 7);

    let properties = proposal("Raise the limit?", END_DATE);
    let (transition, document_id) = proposal_transition(
        &contract,
        &mut proposer,
        properties.clone(),
        None,
        &mut rng,
        platform_version,
    )
    .await;
    process_valid(&platform, transition, 0, platform_version);
    let vote_poll = proposal_poll(document_id, &properties, platform_version);

    // Two evonodes vote yes (4 each) and a masternode votes no (1): 8 of 9 is more than half,
    // and 9 reaches half the network (these three, 9 voting power), rounded up to 5.
    for (seed, evonode, vote_choice) in [
        (700, true, YesNoAbstainVoteChoice::Yes),
        (701, true, YesNoAbstainVoteChoice::Yes),
        (702, false, YesNoAbstainVoteChoice::No),
    ] {
        let (pro_tx_hash, signer, voting_key) =
            setup_voter(&mut platform, seed, evonode, platform_version);
        perform_yes_no_vote(
            &mut platform,
            &vote_poll,
            vote_choice,
            &signer,
            pro_tx_hash,
            &voting_key,
            1,
            platform_version,
        )
        .await
        .expect("expected the vote to be accepted");
    }
    let fund = decision_proposal_required_fund(platform_version);
    let vote_cost = platform_version
        .fee_version
        .vote_resolution_fund_fees
        .contested_document_single_vote_cost;
    assert_eq!(
        prefunded_balance(&platform, &vote_poll, platform_version),
        Some(fund - 3 * vote_cost)
    );
    let processing_credits_before = processing_credits(&platform, platform_version);

    close_ended_polls(&mut platform, platform_version);

    let result = finished_result(&platform, &vote_poll, platform_version);
    assert!(result.passed);
    assert_eq!(
        (
            result.yes_voting_power,
            result.no_voting_power,
            result.required_voting_power
        ),
        (8, 1, 5)
    );
    // What the votes left of the fund goes to processing fees
    assert_eq!(
        prefunded_balance(&platform, &vote_poll, platform_version),
        None
    );
    assert_eq!(
        processing_credits(&platform, platform_version) - processing_credits_before,
        fund - 3 * vote_cost
    );
    assert!(vote_poll_end_dates(&platform.drive, platform_version).is_empty());
    // The proposal stays, and the queries prove the result.
    assert!(stored_proposal(&platform, &contract, document_id).is_some());
    let state = queried_poll_state(
        &platform,
        vote_poll.unique_id(platform_version).expect("id"),
        platform_version,
    );
    assert_matches!(
        state.status,
        Some(state_query_response::yes_no_vote_poll_state::Status::Finished(finished))
            if finished.passed
    );
}

#[tokio::test]
async fn should_let_the_proposer_edit_its_text_for_a_day_and_keep_every_version() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5110);
    let mut proposer = proposer(&mut platform, 8);
    const HOUR_MS: u64 = 3_600_000;
    // Not at time 0: a document's history is read from after a time
    let created_at = 1_000;

    let mut properties = proposal("Raise the limit?", END_DATE);
    properties.insert(
        "url".to_string(),
        Value::Text("https://example.org/first".to_string()),
    );
    let (transition, document_id) = proposal_transition(
        &contract,
        &mut proposer,
        properties.clone(),
        None,
        &mut rng,
        platform_version,
    )
    .await;
    process_valid(&platform, transition, created_at, platform_version);
    let vote_poll = proposal_poll(document_id, &properties, platform_version);
    let original = stored_proposal(&platform, &contract, document_id).expect("stored");

    // Within the day, the rule is still fixed
    let edit = edit_transition(
        &contract,
        &mut proposer,
        original.clone(),
        &[("endDate", Value::U64(END_DATE + 1))],
        platform_version,
    )
    .await;
    assert_matches!(
        process_refused(&platform, edit, created_at + HOUR_MS, platform_version),
        ConsensusError::StateError(StateError::DocumentImmutablePropertyChangedError(_)),
        "the end date never changes"
    );

    // but the title, the description and the url may change
    let edit = edit_transition(
        &contract,
        &mut proposer,
        original.clone(),
        &[
            ("title", Value::Text("Raise the limit to 10?".to_string())),
            (
                "description",
                Value::Text("Yes raises it to 10, no keeps it.".to_string()),
            ),
            ("url", Value::Text("https://example.org/second".to_string())),
        ],
        platform_version,
    )
    .await;
    process_valid(&platform, edit, created_at + 12 * HOUR_MS, platform_version);
    let edited = stored_proposal(&platform, &contract, document_id).expect("stored");
    assert_eq!(
        edited.get("title"),
        Some(&Value::Text("Raise the limit to 10?".to_string()))
    );

    // and no longer once the day has passed.
    let edit = edit_transition(
        &contract,
        &mut proposer,
        edited.clone(),
        &[("title", Value::Text("Raise the limit to 20?".to_string()))],
        platform_version,
    )
    .await;
    assert_matches!(
        process_refused(
            &platform,
            edit,
            created_at + 24 * HOUR_MS + 1,
            platform_version
        ),
        ConsensusError::StateError(StateError::DocumentImmutablePropertyChangedError(_)),
        "the title is frozen after a day"
    );

    // Every version is kept: a voter sees the text as it was when it voted.
    let document_type = contract
        .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
        .expect("expected the proposal document type");
    let history = platform
        .drive
        .fetch_document_history(
            contract.id().to_buffer(),
            document_type,
            document_id.to_buffer(),
            None,
            0,
            None,
            None,
            platform_version,
        )
        .expect("expected the history");
    let titles: Vec<&Value> = history
        .values()
        .filter_map(|version| version.get("title"))
        .collect();
    assert_eq!(
        titles,
        vec![
            &Value::Text("Raise the limit?".to_string()),
            &Value::Text("Raise the limit to 10?".to_string())
        ]
    );

    // The poll is the one the proposal opened, its rule unchanged.
    let stored_info = poll_state(&platform, &vote_poll, platform_version)
        .stored_info
        .expect("expected the poll to be open");
    assert_eq!(stored_info.vote_poll(), &vote_poll);
}

#[tokio::test]
async fn should_refuse_a_proposal_whose_owner_cannot_pay_both_the_fund_and_the_fees() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let contract = decision_proposals_contract(&platform, platform_version);
    let mut rng = StdRng::seed_from_u64(0xDEC1_5111);
    let fund = decision_proposal_required_fund(platform_version);

    // What a proposal costs in fees, besides its fund
    let mut measuring = proposer(&mut platform, 9);
    let (transition, _) = proposal_transition(
        &contract,
        &mut measuring,
        proposal("Raise the limit?", END_DATE),
        None,
        &mut rng,
        platform_version,
    )
    .await;
    let fees = process_valid(&platform, transition, 0, platform_version).total_base_fee();

    // An owner holding the fund and half those fees: the fund counts in the balance the
    // transition needs, so it is refused before anything is written
    let (identity, signer, key) = setup_identity(&mut platform, 10, fund + fees / 2);
    let mut short = Proposer {
        identity,
        signer,
        key,
        next_nonce: 1,
    };
    let properties = proposal("Lower the limit?", END_DATE);
    let (transition, document_id) = proposal_transition(
        &contract,
        &mut short,
        properties.clone(),
        None,
        &mut rng,
        platform_version,
    )
    .await;
    let result = process(&platform, transition, 0, platform_version);
    assert!(
        !matches!(result, SuccessfulExecution { .. } | InternalError(_)),
        "an owner short of the fund and the fees is refused, got {result:?}"
    );
    let vote_poll = proposal_poll(document_id, &properties, platform_version);
    assert!(poll_state(&platform, &vote_poll, platform_version)
        .stored_info
        .is_none());
    assert_eq!(
        prefunded_balance(&platform, &vote_poll, platform_version),
        None
    );
    assert!(stored_proposal(&platform, &contract, document_id).is_none());
    assert!(identity_balance(&platform, short.identity.id(), platform_version) > fund);
}
