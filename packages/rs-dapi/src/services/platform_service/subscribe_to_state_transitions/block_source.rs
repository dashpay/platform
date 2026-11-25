//! Committed blocks, read from the Tenderdash block store and shared by every subscription:
//! concurrent reads of one height (or metas page) share one Tenderdash call and one decode.

use crate::DapiError;
use crate::clients::tenderdash_client::{
    BlockMeta, ResultBlock, ResultBlockResults, ResultBlockchainInfo, TenderdashClient,
};
use crate::error::DAPIResult;
use async_trait::async_trait;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use dash_platform_queries::subscriptions::decode_committed_state_transition;
use dpp::platform_value::Value;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use quick_cache::sync::{Cache, DefaultLifecycle};
use quick_cache::{DefaultHashBuilder, OptionsBuilder, Weighter};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

/// Heights per `blockchain` page; Tenderdash's own cap.
pub const META_PAGE: u64 = 20;
/// Bytes of decoded blocks kept, as weighed by [`BlockWeighter`]: a 2 MiB block weighs
/// 2 MiB × (1 + 2 × `size_of::<Value>()`, 48) = 194 MiB, so two fit.
const BLOCK_CACHE_BYTES: u64 = 512 * 1024 * 1024;
/// Weight of a cached transaction beyond its decoded values: its hash, its entry, its `Bytes`.
const TX_OVERHEAD_BYTES: usize = 256;
/// One shard: quick_cache only keeps an entry that fits a shard, and a 2 MiB block must fit.
const BLOCK_CACHE_SHARDS: usize = 1;
/// Block metas pages kept.
const META_PAGE_CACHE_PAGES: usize = 256;
/// Tenderdash RPCs in flight at once, across all subscriptions and the tip tracker. A block
/// being decoded counts as the two calls that read it.
pub const MAX_CONCURRENT_FETCHES: usize = 8;
/// How long a subscription waits for a read, from when it asks: for its turn to match a
/// block, for another subscription reading the same block, for the fetch budget and for
/// Tenderdash together. None of the calls made here has a deadline of its own: Tenderdash
/// reads its store to answer whoever still waits, and the block queries that share the
/// calls wait for as long as it works. A stream that waited forever would
/// stall without an error, so it stops waiting, ends with `UNAVAILABLE`, and the client
/// resumes on another node. The read itself goes on, and keeps its share of
/// [`MAX_CONCURRENT_FETCHES`] until it ends.
pub(super) const BLOCK_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// The Tenderdash RPCs the block source reads.
#[async_trait]
pub trait TenderdashBlocks: Send + Sync + 'static {
    async fn latest_height(&self) -> DAPIResult<u64>;
    /// The lowest height the store still keeps.
    async fn earliest_height(&self) -> DAPIResult<u64>;
    async fn block(&self, height: u64) -> DAPIResult<ResultBlock>;
    async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults>;
    async fn blockchain(
        &self,
        min_height: u64,
        max_height: u64,
    ) -> DAPIResult<ResultBlockchainInfo>;
}

#[async_trait]
impl TenderdashBlocks for TenderdashClient {
    async fn latest_height(&self) -> DAPIResult<u64> {
        let status = self.status_for_block_query().await?;
        Ok(status.sync_info.latest_block_height.max(0) as u64)
    }

    async fn earliest_height(&self) -> DAPIResult<u64> {
        let status = self.status_for_block_query().await?;
        Ok(status.sync_info.earliest_block_height.max(0) as u64)
    }

    async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
        TenderdashClient::block(self, height).await
    }

    async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults> {
        TenderdashClient::block_results(self, height).await
    }

    async fn blockchain(
        &self,
        min_height: u64,
        max_height: u64,
    ) -> DAPIResult<ResultBlockchainInfo> {
        TenderdashClient::blockchain(self, min_height, max_height).await
    }
}

