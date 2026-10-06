use super::*;
use crate::DapiError;
use crate::clients::tenderdash_client::{
    Block, BlockData, BlockHeader, BlockMeta, ConsensusVersion, ExecTxResult, ResultBlock,
    ResultBlockResults, ResultBlockchainInfo,
};
use crate::error::DAPIResult;
use crate::services::platform_service::subscribe_to_state_transitions::TenderdashBlocks;
use async_trait::async_trait;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use dash_platform_queries::subscriptions::{Role, StateTransitionFilter};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::StateTransition;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio_stream::StreamExt;

/// A chain held in memory: height → successful transactions.
#[derive(Default)]
struct FakeChain {
    blocks: Mutex<BTreeMap<u64, Vec<Vec<u8>>>>,
    /// A height whose execution results are not saved yet.
    results_withheld: Mutex<Option<u64>>,
    /// Block reads of heights up to `counted_below` in flight, and the most seen at once.
    reads_in_flight: AtomicUsize,
    max_reads_in_flight: AtomicUsize,
    counted_below: AtomicU64,
    /// `block` calls made.
    block_reads: AtomicUsize,
}

impl FakeChain {
    fn push(&self, txs: Vec<Vec<u8>>) -> u64 {
        let mut blocks = self.blocks.lock().unwrap();
        let height = blocks.len() as u64 + 1;
        blocks.insert(height, txs);
        height
    }

    fn txs(&self, height: u64) -> Option<Vec<Vec<u8>>> {
        self.blocks.lock().unwrap().get(&height).cloned()
    }
}

#[async_trait]
impl TenderdashBlocks for FakeChain {
    async fn latest_height(&self) -> DAPIResult<u64> {
        Ok(self.blocks.lock().unwrap().len() as u64)
    }

    async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
        self.block_reads.fetch_add(1, Ordering::SeqCst);
        if height < self.counted_below.load(Ordering::SeqCst) {
            let now = self.reads_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_reads_in_flight.fetch_max(now, Ordering::SeqCst);
            sleep(Duration::from_millis(2)).await;
            self.reads_in_flight.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(ResultBlock {
            block: self.txs(height).map(|txs| Block {
                header: BlockHeader {
                    version: ConsensusVersion {
                        block: 14,
                        app: PlatformVersion::latest().protocol_version as u64,
                    },
                    height,
                    time: "2026-10-05T12:00:00Z".to_string(),
                },
                data: BlockData {
                    txs: txs.iter().map(|tx| BASE64_STANDARD.encode(tx)).collect(),
                },
            }),
        })
    }

    async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults> {
        if *self.results_withheld.lock().unwrap() == Some(height) {
            return Err(DapiError::Client("no results for height".to_string()));
        }
        let txs = self
            .txs(height)
            .ok_or_else(|| DapiError::Client("no results for height".to_string()))?;
        Ok(ResultBlockResults {
            height,
            txs_results: txs.iter().map(|_| ExecTxResult::default()).collect(),
        })
    }

    async fn blockchain(
        &self,
        min_height: u64,
        max_height: u64,
    ) -> DAPIResult<ResultBlockchainInfo> {
        let blocks = self.blocks.lock().unwrap();
        let max_height = max_height.min(blocks.len() as u64);
        let min_height = min_height.max(max_height.saturating_sub(19)).max(1);
        Ok(ResultBlockchainInfo {
            last_height: blocks.len() as u64,
            block_metas: (min_height..=max_height)
                .rev()
                .map(|height| BlockMeta {
                    header: BlockHeader {
                        height,
                        ..Default::default()
                    },
                    num_txs: blocks[&height].len() as i64,
                })
                .collect(),
        })
    }
}

fn credit_transfer(recipient: u8) -> Vec<u8> {
    StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
        IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::from([1u8; 32]),
            recipient_id: Identifier::from([recipient; 32]),
            amount: 1000,
            ..Default::default()
        },
    ))
    .serialize_to_bytes()
    .unwrap()
}

fn recipient_filter(recipient: u8) -> ResolvedFilters {
    ResolvedFilters::resolve(
        vec![StateTransitionFilter::Identities {
            identity_ids: vec![Identifier::from([recipient; 32])],
            role: Role::Recipient,
        }],
        |_| None,
        PlatformVersion::latest(),
    )
    .unwrap()
}

