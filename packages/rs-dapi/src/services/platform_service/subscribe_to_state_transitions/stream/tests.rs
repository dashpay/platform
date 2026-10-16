use super::*;
use crate::DapiError;
use crate::clients::tenderdash_client::{
    Block, BlockData, BlockHeader, BlockMeta, ConsensusVersion, ExecTxResult, ResultBlock,
    ResultBlockResults, ResultBlockchainInfo,
};
use crate::services::platform_service::subscribe_to_state_transitions::TenderdashBlocks;
use crate::services::platform_service::subscribe_to_state_transitions::block_source::MAX_CONCURRENT_FETCHES;
use async_trait::async_trait;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use dash_platform_queries::subscriptions::{Role, StateTransitionFilter};
use dpp::address_funds::PlatformAddress;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::StateTransition;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use futures::FutureExt;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_stream::StreamExt;

/// A chain held in memory: height → successful transactions.
#[derive(Default)]
struct FakeChain {
    blocks: Mutex<BTreeMap<u64, Vec<Vec<u8>>>>,
    /// A height whose execution results are not saved yet.
    results_withheld: Mutex<Option<u64>>,
    /// RPCs of any kind in flight, the most seen at once, and the `block` calls made.
    reads_in_flight: AtomicUsize,
    max_reads_in_flight: AtomicUsize,
    block_reads: AtomicUsize,
    /// Held by a test to keep `block` calls waiting before they read.
    read_gate: tokio::sync::Mutex<()>,
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