/// One successfully executed transaction of a committed block.
#[derive(Debug, Clone)]
pub struct CommittedTx {
    /// Position among the block's transactions.
    pub index: u32,
    /// SHA-256 of `bytes`, the Tenderdash transaction hash.
    pub hash: [u8; 32],
    /// The serialized state transition, shared with every response that carries it.
    pub bytes: Bytes,
    /// The decoded transition, or why this build cannot decode it.
    pub state_transition: Result<StateTransition, String>,
}

/// A committed block's header facts and its successfully executed transactions.
#[derive(Debug, Clone)]
pub struct CommittedBlock {
    pub height: u64,
    pub time_ms: u64,
    pub protocol_version: u32,
    pub txs: Vec<CommittedTx>,
}

/// One height's block meta, as far as the subscription needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeightMeta {
    /// The block has no transactions.
    Empty,
    /// The block has transactions.
    HasTxs,
    /// The store keeps no transactions for this height.
    Unavailable,
}

#[derive(Clone)]
struct BlockWeighter;

impl Weighter<u64, Arc<CommittedBlock>> for BlockWeighter {
    fn weight(&self, _height: &u64, block: &Arc<CommittedBlock>) -> u64 {
        // The serialized bytes, and at most one decoded `Value` per serialized byte: every
        // value takes at least one. Doubled: decoded arrays grow by doubling and keep the slack.
        let txs: usize = block
            .txs
            .iter()
            .map(|tx| tx.bytes.len() * (1 + 2 * size_of::<Value>()) + TX_OVERHEAD_BYTES)
            .sum();
        (txs + TX_OVERHEAD_BYTES) as u64
    }
}

/// Reads committed blocks for every subscription, sharing what was read.
pub struct BlockSource {
    tenderdash: Arc<dyn TenderdashBlocks>,
    blocks: Cache<u64, Arc<CommittedBlock>, BlockWeighter>,
    /// Metas pages by their exact `(start, end)` range; committed metas never change.
    meta_pages: Cache<(u64, u64), Arc<Vec<HeightMeta>>>,
    /// One permit per Tenderdash RPC, held by the task that makes the call until Tenderdash
    /// has answered it, whoever still waits for the answer. A slow client cannot hold one.
    fetches: Arc<Semaphore>,
}

impl BlockSource {
    pub fn new(tenderdash: Arc<dyn TenderdashBlocks>) -> Self {
        Self {
            tenderdash,
            blocks: Cache::with_options(
                OptionsBuilder::new()
                    .estimated_items_capacity(4096)
                    .weight_capacity(BLOCK_CACHE_BYTES)
                    .shards(BLOCK_CACHE_SHARDS)
                    .build()
                    .expect("the block cache options are valid"),
                BlockWeighter,
                DefaultHashBuilder::default(),
                DefaultLifecycle::default(),
            ),
            meta_pages: Cache::new(META_PAGE_CACHE_PAGES),
            fetches: Arc::new(Semaphore::new(MAX_CONCURRENT_FETCHES)),
        }
    }

    /// The highest height Tenderdash reports stored. Its results may not be saved yet.
    pub async fn latest_height(&self) -> DAPIResult<u64> {
        let asked = self.dispatch(
            1,
            |tenderdash| async move { tenderdash.latest_height().await },
        );
        within_read_deadline(asked).await
    }

    /// Succeeds once the execution results of `height` are saved, so the height is readable.
    pub async fn results_saved(&self, height: u64) -> DAPIResult<()> {
        let asked = self.dispatch(1, move |tenderdash| async move {
            tenderdash.block_results(height).await.map(drop)
        });
        within_read_deadline(asked).await
    }

    /// The metas of the aligned page holding `height` (heights `page_start..page_start + 20`),
    /// up to `tip`.
    pub async fn meta_page(
        &self,
        height: u64,
        tip: u64,
    ) -> DAPIResult<(u64, Arc<Vec<HeightMeta>>)> {
        let page_start = (height.saturating_sub(1) / META_PAGE) * META_PAGE + 1;
        let page_end = (page_start + META_PAGE - 1).min(tip);
        let page = (page_start, page_end);
        let stored = self.meta_pages.get_or_insert_async(&page, async {
            let metas = self
                .dispatch(1, move |tenderdash| {
                    stored_metas(tenderdash, page_start, page_end)
                })
                .await?;
            Ok::<_, DapiError>(Arc::new(page_from_metas(page_start, page_end, &metas)?))
        });
        Ok((page_start, within_read_deadline(stored).await?))
    }