struct Harness {
    chain: Arc<FakeChain>,
    blocks: Arc<BlockSource>,
    service: SubscriptionService,
    tip: watch::Sender<u64>,
}

fn harness(limits: SubscriptionLimits) -> Harness {
    let chain = Arc::new(FakeChain::default());
    let blocks = Arc::new(BlockSource::new(chain.clone()));
    let (tip, tip_receiver) = watch::channel(0);
    Harness {
        service: SubscriptionService::new(blocks.clone(), tip_receiver, limits),
        blocks,
        chain,
        tip,
    }
}

impl Harness {
    fn commit(&self, txs: Vec<Vec<u8>>) -> u64 {
        let height = self.chain.push(txs);
        self.tip.send_replace(height);
        height
    }
}

enum Message {
    Match { height: u64, index: u32 },
    Checkpoint(u64),
}

async fn next(stream: &mut SubscriptionStream) -> Message {
    // Longer than a checkpoint interval, so tests on paused time reach the next checkpoint.
    let response = timeout(CHECKPOINT_INTERVAL * 3, stream.next())
        .await
        .expect("expected a message")
        .expect("expected the stream to stay open")
        .expect("expected no error");
    let Some(Version::V0(v0)) = response.version else {
        panic!("expected a v0 response");
    };
    match v0.responses.unwrap() {
        Responses::StateTransition(matched) => Message::Match {
            height: matched.block_height,
            index: matched.index_in_block,
        },
        Responses::Checkpoint(checkpoint) => Message::Checkpoint(checkpoint.block_height),
    }
}

async fn expect_checkpoint(stream: &mut SubscriptionStream, height: u64) {
    match next(stream).await {
        Message::Checkpoint(at) => assert_eq!(at, height, "checkpoint height"),
        Message::Match { height: at, .. } => panic!("expected a checkpoint, got a match at {at}"),
    }
}

async fn expect_match(stream: &mut SubscriptionStream, height: u64, index: u32) {
    match next(stream).await {
        Message::Match {
            height: at,
            index: i,
        } => {
            assert_eq!((at, i), (height, index), "match position")
        }
        Message::Checkpoint(at) => panic!("expected a match, got a checkpoint at {at}"),
    }
}

#[tokio::test]
async fn should_replay_history_then_follow_new_blocks_without_gaps() {
    let harness = harness(SubscriptionLimits::default());
    // 45 blocks spanning three meta pages; matches at 3 and 30.
    for height in 1..=45u64 {
        let txs = match height {
            3 => vec![credit_transfer(9), credit_transfer(7)],
            30 => vec![credit_transfer(7)],
            h if h % 2 == 0 => vec![credit_transfer(9)],
            _ => vec![],
        };
        harness.commit(txs);
    }
    let mut stream = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            Some(2),
            None,
        )
        .await
        .unwrap();

    expect_checkpoint(&mut stream, 1).await;
    expect_match(&mut stream, 3, 1).await;
    expect_checkpoint(&mut stream, 3).await;
    expect_match(&mut stream, 30, 0).await;
    expect_checkpoint(&mut stream, 30).await;

    let height = harness.commit(vec![credit_transfer(9), credit_transfer(7)]);
    expect_match(&mut stream, height, 1).await;
    expect_checkpoint(&mut stream, height).await;
}

#[tokio::test]
async fn should_start_after_the_tip_when_no_height_is_given() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![credit_transfer(7)]);
    let mut stream = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            None,
            None,
        )
        .await
        .unwrap();
    expect_checkpoint(&mut stream, 1).await;
    let height = harness.commit(vec![credit_transfer(7)]);
    expect_match(&mut stream, height, 0).await;
}

#[tokio::test(start_paused = true)]
async fn should_send_checkpoints_while_nothing_matches() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![]);
    let mut stream = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            None,
            None,
        )
        .await
        .unwrap();
    expect_checkpoint(&mut stream, 1).await;
    harness.commit(vec![credit_transfer(9)]);
    expect_checkpoint(&mut stream, 2).await;
}

