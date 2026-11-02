//! The end of a contested vote poll holding many contenders: the tally reaches every one, and
//! the cleanup built from it leaves none of their entries behind.

use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0Setters;
use dpp::identifier::Identifier;
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::TimestampMillis;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use drive::drive::votes::paths::{
    vote_contested_resource_identity_votes_tree_path_vec, VotePollPaths,
    RESOURCE_STORED_INFO_KEY_U8_32,
};
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{PathQuery, Query, SizedQuery, Transaction};
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DataContractOwnedResolvedInfo, OwnedDocumentInfo};
use drive::util::storage_flags::StorageFlags;
use drive::util::test_helpers::vote_poll_end_dates;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// The identity id of contender `n`: `n + 1` big endian in its first 8 bytes, so contenders
/// sort by `n`.
fn contender_id(n: u64) -> Identifier {
    let mut id = [0u8; 32];
    id[..8].copy_from_slice(&(n + 1).to_be_bytes());
    Identifier::from(id)
}

/// The pro tx hash of voter `n`
fn voter(n: u64) -> [u8; 32] {
    let mut pro_tx_hash = [0xFFu8; 32];
    pro_tx_hash[..8].copy_from_slice(&n.to_be_bytes());
    pro_tx_hash
}

/// A DPNS name contest on `label` with `contenders` contenders (see [`contender_id`]), written
/// straight to Drive at block time 0. Every document is created at time 1, but the last
/// contender's at time 0. `voted` masternodes vote, one each, for the contenders in turn. Returns the
/// poll and its end time.
fn start_contest(
    platform: &TempPlatform<MockCoreRPCLike>,
    label: &str,
    contenders: u64,
    voted: u64,
    platform_version: &PlatformVersion,
) -> (
    ContestedDocumentResourceVotePollWithContractInfo,
    TimestampMillis,
) {
    let dpns_contract: DataContract =
        load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected the DPNS contract");
    let document_type = dpns_contract
        .document_type_for_name("domain")
        .expect("expected the domain document type");
    let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
        contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            Value::Text("dash".to_string()),
            Value::Text(label.to_string()),
        ],
    };
    let block_info = BlockInfo::default();
    let mut rng = StdRng::seed_from_u64(contenders);
    for n in 0..contenders {
        let owner_id = contender_id(n);
        let mut document = document_type
            .random_document_with_params(
                owner_id,
                Bytes32::random_with_rng(&mut rng),
                Some(if n + 1 == contenders { 0 } else { 1 }),
                Some(1),
                Some(1),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::MinDocumentFillSize,
                &mut rng,
                platform_version,
            )
            .expect("expected a random domain");
        document.set("parentDomainName", "dash".into());
        document.set("normalizedParentDomainName", "dash".into());
        document.set("label", label.into());
        document.set("normalizedLabel", label.into());
        document.set("records.identity", owner_id.into());
        document.set("subdomainRules.allowSubdomains", false.into());
        let stored_info = (n == 0).then(|| {
            ContestedDocumentVotePollStoredInfo::new(block_info, platform_version)
                .expect("expected the poll's stored info")
        });
        platform
            .drive
            .add_contested_document(
                OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_id.to_buffer()),
                },
                vote_poll.clone(),
                false,
                stored_info,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add the contender");
    }
    for n in 0..voted {
        platform
            .drive
            .register_contested_resource_identity_vote(
                voter(n),
                1,
                vote_poll.clone(),
                ResourceVoteChoice::TowardsIdentity(contender_id(n % contenders)),
                None,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to register the vote");
    }
    let end_dates = vote_poll_end_dates(&platform.drive, platform_version);
    let [end_time]: [TimestampMillis; 1] = end_dates
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .try_into()
        .expect("expected the contest to end at one time");
    (vote_poll, end_time)
}

/// The block that ends the polls due at `time_ms`
fn ending_block(time_ms: TimestampMillis) -> BlockInfo {
    BlockInfo {
        time_ms,
        height: 2,
        core_height: 42,
        epoch: Default::default(),
    }
}

/// Every key directly under `path`
fn keys_under(
    platform: &TempPlatform<MockCoreRPCLike>,
    path: Vec<Vec<u8>>,
    transaction: &Transaction,
    platform_version: &PlatformVersion,
) -> Vec<Vec<u8>> {
    let mut query = Query::new();
    query.insert_all();
    match platform.drive.grove_get_raw_path_query(
        &PathQuery::new(path, SizedQuery::new(query, None, None)),
        Some(transaction),
        QueryResultType::QueryKeyElementPairResultType,
        &mut vec![],
        &platform_version.drive,
    ) {
        Ok((elements, _)) => elements.to_keys(),
        Err(drive::error::Error::GroveDB(error))
            if matches!(
                *error,
                drive::grovedb::Error::PathNotFound(_)
                    | drive::grovedb::Error::PathParentLayerNotFound(_)
                    | drive::grovedb::Error::PathKeyNotFound(_)
            ) =>
        {
            vec![]
        }
        Err(error) => panic!("expected to read the keys: {error:?}"),
    }
}