    /// Read the committed block at `height`, which must be at or below the published tip.
    ///
    /// Readers of one height share one read: the first to ask makes it and the others wait for
    /// it, each for as long as its own deadline allows.
    pub async fn read(&self, height: u64) -> DAPIResult<Arc<CommittedBlock>> {
        let stored = self.blocks.get_or_insert_async(&height, self.fetch(height));
        within_read_deadline(stored).await
    }

    /// Ask Tenderdash for something that takes `calls` of the fetch budget, in a task of its
    /// own.
    ///
    /// A caller that goes away, or stops waiting, while it queues for the budget has asked
    /// Tenderdash for nothing. Once asked, Tenderdash works on the request whoever still
    /// waits for it, so the task keeps the budget until the request ends: a caller that
    /// stops waiting, or goes away, does not free it for more work.
    async fn dispatch<T, F>(
        &self,
        calls: u32,
        ask: impl FnOnce(Arc<dyn TenderdashBlocks>) -> F,
    ) -> DAPIResult<T>
    where
        T: Send + 'static,
        F: Future<Output = DAPIResult<T>> + Send + 'static,
    {
        let permit = Arc::clone(&self.fetches)
            .acquire_many_owned(calls)
            .await
            .map_err(|_| DapiError::Internal("the block source is closed".to_string()))?;
        let asked = ask(Arc::clone(&self.tenderdash));
        tokio::spawn(async move {
            let _permit = permit;
            asked.await
        })
        .await?
    }

    async fn fetch(&self, height: u64) -> DAPIResult<Arc<CommittedBlock>> {
        self.dispatch(2, move |tenderdash| async move {
            let read = async {
                let (block, results) =
                    tokio::join!(tenderdash.block(height), tenderdash.block_results(height));
                let (block, results) = (block?, results?);
                // Decoding and hashing every transaction of a full block would hold up the
                // other requests an async thread serves, so it runs on the blocking pool.
                tokio::task::spawn_blocking(move || committed_block(height, block, results)).await?
            };
            match read.await {
                Ok(block) => Ok(block),
                Err(failure) => Err(pruned_or(failure, tenderdash.as_ref(), height).await),
            }
        })
        .await
    }
}

/// `read`, for as long as a subscription waits for one; see [`BLOCK_READ_TIMEOUT`].
pub(super) async fn within_read_deadline<T>(
    read: impl Future<Output = DAPIResult<T>>,
) -> DAPIResult<T> {
    timeout(BLOCK_READ_TIMEOUT, read).await.unwrap_or_else(|_| {
        Err(DapiError::Timeout(format!(
            "Tenderdash did not answer in {BLOCK_READ_TIMEOUT:?}"
        )))
    })
}

/// What the failed read of `height` says: that the store no longer keeps the height, when it
/// does not, and `failure` otherwise. A block can be pruned after its metas were read, and
/// Tenderdash refuses to read one it pruned as it refuses anything else.
async fn pruned_or(
    failure: DapiError,
    tenderdash: &dyn TenderdashBlocks,
    height: u64,
) -> DapiError {
    match tenderdash.earliest_height().await {
        Ok(earliest) if height < earliest => {
            DapiError::FailedPrecondition(format!("block {height} is not available on this node"))
        }
        _ => failure,
    }
}

/// The metas Tenderdash stores for heights `page_start..=page_end`; none when the whole range
/// lies below its store's base. Tenderdash leaves out the heights of a range that are below
/// the base, but refuses a range that is wholly below it, as it refuses a malformed one.
async fn stored_metas(
    tenderdash: Arc<dyn TenderdashBlocks>,
    page_start: u64,
    page_end: u64,
) -> DAPIResult<Vec<BlockMeta>> {
    match tenderdash.blockchain(page_start, page_end).await {
        Ok(info) => Ok(info.block_metas),
        Err(refusal) => match tenderdash.earliest_height().await {
            Ok(earliest) if page_end < earliest => Ok(Vec::new()),
            _ => Err(refusal),
        },
    }
}