#[tokio::test]
async fn should_refuse_starts_too_far_behind_or_ahead_of_the_tip() {
    let harness = harness(SubscriptionLimits {
        max_replay_blocks: 10,
        max_blocks_ahead: 5,
        ..Default::default()
    });
    for _ in 0..30 {
        harness.commit(vec![]);
    }
    for from in [5, 40] {
        let error = harness
            .service
            .start(
                harness.service.admit(None).unwrap(),
                recipient_filter(7),
                Some(from),
                None,
            )
            .await
            .unwrap_err();
        assert_eq!(error.code(), tonic::Code::OutOfRange, "from {from}");
    }
}

#[tokio::test]
async fn should_limit_subscriptions_per_client_address() {
    let harness = harness(SubscriptionLimits {
        max_subscriptions_per_ip: 2,
        ..Default::default()
    });
    let client: IpAddr = "203.0.113.7".parse().unwrap();
    let first = harness.service.admit(Some(client)).unwrap();
    let _second = harness.service.admit(Some(client)).unwrap();
    assert_eq!(
        harness.service.admit(Some(client)).err().unwrap().code(),
        tonic::Code::ResourceExhausted
    );
    // Another /64 is not affected; a released slot can be reused.
    assert!(
        harness
            .service
            .admit(Some("203.0.113.8".parse().unwrap()))
            .is_ok()
    );
    drop(first);
    assert!(harness.service.admit(Some(client)).is_ok());
}

#[test]
fn should_bucket_ipv6_clients_by_64_bit_prefix() {
    let a: IpAddr = "2001:db8:1:2:aaaa::1".parse().unwrap();
    let b: IpAddr = "2001:db8:1:2:bbbb::2".parse().unwrap();
    let c: IpAddr = "2001:db8:1:3::1".parse().unwrap();
    assert_eq!(per_ip_key(a), per_ip_key(b));
    assert_ne!(per_ip_key(a), per_ip_key(c));
}

#[test]
fn should_count_ipv4_clients_of_a_dual_stack_listener_by_their_ipv4_address() {
    let mapped = |ip: &str| -> IpAddr { format!("::ffff:{ip}").parse().unwrap() };
    let native: IpAddr = "198.51.100.1".parse().unwrap();
    assert_ne!(
        per_ip_key(mapped("198.51.100.1")),
        per_ip_key(mapped("203.0.113.7"))
    );
    assert_eq!(per_ip_key(mapped("198.51.100.1")), per_ip_key(native));
}

#[tokio::test]
async fn should_end_the_stream_on_a_transaction_this_node_cannot_decode() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![vec![0xff, 0x00, 0x13]]);
    let mut stream = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            Some(1),
            None,
        )
        .await
        .unwrap();
    expect_checkpoint(&mut stream, 0).await;
    let status = timeout(Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);
}

#[tokio::test]
async fn should_deliver_a_block_once_its_results_are_saved() {
    let harness = harness(SubscriptionLimits::default());
    let mut stream = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            None,
            None,
        )
        .await
        .unwrap();
    expect_checkpoint(&mut stream, 0).await;

    *harness.chain.results_withheld.lock().unwrap() = Some(1);
    harness.commit(vec![credit_transfer(7)]);
    let chain = harness.chain.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(500)).await;
        *chain.results_withheld.lock().unwrap() = None;
    });
    expect_match(&mut stream, 1, 0).await;
    expect_checkpoint(&mut stream, 1).await;
}

#[tokio::test]
async fn should_not_let_a_client_that_stops_reading_hold_the_replay_capacity() {
    let harness = harness(SubscriptionLimits {
        max_replaying: 1,
        ..Default::default()
    });
    // Far more matches for the stalled client than its buffer holds.
    for _ in 0..100 {
        harness.commit(vec![credit_transfer(9)]);
    }
    let height = harness.commit(vec![credit_transfer(7)]);

    let _stalled = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(9),
            Some(1),
            None,
        )
        .await
        .unwrap();
    let mut reader = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            Some(1),
            None,
        )
        .await
        .unwrap();
    expect_checkpoint(&mut reader, 0).await;
    expect_match(&mut reader, height, 0).await;
}