    /// Counts an RPC in flight, letting other calls start meanwhile.
    async fn in_flight(&self) {
        let now = self.reads_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_reads_in_flight.fetch_max(now, Ordering::SeqCst);
        tokio::task::yield_now().await;
        self.reads_in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

#[async_trait]
impl TenderdashBlocks for FakeChain {
    async fn latest_height(&self) -> DAPIResult<u64> {
        self.in_flight().await;
        Ok(self.blocks.lock().unwrap().len() as u64)
    }

    async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
        drop(self.read_gate.lock().await);
        self.block_reads.fetch_add(1, Ordering::SeqCst);
        self.in_flight().await;
        Ok(ResultBlock {
            block: self.txs(height).map(|txs| Block {
                header: BlockHeader {
                    version: ConsensusVersion {
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
        self.in_flight().await;
        let withheld = *self.results_withheld.lock().unwrap() == Some(height);
        let txs = self
            .txs(height)
            .filter(|_| !withheld)
            .ok_or_else(|| DapiError::Client("no results for height".to_string()))?;
        Ok(ResultBlockResults {
            txs_results: txs.iter().map(|_| ExecTxResult::default()).collect(),
        })
    }

    async fn blockchain(
        &self,
        min_height: u64,
        max_height: u64,
    ) -> DAPIResult<ResultBlockchainInfo> {
        self.in_flight().await;
        let blocks = self.blocks.lock().unwrap();
        let max_height = max_height.min(blocks.len() as u64);
        let min_height = min_height.max(max_height.saturating_sub(19)).max(1);
        Ok(ResultBlockchainInfo {
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

fn address(byte: u8) -> PlatformAddress {
    PlatformAddress::P2pkh([byte; 20])
}

/// A serialized transfer from address 1 to address `recipient`.
fn transfer_to(recipient: u8) -> Vec<u8> {
    StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
        AddressFundsTransferTransitionV0 {
            inputs: BTreeMap::from([(address(1), (1, 5000))]),
            outputs: BTreeMap::from([(address(recipient), 4000)]),
            ..Default::default()
        },
    ))
    .serialize_to_bytes()
    .unwrap()
}

fn recipient_filter(recipient: u8) -> ResolvedFilters {
    ResolvedFilters::resolve(
        vec![StateTransitionFilter::Addresses {
            addresses: vec![address(recipient)],
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

    async fn subscribe(&self, recipient: u8, from: Option<u64>) -> SubscriptionStream {
        self.service
            .start(
                self.service.admit(None).unwrap(),
                recipient_filter(recipient),
                from,
            )
            .await
            .unwrap()
    }

    /// A subscription from `source` to transfers to 7 from height 1, or its refusal.
    async fn subscribe_from(
        &self,
        source: Option<SourceKey>,
    ) -> Result<SubscriptionStream, Status> {
        let admission = self.service.admit(source)?;
        self.service
            .start(admission, recipient_filter(7), Some(1))
            .await
    }
}

enum Message {
    Match { height: u64, tx: Vec<u8> },
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
            tx: matched.state_transition.to_vec(),
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

async fn expect_match(stream: &mut SubscriptionStream, height: u64, expected: &[u8]) {
    match next(stream).await {
        Message::Match { height: at, tx } => {
            assert_eq!(at, height, "match height");
            assert_eq!(tx, expected, "matched transition");
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
            3 => vec![transfer_to(9), transfer_to(7)],
            30 => vec![transfer_to(7)],
            h if h % 2 == 0 => vec![transfer_to(9)],
            _ => vec![],
        };
        harness.commit(txs);
    }
    let mut stream = harness.subscribe(7, Some(2)).await;

    expect_checkpoint(&mut stream, 1).await;
    expect_match(&mut stream, 3, &transfer_to(7)).await;
    expect_checkpoint(&mut stream, 3).await;
    expect_match(&mut stream, 30, &transfer_to(7)).await;
    expect_checkpoint(&mut stream, 30).await;

    let height = harness.commit(vec![transfer_to(9), transfer_to(7)]);
    expect_match(&mut stream, height, &transfer_to(7)).await;
    expect_checkpoint(&mut stream, height).await;
}

#[tokio::test]
async fn should_start_after_the_tip_when_no_height_is_given() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![transfer_to(7)]);
    let mut stream = harness.subscribe(7, None).await;
    expect_checkpoint(&mut stream, 1).await;
    let height = harness.commit(vec![transfer_to(7)]);
    expect_match(&mut stream, height, &transfer_to(7)).await;
}

#[tokio::test]
async fn should_checkpoint_just_before_a_start_ahead_of_the_tip() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![]);
    let mut stream = harness.subscribe(7, Some(5)).await;
    expect_checkpoint(&mut stream, 4).await;
    // Matches before the start are not delivered.
    for _ in 2..5 {
        harness.commit(vec![transfer_to(7)]);
    }
    let height = harness.commit(vec![transfer_to(7)]);
    expect_match(&mut stream, height, &transfer_to(7)).await;
    expect_checkpoint(&mut stream, height).await;
}

#[tokio::test]
async fn should_send_a_match_sharing_the_cached_transaction_bytes() {
    let harness = harness(SubscriptionLimits::default());
    let height = harness.commit(vec![transfer_to(7)]);
    let mut stream = harness.subscribe(7, Some(height)).await;
    expect_checkpoint(&mut stream, height - 1).await;
    let response = stream.next().await.unwrap().unwrap();
    let Some(Version::V0(SubscribeToStateTransitionsResponseV0 {
        responses: Some(Responses::StateTransition(matched)),
    })) = response.version
    else {
        panic!("expected a match");
    };
    let block = harness.blocks.read(height).await.unwrap();
    assert_eq!(
        matched.state_transition.as_ptr(),
        block.txs[0].bytes.as_ptr(),
        "the response shares the cached bytes"
    );
}

#[tokio::test(start_paused = true)]
async fn should_send_checkpoints_while_nothing_matches() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![]);
    let mut stream = harness.subscribe(7, None).await;
    expect_checkpoint(&mut stream, 1).await;
    harness.commit(vec![transfer_to(9)]);
    expect_checkpoint(&mut stream, 2).await;
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
    for (from, code) in [(5, tonic::Code::OutOfRange), (40, tonic::Code::Unavailable)] {
        let error = harness
            .service
            .start(
                harness.service.admit(None).unwrap(),
                recipient_filter(7),
                Some(from),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code(), code, "from {from}");
    }
}

#[tokio::test]
async fn should_limit_subscriptions_per_source() {
    let harness = harness(SubscriptionLimits {
        max_subscriptions_per_source: 2,
        ..Default::default()
    });
    let source = |ip: &str| Some(SourceKey::from(ip.parse::<IpAddr>().unwrap()));
    let first = harness.service.admit(source("203.0.113.7")).unwrap();
    let _second = harness.service.admit(source("203.0.113.7")).unwrap();
    assert_eq!(
        harness
            .service
            .admit(source("203.0.113.7"))
            .err()
            .unwrap()
            .code(),
        tonic::Code::ResourceExhausted
    );
    // Another source is not affected; a released slot can be reused.
    assert!(harness.service.admit(source("203.0.113.8")).is_ok());
    drop(first);
    assert!(harness.service.admit(source("203.0.113.7")).is_ok());
}

#[tokio::test]
async fn should_end_the_stream_on_a_transaction_this_node_cannot_decode() {
    let harness = harness(SubscriptionLimits::default());
    harness.commit(vec![vec![0xff, 0x00, 0x13]]);
    let mut stream = harness.subscribe(7, Some(1)).await;
    expect_checkpoint(&mut stream, 0).await;
    let status = timeout(Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);
}

fn buffered(stream: &SubscriptionStream) -> usize {
    stream.buffer.lock().unwrap().messages.len()
}

#[tokio::test(start_paused = true)]
async fn should_free_a_slow_clients_slot_and_buffer_and_end_its_stream() {
    let harness = harness(SubscriptionLimits {
        max_subscriptions: 1,
        ..Default::default()
    });
    // More matches than the stream buffer holds, and a client that stops reading.
    for _ in 0..100 {
        harness.commit(vec![transfer_to(7)]);
    }
    let mut stream = harness.subscribe(7, Some(1)).await;
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

    // What it left buffered is discarded: it resumes from its last received checkpoint.
    let status = stream
        .next()
        .await
        .expect("the stream ends with a status")
        .expect_err("no buffered message is kept");
    assert_eq!(status.code(), tonic::Code::ResourceExhausted);
    assert!(stream.next().await.is_none(), "the status is the last item");
}

#[tokio::test(start_paused = true)]
async fn should_drop_the_buffers_of_stalled_streams_while_replacements_are_admitted() {
    let harness = harness(SubscriptionLimits {
        max_subscriptions: 3,
        ..Default::default()
    });
    for _ in 0..100 {
        harness.commit(vec![transfer_to(7)]);
    }
    // Batches of stalled streams, each opened once the previous batch timed out, all kept open.
    let mut stalled = Vec::new();
    for _ in 0..3 {
        let mut batch = Vec::new();
        for _ in 0..3 {
            batch.push(harness.subscribe(7, Some(1)).await);
        }
        sleep(Duration::from_secs(1)).await;
        for stream in &batch {
            assert_eq!(buffered(stream), STREAM_BUFFER, "the buffer is full");
        }
        sleep(SEND_DEADLINE + Duration::from_secs(5)).await;
        stalled.extend(batch);
        for stream in &stalled {
            assert_eq!(buffered(stream), 0, "a stalled stream keeps no messages");
        }
    }
    for mut stream in stalled {
        let status = stream.next().await.unwrap().unwrap_err();
        assert_eq!(status.code(), tonic::Code::ResourceExhausted);
    }
}

#[tokio::test(start_paused = true)]
async fn should_hold_a_failed_streams_slot_until_its_buffer_is_read_or_discarded() {
    let harness = harness(SubscriptionLimits {
        max_subscriptions_per_source: 2,
        ..Default::default()
    });
    // A match, then a height this node cannot decode, which ends every scan.
    harness.commit(vec![transfer_to(7)]);
    harness.commit(vec![vec![0xff, 0x00, 0x13]]);
    let source = Some(SourceKey::from("203.0.113.7".parse::<IpAddr>().unwrap()));
    let subscribe = || harness.subscribe_from(source);

    // Failed streams nobody reads keep their slots until the deadline, then lose their buffers.
    let stalled = [subscribe().await.unwrap(), subscribe().await.unwrap()];
    sleep(Duration::from_secs(1)).await;
    for stream in &stalled {
        assert_eq!(buffered(stream), 3, "checkpoint, match, checkpoint");
    }
    assert_eq!(
        harness.service.admit(source).err().unwrap().code(),
        tonic::Code::ResourceExhausted,
        "a failed stream keeps its slot while its buffer may still be read"
    );
    sleep(SEND_DEADLINE).await;
    for stream in &stalled {
        assert_eq!(buffered(stream), 0, "an unread buffer is discarded");
    }
    for mut stream in stalled {
        let status = stream.next().await.unwrap().unwrap_err();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    }

    // A client that reads gets what was buffered, then the failure, then its slot back.
    let mut readers = [subscribe().await.unwrap(), subscribe().await.unwrap()];
    for stream in &mut readers {
        expect_checkpoint(stream, 0).await;
        expect_match(stream, 1, &transfer_to(7)).await;
        expect_checkpoint(stream, 1).await;
        let status = stream.next().await.unwrap().unwrap_err();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
        assert!(stream.next().await.is_none(), "the status is the last item");
    }
    assert!(subscribe().await.is_ok(), "the readers' slots are free");
}

#[tokio::test]
async fn should_start_a_live_subscription_below_a_tip_whose_results_are_not_saved() {
    let harness = harness(SubscriptionLimits::default());
    // Stored but not published: the tip watch is still 0.
    harness.chain.push(vec![]);
    let withheld = harness.chain.push(vec![transfer_to(7)]);
    *harness.chain.results_withheld.lock().unwrap() = Some(withheld);
    let mut stream = harness.subscribe(7, None).await;
    expect_checkpoint(&mut stream, withheld - 1).await;

    *harness.chain.results_withheld.lock().unwrap() = None;
    harness.tip.send_replace(withheld);
    expect_match(&mut stream, withheld, &transfer_to(7)).await;
    expect_checkpoint(&mut stream, withheld).await;
}

#[tokio::test(start_paused = true)]
async fn should_not_pin_decoded_blocks_while_waiting_on_a_stalled_reader() {
    let harness = harness(SubscriptionLimits::default());
    // Several matches a block, many blocks: far more than the stream buffer holds.
    for _ in 0..40 {
        harness.commit(vec![transfer_to(7), transfer_to(7), transfer_to(7)]);
    }
    let _stalled = harness.subscribe(7, Some(1)).await;
    // On paused time this returns only once the scan is blocked on the full buffer.
    sleep(Duration::from_secs(1)).await;

    for height in 1..=40 {
        let block = harness.blocks.read(height).await.unwrap();
        // Held by the cache and by this test only, never by the stalled scan.
        assert!(
            Arc::strong_count(&block) <= 2,
            "block {height} is pinned by {} owners",
            Arc::strong_count(&block)
        );
    }
}

#[tokio::test(start_paused = true)]
async fn should_free_the_slots_of_dropped_streams_whose_reads_are_queued() {
    let harness = harness(SubscriptionLimits {
        max_subscriptions: 4,
        ..Default::default()
    });
    for _ in 0..25 {
        harness.commit(vec![transfer_to(7)]);
    }
    // Heights 1-20 have their metas cached; 21-25 do not.
    harness.blocks.meta_page(1, 25).await.unwrap();
    // Every fetch permit is taken by a read held at the gate.
    let gate = harness.chain.read_gate.lock().await;
    let held: Vec<_> = (1..=MAX_CONCURRENT_FETCHES as u64)
        .map(|height| {
            let blocks = harness.blocks.clone();
            tokio::spawn(async move { blocks.read(height).await })
        })
        .collect();
    sleep(Duration::from_secs(1)).await;

    // Two scans queue for block 9, two for the metas of 21-25.
    let mut streams = Vec::new();
    for from in [9, 9, 21, 21] {
        streams.push(harness.subscribe(7, Some(from)).await);
    }
    sleep(Duration::from_secs(1)).await;
    assert!(harness.service.admit(None).is_err(), "every slot is taken");

    drop(streams);
    sleep(Duration::from_secs(1)).await;
    let admissions: Vec<_> = (0..4).map(|_| harness.service.admit(None)).collect();
    assert!(
        admissions.iter().all(Result::is_ok),
        "dropped streams free their slots while their reads are queued"
    );
    assert_eq!(harness.chain.block_reads.load(Ordering::SeqCst), 0);

    drop(gate);
    for read in held {
        read.await.unwrap().unwrap();
    }
    sleep(Duration::from_secs(1)).await;
    assert_eq!(
        harness.chain.block_reads.load(Ordering::SeqCst),
        MAX_CONCURRENT_FETCHES,
        "no read runs for a dropped stream"
    );
}

#[tokio::test]
async fn should_bound_concurrent_tenderdash_fetches() {
    let harness = harness(SubscriptionLimits::default());
    for _ in 0..MAX_CONCURRENT_FETCHES * 3 {
        harness.commit(vec![transfer_to(7)]);
    }
    let tip = harness.chain.blocks.lock().unwrap().len() as u64;
    // Block reads, readiness probes and metas pages all share the budget.
    let reads = (1..=tip).map(|height| harness.blocks.read(height).map(|read| read.map(drop)));
    let probes = (1..=tip).map(|height| harness.blocks.results_saved(height).boxed());
    let tips = (1..=tip).map(|_| {
        harness
            .blocks
            .latest_height()
            .map(|tip| tip.map(drop))
            .boxed()
    });
    let pages = (1..=tip).step_by(20).map(|height| {
        harness
            .blocks
            .meta_page(height, tip)
            .map(|page| page.map(drop))
            .boxed()
    });
    let calls = reads
        .map(|read| read.boxed())
        .chain(probes)
        .chain(tips)
        .chain(pages);
    for call in futures::future::join_all(calls).await {
        call.unwrap();
    }
    assert_eq!(
        harness.chain.max_reads_in_flight.load(Ordering::SeqCst),
        MAX_CONCURRENT_FETCHES
    );
}

#[tokio::test(start_paused = true)]
async fn should_publish_a_tip_only_once_its_results_are_saved() {
    let chain = Arc::new(FakeChain::default());
    chain.push(vec![]);
    let withheld = chain.push(vec![transfer_to(7)]);
    *chain.results_withheld.lock().unwrap() = Some(withheld);
    let (tip, mut tip_receiver) = watch::channel(0);
    let (events, new_blocks) = broadcast::channel(4);
    tokio::spawn(SubscriptionService::track_tip(
        Arc::new(BlockSource::new(chain.clone())),
        tip,
        new_blocks,
    ));

    sleep(Duration::from_secs(1)).await;
    assert_eq!(
        *tip_receiver.borrow_and_update(),
        withheld - 1,
        "height 2 has no results yet; the height before it does"
    );

    // A new-block event publishes it as soon as its results are saved.
    *chain.results_withheld.lock().unwrap() = None;
    events.send(()).unwrap();
    timeout(Duration::from_secs(1), tip_receiver.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(*tip_receiver.borrow_and_update(), withheld);

    // Blocks faster than their results: each event still advances the tip to the height
    // before the newest.
    for _ in 0..3 {
        chain.push(vec![]);
        let newest = chain.push(vec![]);
        *chain.results_withheld.lock().unwrap() = Some(newest);
        events.send(()).unwrap();
        timeout(Duration::from_secs(1), tip_receiver.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*tip_receiver.borrow_and_update(), newest - 1);
    }

    // Without events, the poll still advances the tip.
    *chain.results_withheld.lock().unwrap() = None;
    drop(events);
    sleep(Duration::from_secs(1)).await;
    tip_receiver.borrow_and_update();
    let height = chain.push(vec![]);
    timeout(TIP_POLL_INTERVAL * 2, tip_receiver.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(*tip_receiver.borrow(), height);
}

#[tokio::test]
async fn should_keep_a_two_mib_block_cached() {
    let harness = harness(SubscriptionLimits::default());
    let height = harness.commit(vec![vec![0u8; 1024]; 2048]);
    let reads = || harness.chain.block_reads.load(Ordering::SeqCst);

    harness.blocks.read(height).await.unwrap();
    assert_eq!(reads(), 1);
    // Read again after the first load completed: served from the cache.
    harness.blocks.read(height).await.unwrap();
    assert_eq!(reads(), 1, "the block was kept");
}