/// The committed block Tenderdash's answers for `height` describe: its successfully executed
/// transactions, hashed and decoded.
fn committed_block(
    height: u64,
    block: ResultBlock,
    results: ResultBlockResults,
) -> DAPIResult<Arc<CommittedBlock>> {
    let block = block
        .block
        .ok_or_else(|| DapiError::Internal(format!("block {height} is not stored")))?;
    if block.data.txs.len() != results.txs_results.len() {
        return Err(DapiError::Internal(format!(
            "block {height} has {} transactions but {} execution results",
            block.data.txs.len(),
            results.txs_results.len()
        )));
    }

    let protocol_version = u32::try_from(block.header.version.app).map_err(|_| {
        DapiError::Internal(format!(
            "block {height} has protocol version {} out of range",
            block.header.version.app
        ))
    })?;
    // Transactions decode under the rules of the version the block executed with, as
    // consensus decoded them (older versions put no depth limit on document values).
    let platform_version = PlatformVersion::get(protocol_version).ok();
    let mut txs = Vec::new();
    for (index, (tx, result)) in block.data.txs.iter().zip(&results.txs_results).enumerate() {
        if result.code != 0 {
            continue;
        }
        let bytes = BASE64_STANDARD.decode(tx).map_err(|e| {
            DapiError::Internal(format!(
                "block {height} transaction {index} is not valid base64: {e}"
            ))
        })?;
        txs.push(CommittedTx {
            index: index as u32,
            hash: Sha256::digest(&bytes).into(),
            state_transition: decode_state_transition(&bytes, platform_version),
            bytes: Bytes::from(bytes),
        });
    }
    let time_ms = parse_block_time_ms(&block.header.time).ok_or_else(|| {
        DapiError::Internal(format!(
            "block {height} has an unreadable time '{}'",
            block.header.time
        ))
    })?;
    Ok(Arc::new(CommittedBlock {
        height,
        time_ms,
        protocol_version,
        txs,
    }))
}

/// A committed transaction decoded as the block's protocol version decoded it; `None` for a
/// version this build does not know, whose block the scan refuses anyway.
fn decode_state_transition(
    bytes: &[u8],
    platform_version: Option<&PlatformVersion>,
) -> Result<StateTransition, String> {
    let platform_version =
        platform_version.ok_or_else(|| "unknown protocol version".to_string())?;
    decode_committed_state_transition(bytes, platform_version).map_err(|e| e.to_string())
}

/// The page `[page_start, page_end]` from the metas Tenderdash returned, which must cover
/// exactly those heights. Heights below the store's base come back missing: they are
/// unavailable, as are header-only metas (`num_txs` < 0, from state sync).
fn page_from_metas(
    page_start: u64,
    page_end: u64,
    metas: &[BlockMeta],
) -> DAPIResult<Vec<HeightMeta>> {
    let mut page = vec![HeightMeta::Unavailable; (page_end - page_start + 1) as usize];
    let mut lowest_returned = None;
    for meta in metas {
        let height = meta.header.height;
        if !(page_start..=page_end).contains(&height) {
            return Err(DapiError::Internal(format!(
                "Tenderdash returned block meta {height} outside of {page_start}..={page_end}"
            )));
        }
        lowest_returned = Some(lowest_returned.map_or(height, |lowest: u64| lowest.min(height)));
        page[(height - page_start) as usize] = match meta.num_txs {
            n if n < 0 => HeightMeta::Unavailable,
            0 => HeightMeta::Empty,
            _ => HeightMeta::HasTxs,
        };
    }
    // Every height from the lowest returned up to the page end must be there: Tenderdash only
    // omits heights below its base.
    if let Some(lowest) = lowest_returned {
        let returned = metas.len() as u64;
        if returned != page_end - lowest + 1 {
            return Err(DapiError::Internal(format!(
                "Tenderdash returned {returned} block metas for heights {lowest}..={page_end}"
            )));
        }
    }
    Ok(page)
}