#[tokio::test]
async fn should_bound_concurrent_catch_up_reads_after_sending_matches() {
    let harness = harness(SubscriptionLimits {
        max_replaying: 1,
        ..Default::default()
    });
    for _ in 0..100 {
        harness.commit(vec![credit_transfer(7)]);
    }
    // Heights read while a subscription is at least a page behind the tip of 100.
    harness.chain.counted_below.store(81, Ordering::SeqCst);

    let mut drains = Vec::new();
    for from in [1, 41] {
        let mut stream = harness
            .service
            .start(
                harness.service.admit(None).unwrap(),
                recipient_filter(7),
                Some(from),
                None,
            )
            .await
            .unwrap();
        drains.push(tokio::spawn(async move {
            loop {
                if let Message::Checkpoint(100) = next(&mut stream).await {
                    return;
                }
            }
        }));
    }
    for drain in drains {
        drain.await.unwrap();
    }
    assert_eq!(harness.chain.max_reads_in_flight.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn should_not_pin_decoded_blocks_while_waiting_on_a_stalled_reader() {
    let harness = harness(SubscriptionLimits::default());
    // Several matches a block, many blocks: far more than the stream buffer holds.
    for _ in 0..40 {
        harness.commit(vec![
            credit_transfer(7),
            credit_transfer(7),
            credit_transfer(7),
        ]);
    }
    let _stalled = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            Some(1),
            None,
        )
        .await
        .unwrap();
    // Let the scan fill the buffer and block on a send.
    sleep(Duration::from_millis(300)).await;

    for height in 1..=40 {
        if let Ok(BlockRead::Block(block)) = harness.blocks.read(height).await {
            // Held by the cache and by this test only, never by the stalled scan.
            assert!(
                Arc::strong_count(&block) <= 2,
                "block {height} is pinned by {} owners",
                Arc::strong_count(&block)
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn should_keep_the_oldest_waiter_first_for_replay_capacity_across_heartbeats() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![]);
    let replaying = Arc::new(Semaphore::new(0));
    let scan = |sender| Scan {
        blocks: harness.blocks.clone(),
        tip: harness.tip.subscribe(),
        replaying: replaying.clone(),
        filters: recipient_filter(7),
        sender,
        next_height: 1,
        deliver_from: 1,
    };
    let (oldest_sender, mut oldest_receiver) = mpsc::channel(STREAM_BUFFER);
    let (newer_sender, mut newer_receiver) = mpsc::channel(STREAM_BUFFER);
    let mut oldest = scan(oldest_sender);
    let mut newer = scan(newer_sender);

    // Each waiter hands back its permit, so the test holds the capacity it was granted.
    let oldest_wait = tokio::spawn(async move {
        let mut last_checkpoint = Instant::now();
        oldest.replay_permit(&mut last_checkpoint).await.ok()
    });
    sleep(Duration::from_secs(5)).await;
    let newer_wait = tokio::spawn(async move {
        let mut last_checkpoint = Instant::now();
        newer.replay_permit(&mut last_checkpoint).await.ok()
    });
    // Both cross several heartbeats while waiting; their checkpoints keep arriving. Capacity
    // frees at 32s: the oldest last ticked at 30s, after the newer one's 25s tick, so a waiter
    // that re-queued on every heartbeat would now be behind the newer one.
    sleep(CHECKPOINT_INTERVAL * 3 - Duration::from_secs(3)).await;
    assert!(
        oldest_receiver.try_recv().is_ok(),
        "the oldest waiter sent heartbeats"
    );
    assert!(
        newer_receiver.try_recv().is_ok(),
        "the newer waiter sent heartbeats"
    );

    replaying.add_permits(1);
    let permit = timeout(Duration::from_secs(1), oldest_wait)
        .await
        .expect("the oldest waiter gets the first permit")
        .unwrap();
    assert!(permit.is_some(), "the oldest waiter gets the first permit");
    tokio::task::yield_now().await;
    assert!(
        !newer_wait.is_finished(),
        "the newer waiter is still queued"
    );
    drop(permit);
    assert!(newer_wait.await.unwrap().is_some(), "then the newer waiter");
}

#[tokio::test(start_paused = true)]
async fn should_drop_a_client_that_stops_reading_while_waiting_for_replay_capacity() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![]);
    // A full buffer the client never drains, and no replay capacity.
    let (sender, _receiver) = mpsc::channel(1);
    sender.try_send(Err(Status::ok("filler"))).unwrap();
    let mut scan = Scan {
        blocks: harness.blocks.clone(),
        tip: harness.tip.subscribe(),
        replaying: Arc::new(Semaphore::new(0)),
        filters: recipient_filter(7),
        sender,
        next_height: 1,
        deliver_from: 1,
    };
    let mut last_checkpoint = Instant::now();
    let outcome = timeout(
        SEND_DEADLINE + CHECKPOINT_INTERVAL * 3,
        scan.replay_permit(&mut last_checkpoint),
    )
    .await
    .expect("the wait ends without a permit");
    match outcome {
        Err(Stop::SlowClient) => {}
        Err(_) => panic!("expected the slow client to be dropped"),
        Ok(_) => panic!("expected no permit"),
    }
}

#[tokio::test(start_paused = true)]
async fn should_free_a_slow_clients_slot_once_its_deadline_passes() {
    let harness = harness(SubscriptionLimits {
        max_subscriptions: 1,
        ..Default::default()
    });
    // More matches than the stream buffer holds, and a client that never reads.
    for _ in 0..100 {
        harness.commit(vec![credit_transfer(7)]);
    }
    let _unread = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            Some(1),
            None,
        )
        .await
        .unwrap();
    assert!(
        harness.service.admit(None).is_err(),
        "the only slot is taken"
    );

    // Just past the slow-reader deadline, not a second one, the slot is free again.
    sleep(SEND_DEADLINE + Duration::from_secs(5)).await;
    assert!(
        harness.service.admit(None).is_ok(),
        "a slow client's slot is freed at its deadline"
    );
}