/// The poll's stored info
fn stored_info(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
    transaction: &Transaction,
    platform_version: &PlatformVersion,
) -> ContestedDocumentVotePollStoredInfo {
    platform
        .drive
        .fetch_contested_document_vote_poll_stored_info(
            vote_poll,
            None,
            Some(transaction),
            platform_version,
        )
        .expect("expected to read the stored info")
        .1
        .expect("expected the poll to keep its stored info")
}

/// What the end of a poll leaves: the keys under its choices, its contested documents and the
/// voters' vote records
fn left_behind(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
    transaction: &Transaction,
    platform_version: &PlatformVersion,
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let choices = keys_under(
        platform,
        vote_poll
            .contenders_path(platform_version)
            .expect("expected the choices path"),
        transaction,
        platform_version,
    );
    let documents = keys_under(
        platform,
        vote_poll.documents_storage_path_vec(),
        transaction,
        platform_version,
    );
    let voters = keys_under(
        platform,
        vote_contested_resource_identity_votes_tree_path_vec(),
        transaction,
        platform_version,
    )
    .into_iter()
    .filter(|voter_tree| {
        !keys_under(
            platform,
            vote_contested_resource_identity_votes_tree_path_vec()
                .into_iter()
                .chain([voter_tree.clone()])
                .collect(),
            transaction,
            platform_version,
        )
        .is_empty()
    })
    .collect();
    (choices, documents, voters)
}

#[test]
fn should_end_a_poll_of_more_contenders_than_protocol_13_tallied_leaving_nothing_behind() {
    let platform_version = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (vote_poll, end_time) = start_contest(&platform, "quantum", 150, 0, platform_version);

    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform
        .check_for_ended_vote_polls(
            &platform_state,
            &platform_state,
            &ending_block(end_time),
            Some(&transaction),
            platform_version,
        )
        .expect("expected the block to end the poll");

    let (choices, contested_documents, _) =
        left_behind(&platform, &vote_poll, &transaction, platform_version);
    assert_eq!(choices, vec![RESOURCE_STORED_INFO_KEY_U8_32.to_vec()]);
    assert!(contested_documents.is_empty());

    // Nobody voted, so all 150 tie, and the earliest document wins: the last contender's
    let stored_info = stored_info(&platform, &vote_poll, &transaction, platform_version);
    assert_eq!(
        stored_info.vote_poll_status(),
        ContestedDocumentVotePollStatus::Awarded(contender_id(149))
    );
    assert_eq!(
        stored_info
            .contender_votes_in_vec_of_contender_with_serialized_document()
            .expect("expected the contenders of the finished poll")
            .len(),
        150
    );
}

/// Protocol version 13 tallies at most 100 contenders, and its cleanup, built from the tally,
/// removes the entries of the contenders it tallied; kept for replay.
#[test]
fn should_clean_up_the_tallied_contenders_protocol_version_13() {
    let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
    let platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(13)
        .build_with_mock_rpc()
        .set_genesis_state();
    let (vote_poll, end_time) = start_contest(&platform, "quantum", 150, 0, platform_version);

    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform
        .check_for_ended_vote_polls(
            &platform_state,
            &platform_state,
            &ending_block(end_time),
            Some(&transaction),
            platform_version,
        )
        .expect("expected the block to end the poll");

    let (choices, contested_documents, _) =
        left_behind(&platform, &vote_poll, &transaction, platform_version);
    let mut expected = vec![RESOURCE_STORED_INFO_KEY_U8_32.to_vec()];
    expected.extend((100..150).map(|n| contender_id(n).to_vec()));
    assert_eq!(choices, expected);
    // The documents removal reads every document, tallied or not
    assert!(contested_documents.is_empty());
}

/// Measures the end of a poll holding the most contenders a contest accepts. Run in release
/// on drive-abci's 8 MiB runtime stack:
///
/// `CONTENDERS` defaults to `max_contenders_per_contest`; `VOTED` masternodes (2,000 by default)
/// vote, one each, for the contenders in turn.
///
/// ```text
/// CONTENDERS=1000 VOTED=3000 cargo test --release -p drive-abci --lib \
///   should_end_a_poll_of_the_most_contenders_a_contest_accepts -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn should_end_a_poll_of_the_most_contenders_a_contest_accepts() {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(end_a_full_poll)
        .expect("expected to spawn the measuring thread")
        .join()
        .expect("expected the measurement to succeed");
}