/// A Tenderdash RFC 3339 block time in milliseconds since the Unix epoch.
fn parse_block_time_ms(time: &str) -> Option<u64> {
    let time = chrono::DateTime::parse_from_rfc3339(time).ok()?;
    u64::try_from(time.timestamp_millis()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tenderdash_client::{
        Block, BlockData, BlockHeader, BlockTxResult, ConsensusVersion,
    };
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
    use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
    use tokio::time::Instant;

    fn meta(height: u64, num_txs: i64) -> BlockMeta {
        BlockMeta {
            header: BlockHeader {
                height,
                ..Default::default()
            },
            num_txs,
        }
    }

    #[test]
    fn should_accept_a_complete_page_and_mark_empty_heights() {
        let metas: Vec<_> = (1..=20).rev().map(|h| meta(h, (h % 2) as i64)).collect();
        let page = page_from_metas(1, 20, &metas).unwrap();
        assert_eq!(page[0], HeightMeta::HasTxs);
        assert_eq!(page[1], HeightMeta::Empty);
    }

    #[test]
    fn should_treat_heights_below_the_store_base_and_header_only_metas_as_unavailable() {
        let metas = vec![meta(40, 1), meta(39, -1), meta(38, 0)];
        let page = page_from_metas(21, 40, &metas).unwrap();
        // Heights 21..=37 are below the store's base.
        assert_eq!(page[0], HeightMeta::Unavailable);
        assert_eq!(page[16], HeightMeta::Unavailable);
        assert_eq!(page[17], HeightMeta::Empty);
        // Height 39 keeps only its header.
        assert_eq!(page[18], HeightMeta::Unavailable);
        assert_eq!(page[19], HeightMeta::HasTxs);
    }

    #[test]
    fn should_reject_a_page_with_a_gap_above_the_base() {
        let metas = vec![meta(40, 1), meta(38, 0)];
        assert!(page_from_metas(21, 40, &metas).is_err());
    }

    #[test]
    fn should_reject_metas_outside_the_page() {
        assert!(page_from_metas(1, 20, &[meta(21, 0)]).is_err());
    }

    #[test]
    fn should_parse_tenderdash_block_times() {
        assert_eq!(
            parse_block_time_ms("2026-10-05T12:00:00.123456789Z"),
            Some(1_791_201_600_123)
        );
        assert_eq!(parse_block_time_ms("not a time"), None);
    }

    /// A chain of one block. `base` is the lowest height its store keeps, and `held` keeps
    /// every read of the block waiting while a test holds it.
    #[derive(Default)]
    struct OneBlock {
        txs: Vec<String>,
        results: Option<Vec<BlockTxResult>>,
        base: u64,
        /// Whether Tenderdash fails every request for metas.
        metas_fail: bool,
        held: tokio::sync::Mutex<()>,
    }

    #[async_trait]
    impl TenderdashBlocks for OneBlock {
        async fn latest_height(&self) -> DAPIResult<u64> {
            Ok(1)
        }
        async fn earliest_height(&self) -> DAPIResult<u64> {
            Ok(self.base)
        }
        async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
            drop(self.held.lock().await);
            // As Tenderdash refuses to read a height below its base.
            if height < self.base {
                return Err(DapiError::Client(format!(
                    "height {height} is not available, lowest height is {}",
                    self.base
                )));
            }
            Ok(ResultBlock {
                block: Some(Block {
                    header: BlockHeader {
                        version: ConsensusVersion {
                            app: 12,
                            ..Default::default()
                        },
                        height,
                        time: "2026-10-05T12:00:00Z".to_string(),
                        ..Default::default()
                    },
                    data: BlockData {
                        txs: self.txs.clone(),
                    },
                }),
                ..Default::default()
            })
        }
        async fn block_results(&self, _: u64) -> DAPIResult<ResultBlockResults> {
            match &self.results {
                Some(results) => Ok(ResultBlockResults {
                    txs_results: results.clone(),
                }),
                None => Err(DapiError::Client("no results for height".to_string())),
            }
        }
        async fn blockchain(&self, _: u64, max: u64) -> DAPIResult<ResultBlockchainInfo> {
            // As Tenderdash refuses a range wholly below its base.
            if max < self.base || self.metas_fail {
                return Err(DapiError::Client(format!(
                    "min height {} can't be greater than max height {max}",
                    self.base
                )));
            }
            Ok(ResultBlockchainInfo::default())
        }
    }

    fn result(code: u32) -> BlockTxResult {
        BlockTxResult {
            code,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn should_keep_only_successful_transactions_with_their_block_positions() {
        let txs = vec![
            BASE64_STANDARD.encode(b"first"),
            BASE64_STANDARD.encode(b"second"),
        ];
        let source = BlockSource::new(Arc::new(OneBlock {
            txs,
            results: Some(vec![result(1), result(0)]),
            ..Default::default()
        }));
        let block = source.read(1).await.unwrap();
        assert_eq!(block.protocol_version, 12);
        assert_eq!(block.txs.len(), 1);
        assert_eq!(block.txs[0].index, 1);
        assert_eq!(&block.txs[0].bytes[..], b"second");
        assert_eq!(
            block.txs[0].hash,
            <[u8; 32]>::from(Sha256::digest(b"second"))
        );
    }

    /// A subscription stops waiting for a read at the deadline, but Tenderdash is still working
    /// on it: the read keeps its share of the fetch budget until Tenderdash answers, so the
    /// ones that gave up waiting make no room for more.
    #[tokio::test(start_paused = true)]
    async fn should_keep_the_fetch_budget_of_a_read_nobody_waits_for_until_tenderdash_answers() {
        let chain = Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(b"tx")],
            results: Some(vec![result(0)]),
            ..Default::default()
        });
        let source = BlockSource::new(chain.clone());
        let free = || source.fetches.available_permits();

        let held = chain.held.lock().await;
        let started = Instant::now();
        let unanswered = source.read(1).await;
        assert!(
            matches!(unanswered, Err(DapiError::Timeout(_))),
            "{unanswered:?}"
        );
        assert_eq!(started.elapsed(), BLOCK_READ_TIMEOUT);
        assert_eq!(
            free(),
            MAX_CONCURRENT_FETCHES - 2,
            "the read is still Tenderdash's work"
        );

        // A reader that goes away is no different.
        let abandoned = tokio::spawn({
            let chain = chain.clone();
            let source = BlockSource::new(chain);
            async move { (source.read(1).await.map(drop), source) }
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        abandoned.abort();

        drop(held);
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(free(), MAX_CONCURRENT_FETCHES, "Tenderdash answered");
        assert!(source.read(1).await.is_ok());
    }

    /// A reader waits its own 30 seconds, from when it asked. Readers of one height share one
    /// read, and the one that makes it gives up first; the next would otherwise start over
    /// with a deadline of its own, having already waited.
    #[tokio::test(start_paused = true)]
    async fn should_time_a_reader_from_when_it_asked_whoever_makes_the_read() {
        let chain = Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(b"tx")],
            results: Some(vec![result(0)]),
            ..Default::default()
        });
        let source = Arc::new(BlockSource::new(chain.clone()));
        let _held = chain.held.lock().await;

        let started = Instant::now();
        let first = tokio::spawn({
            let source = source.clone();
            async move { (source.read(1).await.map(drop), started.elapsed()) }
        });
        let later = Duration::from_secs(10);
        tokio::time::sleep(later).await;
        let second = source.read(1).await;

        assert!(matches!(second, Err(DapiError::Timeout(_))), "{second:?}");
        assert_eq!(started.elapsed(), later + BLOCK_READ_TIMEOUT);
        let (first, waited) = first.await.unwrap();
        assert!(matches!(first, Err(DapiError::Timeout(_))), "{first:?}");
        assert_eq!(waited, BLOCK_READ_TIMEOUT);
    }

    /// A page of metas is kept once read, and the store can prune a block afterwards.
    /// Tenderdash then refuses to read the block. That is a height this node does not keep,
    /// which another node may, not a failure of this one.
    #[tokio::test]
    async fn should_read_a_block_pruned_after_its_metas_as_a_height_the_node_lacks() {
        let pruned = BlockSource::new(Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(b"tx")],
            results: Some(vec![result(0)]),
            base: 5,
            ..Default::default()
        }));
        let lacking = pruned.read(1).await;
        assert!(
            matches!(lacking, Err(DapiError::FailedPrecondition(_))),
            "{lacking:?}"
        );
        assert!(pruned.read(5).await.is_ok(), "the base itself is kept");

        // A read that fails at a height the store keeps is a failure.
        let failing = BlockSource::new(Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(b"tx")],
            results: None,
            base: 1,
            ..Default::default()
        }));
        let failed = failing.read(1).await;
        assert!(matches!(failed, Err(DapiError::Client(_))), "{failed:?}");
    }

    /// Tenderdash refuses a range of metas that lies wholly below its store's base, where it
    /// leaves the missing heights out of a range that reaches the base. Both are heights this
    /// node does not keep.
    #[tokio::test]
    async fn should_read_a_page_wholly_below_the_store_base_as_unavailable_heights() {
        let pruned = BlockSource::new(Arc::new(OneBlock {
            base: 100,
            ..Default::default()
        }));
        let (page_start, page) = pruned.meta_page(25, 500).await.unwrap();
        assert_eq!(page_start, 21);
        assert_eq!(*page, vec![HeightMeta::Unavailable; 20]);

        // A failure to read a range the store reaches is a failure, not missing history.
        let failing = BlockSource::new(Arc::new(OneBlock {
            base: 100,
            metas_fail: true,
            ..Default::default()
        }));
        assert!(failing.meta_page(101, 500).await.is_err());
    }

    #[tokio::test]
    async fn should_fail_to_read_a_block_without_saved_results() {
        let source = BlockSource::new(Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(b"tx")],
            results: None,
            ..Default::default()
        }));
        assert!(source.read(1).await.is_err());
        assert!(source.results_saved(1).await.is_err());
    }

    /// A batch creating a document whose data holds `len` copies of `element`, a compact
    /// encoding that decodes to one `Value` each.
    fn array_batch(element: Value, len: usize) -> Vec<u8> {
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([1u8; 32]),
            transitions: vec![BatchedTransition::Document(DocumentTransition::Create(
                DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                    base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                        id: Identifier::from([2u8; 32]),
                        identity_contract_nonce: 1,
                        document_type_name: "note".to_string(),
                        data_contract_id: Identifier::from([3u8; 32]),
                    }),
                    entropy: [0; 32],
                    data: [("flags".to_string(), Value::Array(vec![element; len]))].into(),
                    prefunded_voting_balance: None,
                }),
            ))],
            ..Default::default()
        }))
        .serialize_to_bytes()
        .expect("serializable")
    }

    #[tokio::test]
    async fn should_weigh_a_decoded_compact_array_at_least_at_its_decoded_size() {
        let len = 50_000;
        let source = BlockSource::new(Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(array_batch(Value::Bool(true), len))],
            results: Some(vec![result(0)]),
            ..Default::default()
        }));
        let block = source.read(1).await.unwrap();
        assert!(
            block.txs[0].state_transition.is_ok(),
            "the transaction decodes"
        );
        let decoded = len * size_of::<Value>();
        assert!(BlockWeighter.weight(&1, &block) >= decoded as u64);
    }

    #[tokio::test]
    async fn should_weigh_a_decoded_array_at_least_at_its_retained_capacity() {
        // One past a power of two: growing one at a time leaves room for 16,384.
        let source = BlockSource::new(Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(array_batch(Value::Null, 8_193))],
            results: Some(vec![result(0)]),
            ..Default::default()
        }));
        let block = source.read(1).await.unwrap();
        assert!(
            block.txs[0].state_transition.is_ok(),
            "the transaction decodes"
        );
        let retained = 16_384 * size_of::<Value>();
        assert!(BlockWeighter.weight(&1, &block) >= retained as u64);
    }
}