/// The `rating` contract with `stars` allowed up to `maximum`: a u8 field up to 255, a u16
/// field once an update widens it.
fn rating_contract(maximum: u64, version: u32) -> dpp::data_contract::DataContract {
    use dpp::data_contract::DataContractFactory;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::platform_value::platform_value;

    let documents = platform_value!({
        "rating": {
            "type": "object",
            "properties": {
                "stars": { "type": "integer", "minimum": 0, "maximum": maximum, "position": 0 }
            },
            "additionalProperties": false
        }
    });
    let mut contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .unwrap()
        .create_with_value_config(Identifier::from([3u8; 32]), 1, documents, None, None)
        .unwrap()
        .data_contract_owned();
    contract.set_version(version);
    contract
}

/// A stream of ratings over 3 stars, bound to the contract read at height 1 with `stars` a u8.
/// An update widening `stars` commits at 2, then `blocks_after_update` empty blocks, then the
/// stream starts `ahead` blocks after the tip (just after it when `None`). A 300-star rating at
/// the start must match: it is a rating only under the widened schema.
async fn should_follow_an_update_between_binding_and_start(
    blocks_after_update: usize,
    ahead: Option<u64>,
) {
    use dash_platform_queries::subscriptions::{DocumentAction, DocumentActionMatch, DocumentFilter};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::platform_value::Value;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
    use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::version::TryFromPlatformVersioned;
    use drive::query::{WhereClause, WhereOperator};

    let harness = harness(SubscriptionLimits::default());
    let narrow = Arc::new(rating_contract(255, 1));
    let filters = ResolvedFilters::resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(narrow.id())
                .with_document_type("rating")
                .with_action(
                    DocumentActionMatch::new(DocumentAction::Create).with_new_document_where(
                        WhereClause {
                            field: "stars".to_string(),
                            operator: WhereOperator::GreaterThan,
                            value: Value::U64(3),
                        },
                    ),
                ),
        )],
        |_| Some(narrow.clone()),
        PlatformVersion::latest(),
    )
    .unwrap();
    // The contract is read at height 1; the update widening `stars` commits at 2, before the
    // stream starts after the tip.
    let read_at = harness.commit(vec![]);
    let update = StateTransition::DataContractUpdate(
        DataContractUpdateTransition::try_from_platform_versioned(
            (rating_contract(1000, 2), 2),
            PlatformVersion::latest(),
        )
        .unwrap(),
    )
    .serialize_to_bytes()
    .unwrap();
    harness.commit(vec![update]);
    for _ in 0..blocks_after_update {
        harness.commit(vec![]);
    }
    let tip = *harness.tip.borrow();
    let start = tip + ahead.unwrap_or(1);
    let mut stream = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            filters,
            ahead.map(|_| start),
            Some(read_at),
        )
        .await
        .unwrap();
    // Nothing before the start is delivered...
    expect_checkpoint(&mut stream, start - 1).await;
    while *harness.tip.borrow() + 1 < start {
        harness.commit(vec![]);
    }

    // ...but the update was followed: 300 stars is a rating only under the widened schema.
    let rating = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
        owner_id: Identifier::from([4u8; 32]),
        transitions: vec![BatchedTransition::Document(DocumentTransition::Create(
            DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                    id: Identifier::from([5u8; 32]),
                    identity_contract_nonce: 1,
                    document_type_name: "rating".to_string(),
                    data_contract_id: narrow.id(),
                }),
                entropy: [0; 32],
                data: [("stars".to_string(), Value::U64(300))].into(),
                prefunded_voting_balance: None,
            }),
        ))],
        ..Default::default()
    }))
    .serialize_to_bytes()
    .unwrap();
    let height = harness.commit(vec![rating]);
    assert_eq!(height, start);
    expect_match(&mut stream, height, 0).await;
}