fn end_a_full_poll() {
    use std::time::Instant;

    let platform_version = PlatformVersion::latest();
    let contenders: u64 = std::env::var("CONTENDERS")
        .ok()
        .map(|contenders| contenders.parse().expect("expected a number of contenders"))
        .unwrap_or(platform_version.system_limits.max_contenders_per_contest as u64);
    let voted: u64 = std::env::var("VOTED")
        .ok()
        .map(|voted| voted.parse().expect("expected a number of votes"))
        .unwrap_or(2_000);
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    let started = Instant::now();
    let (vote_poll, end_time) =
        start_contest(&platform, "quantum", contenders, voted, platform_version);
    println!(
        "setup: {contenders} contenders, {voted} votes in {:?}",
        started.elapsed()
    );

    let started = Instant::now();
    let (join_fee, counted) = platform
        .drive
        .fetch_contested_document_vote_poll_contender_count(
            &vote_poll,
            platform_version.system_limits.max_contenders_per_contest,
            &Default::default(),
            None,
            platform_version,
        )
        .expect("expected the contender count");
    println!(
        "join count read: {counted} contenders in {:?}, {} processing credits",
        started.elapsed(),
        join_fee.processing_fee
    );
    // A join counts at most the limit
    assert_eq!(
        counted as u64,
        contenders.min(platform_version.system_limits.max_contenders_per_contest as u64)
    );

    let platform_state = platform.state.load();
    let block_info = ending_block(end_time);

    // The whole end of the poll, rolled back after
    {
        let transaction = platform.drive.grove.start_transaction();
        let started = Instant::now();
        platform
            .check_for_ended_vote_polls(
                &platform_state,
                &platform_state,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the block to end the poll");
        println!("end of the poll: {:?}", started.elapsed());
        let (choices, contested_documents, voters) =
            left_behind(&platform, &vote_poll, &transaction, platform_version);
        assert_eq!(choices, vec![RESOURCE_STORED_INFO_KEY_U8_32.to_vec()]);
        assert!(contested_documents.is_empty());
        assert!(voters.is_empty());
        let stored_info_bytes = platform
            .drive
            .grove_get_raw(
                vote_poll
                    .contenders_path(platform_version)
                    .expect("expected the choices path")
                    .as_slice()
                    .into(),
                &RESOURCE_STORED_INFO_KEY_U8_32,
                drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the stored info")
            .expect("expected the stored info")
            .into_item_bytes()
            .expect("expected an item");
        println!("finished poll record: {} bytes", stored_info_bytes.len());
    }

    // Its steps
    let transaction = platform.drive.grove.start_transaction();
    let started = Instant::now();
    let tally = platform
        .tally_votes_for_contested_document_resource_vote_poll(
            (&vote_poll).into(),
            Some(&transaction),
            platform_version,
        )
        .expect("expected the tally");
    println!(
        "tally: {} contenders in {:?}",
        tally.contenders.len(),
        started.elapsed()
    );
    assert_eq!(tally.contenders.len() as u64, contenders);

    let started = Instant::now();
    let (with_votes, without_votes): (Vec<_>, Vec<_>) = tally
        .contenders
        .iter()
        .partition(|contender| contender.final_vote_tally > 0);
    let mut votes = platform
        .drive
        .fetch_identities_voting_for_contenders(
            &vote_poll,
            with_votes
                .iter()
                .map(|contender| contender.identity_id)
                .collect(),
            true,
            Some(&transaction),
            platform_version,
        )
        .expect("expected the voters");
    votes.extend(without_votes.iter().map(|contender| {
        (
            ResourceVoteChoice::TowardsIdentity(contender.identity_id),
            vec![],
        )
    }));
    println!("voters: {:?}", started.elapsed());

    let started = Instant::now();
    let finished = [(&vote_poll, &end_time, &votes)];
    let operations = platform
        .clean_up_after_contested_resources_vote_polls_end_operations_v0(
            &finished,
            false,
            Some(&transaction),
            platform_version,
        )
        .expect("expected the cleanup");
    println!(
        "cleanup build: {} operations in {:?}",
        operations.len(),
        started.elapsed()
    );

    let started = Instant::now();
    platform
        .drive
        .apply_batch_low_level_drive_operations(
            None,
            Some(&transaction),
            operations,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to apply the cleanup");
    println!("cleanup apply: {:?}", started.elapsed());
}