#[tokio::test]
async fn should_follow_a_contract_update_committed_while_a_live_stream_starts() {
    should_follow_an_update_between_binding_and_start(0, None).await;
}

#[tokio::test]
async fn should_follow_a_contract_update_more_than_a_page_before_a_live_start() {
    should_follow_an_update_between_binding_and_start(META_PAGE as usize, None).await;
}

#[tokio::test]
async fn should_follow_a_contract_update_before_a_start_ahead_of_the_tip() {
    should_follow_an_update_between_binding_and_start(0, Some(5)).await;
    should_follow_an_update_between_binding_and_start(0, Some(META_PAGE + 10)).await;
}

#[tokio::test]
async fn should_not_checkpoint_before_the_start_while_scanning_before_it() {
    let harness = harness(SubscriptionLimits::default());
    let read_at = harness.commit(vec![]);
    harness.commit(vec![credit_transfer(7)]);
    harness.commit(vec![credit_transfer(7)]);
    let mut stream = harness
        .service
        .start(
            harness.service.admit(None).unwrap(),
            recipient_filter(7),
            None,
            Some(read_at),
        )
        .await
        .unwrap();
    // Matches at 2 and 3 are before the start (4): scanned, not delivered.
    expect_checkpoint(&mut stream, 3).await;
    let height = harness.commit(vec![credit_transfer(7)]);
    expect_match(&mut stream, height, 0).await;
    expect_checkpoint(&mut stream, height).await;
}

#[tokio::test(start_paused = true)]
async fn should_share_a_height_not_readable_yet_between_readers() {
    let harness = harness(SubscriptionLimits::default());
    let height = harness.commit(vec![credit_transfer(7)]);
    *harness.chain.results_withheld.lock().unwrap() = Some(height);
    let reads = || harness.chain.block_reads.load(Ordering::SeqCst);

    // Subscriptions waiting on the height ask Tenderdash once between them.
    let outcomes = futures::future::join_all((0..8).map(|_| harness.blocks.read(height))).await;
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome, Ok(BlockRead::NotYet)))
    );
    assert_eq!(reads(), 1);
    assert!(matches!(
        harness.blocks.read(height).await,
        Ok(BlockRead::NotYet)
    ));
    assert_eq!(reads(), 1, "the miss is remembered for a moment");

    // After the retry delay the height is read again, and once its results are saved it is
    // a block like any other.
    sleep(NOT_YET_RETRY).await;
    assert!(matches!(
        harness.blocks.read(height).await,
        Ok(BlockRead::NotYet)
    ));
    assert_eq!(reads(), 2);
    *harness.chain.results_withheld.lock().unwrap() = None;
    sleep(NOT_YET_RETRY).await;
    assert!(matches!(
        harness.blocks.read(height).await,
        Ok(BlockRead::Block(_))
    ));
    assert_eq!(reads(), 3);
}

#[tokio::test]
async fn should_keep_a_block_with_hundreds_of_kib_of_transitions_cached() {
    let harness = harness(SubscriptionLimits::default());
    let transfer = credit_transfer(7);
    let height = harness.commit(vec![transfer.clone(); 256 * 1024 / transfer.len() + 1]);
    let reads = || harness.chain.block_reads.load(Ordering::SeqCst);

    assert!(matches!(
        harness.blocks.read(height).await,
        Ok(BlockRead::Block(_))
    ));
    assert_eq!(reads(), 1);
    // Read again after the first load completed: served from the cache.
    assert!(matches!(
        harness.blocks.read(height).await,
        Ok(BlockRead::Block(_))
    ));
    assert_eq!(reads(), 1, "the block was kept");
}
